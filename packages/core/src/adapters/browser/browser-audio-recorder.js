import { IAudioRecorder } from '../../interfaces/audio-interfaces.js';

/**
 * 浏览器环境下的音频录音器实现 (PCM)
 * 基于 Web Audio API 和 ScriptProcessorNode
 */
export class BrowserAudioRecorder extends IAudioRecorder {
    constructor(options = {}) {
        super();
        this.options = {
            sampleRate: 16000,
            channels: 1,
            bufferSize: 4096, // ScriptProcessorNode buffer size
            ...options
        };
        this.mediaStream = null;
        this.audioContext = null;
        this._isRecording = false;
        this.paused = false;

        // Web Audio API 相关
        this.sourceNode = null;
        this.processorNode = null;
        this.gainNode = null;

        // 配置
        this.recordingGain = 1.0; // 录音增益

        // 检查浏览器支持
        this.checkBrowserSupport();
    }

    /**
     * 检查浏览器支持
     */
    checkBrowserSupport() {
        if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
            throw new Error('浏览器不支持 getUserMedia API');
        }

        if (!window.AudioContext && !window.webkitAudioContext) {
            throw new Error('浏览器不支持 Web Audio API');
        }
    }

    /**
     * 初始化音频上下文
     */
    async initializeAudioContext() {
        if (this.audioContext) {
            return;
        }

        try {
            const AudioContextClass = window.AudioContext || window.webkitAudioContext;
            // 尝试使用期望的采样率创建，如果浏览器不支持，它会回退到硬件支持的采样率
            this.audioContext = new AudioContextClass({
                sampleRate: this.options.sampleRate
            });

            // 检查实际采样率
            console.log(`🔧 音频上下文初始化，期望采样率: ${this.options.sampleRate}Hz, 实际采样率: ${this.audioContext.sampleRate}Hz`);

            if (this.audioContext.sampleRate !== this.options.sampleRate) {
                console.warn(`⚠️ 采样率不匹配，这可能会影响音频质量。理想情况下，浏览器应支持请求的采样率。`);
            }

        } catch (error) {
            console.error('初始化音频上下文失败:', error);
            throw error;
        }
    }

    /**
     * 获取用户麦克风权限
     */
    async getUserMedia() {
        try {
            const constraints = {
                audio: {
                    sampleRate: this.options.sampleRate,
                    channelCount: this.options.channels,
                    echoCancellation: true,
                    noiseSuppression: true,
                    autoGainControl: true
                }
            };

            this.mediaStream = await navigator.mediaDevices.getUserMedia(constraints);
            console.log('✅ 获取麦克风权限成功');

            // 获取实际的音轨设置
            const audioTrack = this.mediaStream.getAudioTracks()[0];
            if (audioTrack) {
                const settings = audioTrack.getSettings();
                console.log('🔧 音轨设置:', settings);
            }

            return this.mediaStream;
        } catch (error) {
            console.error('获取麦克风权限失败:', error);
            throw error;
        }
    }

    /**
     * 设置音频处理链
     */
    async setupAudioProcessing() {
        if (!this.mediaStream || !this.audioContext) {
            throw new Error('音频上下文或媒体流未初始化');
        }

        try {
            // 创建源节点
            this.sourceNode = this.audioContext.createMediaStreamSource(this.mediaStream);

            // 创建增益节点
            this.gainNode = this.audioContext.createGain();
            this.gainNode.gain.value = this.recordingGain;

            // 创建处理器节点 (ScriptProcessorNode 已废弃，但对于简单 PCM 捕获足够)
            this.processorNode = this.audioContext.createScriptProcessor(
                this.options.bufferSize,
                this.options.channels,
                this.options.channels
            );

            // 设置音频处理回调
            this.processorNode.onaudioprocess = (event) => {
                if (this._isRecording && !this.paused) {
                    this.processAudioBuffer(event.inputBuffer);
                }
            };

            // 连接音频节点: source -> gain -> processor -> destination
            // processor 连接到 destination 是为了让音频在某些浏览器环境中能够持续处理
            this.sourceNode.connect(this.gainNode);
            this.gainNode.connect(this.processorNode);
            this.processorNode.connect(this.audioContext.destination);

            console.log('✅ 音频处理链设置完成');

        } catch (error) {
            console.error('设置音频处理链失败:', error);
            throw error;
        }
    }

    /**
     * 处理音频缓冲区，提取 PCM 数据
     */
    processAudioBuffer(inputBuffer) {
        try {
            // 如果音频上下文的采样率与我们期望的不同，这里需要重采样
            // 为简单起见，我们暂时忽略重采样，但这在生产环境中很重要
            // if (inputBuffer.sampleRate !== this.options.sampleRate) { ... }

            // 获取原始 Float32 音频数据
            const float32Data = inputBuffer.getChannelData(0); // 获取第一个声道

            // 转换为 16-bit PCM
            const pcm16Data = this.float32ToPCM16(float32Data);

            // 触发 PCM 数据回调，发送 ArrayBuffer
            if (this.onPcmData) {
                this.onPcmData(pcm16Data.buffer);
            }

        } catch (error) {
            console.error('处理音频缓冲区失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 将 Float32Array 转换为 16-bit PCM (Int16Array)
     */
    float32ToPCM16(float32Array) {
        const length = float32Array.length;
        const int16Array = new Int16Array(length);

        for (let i = 0; i < length; i++) {
            // 将 [-1, 1] 范围的浮点数转换为 [-32768, 32767] 范围的整数
            const sample = Math.max(-1, Math.min(1, float32Array[i]));
            int16Array[i] = sample < 0 ? sample * 0x8000 : sample * 0x7FFF;
        }

        return int16Array;
    }

    /**
     * 开始录音
     */
    async startRecording() {
        if (this._isRecording) {
            console.warn('录音已在进行中');
            return;
        }
        try {
            console.log('🎤 开始浏览器 PCM 录音...');

            // 初始化音频上下文
            await this.initializeAudioContext();

            // 获取麦克风权限
            await this.getUserMedia();

            // 设置音频处理
            await this.setupAudioProcessing();

            // 恢复音频上下文 (某些浏览器需要用户交互后才能启动)
            if (this.audioContext.state === 'suspended') {
                await this.audioContext.resume();
            }

            this._isRecording = true;
            this.paused = false;

            console.log('✅ 浏览器 PCM 录音已开始');

        } catch (error) {
            console.error('启动录音失败:', error);
            this._isRecording = false;
            if (this.onError) {
                this.onError(error);
            }
            throw error; // 重新抛出异常，让调用者知道失败了
        }
    }

    /**
     * 停止录音
     */
    async stopRecording() {
        if (!this._isRecording) {
            return;
        }

        try {
            console.log('🛑 停止录音...');
            this._isRecording = false;
            this.paused = false;

            // 断开音频节点以停止处理
            if (this.processorNode) this.processorNode.disconnect();
            if (this.gainNode) this.gainNode.disconnect();
            if (this.sourceNode) this.sourceNode.disconnect();

            // 停止媒体流轨道
            if (this.mediaStream) {
                this.mediaStream.getTracks().forEach(track => track.stop());
                this.mediaStream = null;
            }

            console.log('✅ 录音已停止');

        } catch (error) {
            console.error('停止录音失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 暂停录音
     */
    pauseRecording() {
        if (!this._isRecording) return;
        this.paused = true;
        console.log('⏸️ 录音已暂停');
    }

    /**
     * 恢复录音
     */
    resumeRecording() {
        if (!this._isRecording || !this.paused) return;
        this.paused = false;
        console.log('▶️ 录音已恢复');
    }

    /**
     * 是否正在录音
     */
    isRecording() {
        return this._isRecording && !this.paused;
    }

    /**
     * 设置录音增益
     */
    setGain(gain) {
        this.recordingGain = Math.max(0, Math.min(2, gain)); // 限制在 0-2 之间
        if (this.gainNode) {
            this.gainNode.gain.value = this.recordingGain;
        }
        console.log(`🔧 录音增益设置为: ${this.recordingGain}`);
    }

    /**
     * 获取音频配置信息
     */
    getAudioConfig() {
        return {
            sampleRate: this.audioContext ? this.audioContext.sampleRate : this.options.sampleRate,
            channels: this.options.channels,
            bufferSize: this.options.bufferSize,
            recording: this._isRecording,
            paused: this.paused,
            gain: this.recordingGain
        };
    }

    /**
     * 获取设备信息
     */
    async getDeviceInfo() {
        try {
            if (!navigator.mediaDevices || !navigator.mediaDevices.enumerateDevices) {
                return { error: '浏览器不支持设备枚举' };
            }

            const devices = await navigator.mediaDevices.enumerateDevices();
            const audioInputDevices = devices.filter(device => device.kind === 'audioinput');

            return {
                availableDevices: audioInputDevices,
                currentConfig: this.getAudioConfig(),
                browserSupport: {
                    getUserMedia: !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia),
                    webAudio: !!(window.AudioContext || window.webkitAudioContext),
                }
            };
        } catch (error) {
            console.error('获取设备信息失败:', error);
            return { error: error.message };
        }
    }

    /**
     * 清理资源
     */
    cleanup() {
        try {
            if (this._isRecording) {
                this.stopRecording();
            }

            if (this.audioContext && this.audioContext.state !== 'closed') {
                this.audioContext.close();
                this.audioContext = null;
            }

            this.sourceNode = null;
            this.gainNode = null;
            this.processorNode = null;

            console.log('✅ 浏览器录音器资源已清理');
        } catch (error) {
            console.error('清理录音器资源失败:', error);
        }
    }
}