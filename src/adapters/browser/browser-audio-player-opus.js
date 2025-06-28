import { IAudioPlayer } from '../../interfaces/audio-interfaces.js';

/**
 * 浏览器环境下的音频播放器实现 (使用 opus-decoder)
 * 基于 opus-decoder 库进行实时Opus解码和播放
 */
export class BrowserAudioPlayerOpus extends IAudioPlayer {
    constructor(options = {}) {
        super(options);

        // opus-decoder 相关
        this.decoder = null;
        this.audioContext = null;
        this.gainNode = null;
        this.analyserNode = null;

        // 播放控制
        this.isPlaying = false;
        this.volume = options.volume || 1.0;
        this.scheduledTime = 0;
        this.activeSources = new Set();
        this.ttsStopReceived = false;

        // 统计信息
        this.stats = {
            totalFrames: 0,
            droppedFrames: 0,
            avgLatency: 0,
            bufferUnderruns: 0
        };

        // 检查浏览器支持
        this.checkBrowserSupport();
    }

    /**
     * 检查浏览器支持
     */
    checkBrowserSupport() {
        const support = {
            webAudio: !!(window.AudioContext || window.webkitAudioContext),
            webAssembly: !!window.WebAssembly,
            dynamicImport: true // 现代浏览器都支持动态导入
        };

        const unsupported = Object.entries(support)
            .filter(([key, value]) => !value)
            .map(([key]) => key);

        if (unsupported.length > 0) {
            const message = `浏览器不支持以下功能: ${unsupported.join(', ')}`;
            console.error(message);
            console.error('🚨 请确保已安装 opus-decoder:');
            console.error('   npm install opus-decoder');
            console.error('   或使用 ES模块动态导入');
            throw new Error(message);
        }
    }

    /**
     * 初始化音频上下文
     */
    async initializeAudioContext() {
        if (this.audioContext) {
            return true;
        }

        try {
            const AudioContextClass = window.AudioContext || window.webkitAudioContext;
            this.audioContext = new AudioContextClass();

            // 恢复音频上下文 (某些浏览器需要用户交互后才能启动)
            if (this.audioContext.state === 'suspended') {
                await this.audioContext.resume();
            }

            // 创建增益节点 (音量控制)
            this.gainNode = this.audioContext.createGain();
            this.gainNode.gain.value = this.volume;
            this.gainNode.connect(this.audioContext.destination);

            // 创建分析器节点 (可选，用于音频可视化)
            this.analyserNode = this.audioContext.createAnalyser();
            this.analyserNode.fftSize = 256;
            this.gainNode.connect(this.analyserNode);

            // 初始化调度时间
            this.scheduledTime = this.audioContext.currentTime;

            console.log(`🔧 音频上下文初始化，采样率: ${this.audioContext.sampleRate}Hz`);
            return true;
        } catch (error) {
            console.error('初始化音频上下文失败:', error);
            return false;
        }
    }

    /**
     * 初始化 opus-decoder
     */
    async initializeOpusDecoder() {
        try {
            // 检查 window['opus-decoder'] 是否存在
            if (typeof window['opus-decoder'] === 'undefined' || typeof window['opus-decoder'].OpusDecoder === 'undefined') {
                const message = 'OpusDecoder 未在 window["opus-decoder"] 对象上找到。请确保通过 <script> 标签正确加载了 opus-decoder.min.js。';
                console.error(message);
                console.error('例如: <script src="/public/opus-decoder.min.js"></script>');
                throw new Error(message);
            }

            const { OpusDecoder } = window['opus-decoder'];

            // 创建解码器实例
            this.decoder = new OpusDecoder();

            // 等待 WebAssembly 模块加载完成
            await this.decoder.ready;

            console.log('✅ opus-decoder 初始化完成');
            console.log(`🔧 解码器配置: 采样率=${this.decoder.sampleRate}Hz, 声道=${this.decoder.numberOfChannels}`);

            return true;
        } catch (error) {
            console.error('初始化 opus-decoder 失败:', error);
            console.error('请确保已正确安装 opus-decoder 并且WASM文件可访问。');
            throw error;
        }
    }

    /**
     * 开始播放
     */
    async start() {
        if (this.isPlaying) {
            console.log('播放器已在运行中');
            return;
        }

        try {
            console.log('🔊 启动 Opus 播放器...');

            // 初始化音频上下文
            const audioContextReady = await this.initializeAudioContext();
            if (!audioContextReady) {
                throw new Error('音频上下文初始化失败');
            }

            // 初始化 opus-decoder
            await this.initializeOpusDecoder();

            this.isPlaying = true;
            console.log('✅ Opus 播放器已启动');

        } catch (error) {
            console.error('启动播放器失败:', error);
            this.isPlaying = false;
            if (this.onError) {
                this.onError(error);
            }
            throw error;
        }
    }

    /**
     * 停止播放
     */
    async stop() {
        if (!this.isPlaying) {
            console.log('播放器未在运行中');
            return;
        }

        try {
            console.log('🛑 停止播放器...');

            this.isPlaying = false;
            this.ttsStopReceived = false;

            // 停止并清除所有活动的音频源
            if (this.activeSources) {
                this.activeSources.forEach(source => {
                    source.onended = null; // 移除 onended 回调
                    source.stop();
                });
                this.activeSources.clear();
            }

            // 重置调度时间
            this.scheduledTime = this.audioContext ? this.audioContext.currentTime : 0;

            console.log('✅ 播放器已停止');

        } catch (error) {
            console.error('停止播放器失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 处理音频数据
     * @param {ArrayBuffer} opusData
     */
    processAudioData(opusData) {
        if (!this.isPlaying) {
            this.start().then(() => {
                this.playOpusData(opusData);
            });
        } else {
            this.playOpusData(opusData);
        }
    }

    /**
     * 播放 Opus 数据
     */
    async playOpusData(opusData) {
        if (!this.isPlaying || !this.decoder || !this.audioContext) {
            console.warn('播放器未初始化或未启动');
            return;
        }

        try {
            const startTime = performance.now();

            // 确保数据是 Uint8Array
            const opusPacket = opusData instanceof Uint8Array ? opusData : new Uint8Array(opusData);

            // 解码 Opus 数据
            const { channelData, samplesDecoded, sampleRate, errors } = await this.decoder.decodeFrame(opusPacket);

            if (errors && errors.length > 0) {
                console.warn('解码时发生错误:', errors);
            }

            if (!channelData || channelData.length === 0 || samplesDecoded === 0) {
                console.warn('解码后的数据为空');
                this.stats.droppedFrames++;
                return;
            }

            // 创建音频缓冲区
            const frameCount = samplesDecoded;
            const numberOfChannels = channelData.length;

            const audioBuffer = this.audioContext.createBuffer(
                numberOfChannels,
                frameCount,
                sampleRate
            );

            // 填充PCM数据
            for (let i = 0; i < numberOfChannels; i++) {
                audioBuffer.getChannelData(i).set(channelData[i]);
            }

            const latency = performance.now() - startTime;

            this.schedulePlayback(audioBuffer, latency);

        } catch (error) {
            console.error('播放 Opus 数据失败:', error);
            this.stats.droppedFrames++;
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 批量播放 Opus 数据
     * @param {Array<Uint8Array>} opusDataArray
     */
    async playOpusDataBatch(opusDataArray) {
        if (!this.isPlaying || !this.decoder || !this.audioContext) {
            console.warn('批量播放时播放器未初始化或未启动');
            return;
        }

        try {
            const startTime = performance.now();

            // 确保所有数据都是 Uint8Array
            const opusPackets = opusDataArray.map(p => p instanceof Uint8Array ? p : new Uint8Array(p));

            // 使用 decodeFrames 批量解码
            const { channelData, samplesDecoded, sampleRate, errors } = await this.decoder.decodeFrames(opusPackets);

            if (errors && errors.length > 0) {
                console.warn('批量解码时发生错误:', errors);
                // 即使有错误，也可能解码出部分数据，所以我们继续处理
            }

            if (!channelData || channelData.length === 0 || samplesDecoded === 0) {
                console.warn('批量解码后的数据为空');
                this.stats.droppedFrames += opusDataArray.length;
                return;
            }

            // 创建音频缓冲区
            const frameCount = samplesDecoded;
            const numberOfChannels = channelData.length;

            const audioBuffer = this.audioContext.createBuffer(
                numberOfChannels,
                frameCount,
                sampleRate
            );

            // 填充PCM数据
            for (let i = 0; i < numberOfChannels; i++) {
                audioBuffer.getChannelData(i).set(channelData[i]);
            }

            const latency = performance.now() - startTime;

            // 调度播放
            this.schedulePlayback(audioBuffer, latency);

        } catch (error) {
            console.error('批量播放 Opus 数据失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 调度音频缓冲区播放
     * @param {AudioBuffer} audioBuffer
     * @param {number} latency 解码延迟
     */
    schedulePlayback(audioBuffer, latency) {
        // 创建播放节点
        const sourceNode = this.audioContext.createBufferSource();
        sourceNode.buffer = audioBuffer;
        sourceNode.connect(this.gainNode);

        // 跟踪此音频源
        this.activeSources.add(sourceNode);

        sourceNode.onended = () => {
            // 当此音频源播放结束时，将其从跟踪集合中移除
            this.activeSources.delete(sourceNode);

            // 检查是否所有音频源都已播放完毕
            this.checkPlaybackFinished();
        };

        const now = this.audioContext.currentTime;
        const playTime = Math.max(now, this.scheduledTime);

        if (playTime > now + 0.1) {
            this.stats.bufferUnderruns++;
            console.warn('⚠️ 音频缓冲区欠载，可能出现断续');
        }

        sourceNode.start(playTime);

        const duration = audioBuffer.duration;
        this.scheduledTime = playTime + duration;

        this.stats.totalFrames++;
        this.stats.avgLatency = (this.stats.avgLatency * (this.stats.totalFrames - 1) + latency) / this.stats.totalFrames;

        if (this.onAudioData) {
            this.onAudioData({
                duration: duration,
                sampleRate: audioBuffer.sampleRate,
                channels: audioBuffer.numberOfChannels,
                samples: audioBuffer.length
            });
        }
    }

    /**
     * 接收到TTS停止信号
     */
    signalTtsStop() {
        console.log('播放器收到 TTS 停止信号，开始播放完成倒计时...');
        this.ttsStopReceived = true;

        // 使用 setTimeout 创建一个宽限期。
        // 这让任何最终的、在途的音频包有机会到达并被调度播放。
        setTimeout(() => {
            // 在宽限期后，检查播放是否真正完成。
            // `checkPlaybackFinished` 将验证在此期间没有新的音频源被添加。
            this.checkPlaybackFinished();
        }, 300); // 300毫秒的宽限期
    }

    /**
     * 检查播放是否完成
     */
    checkPlaybackFinished() {
        // 必须同时满足两个条件：
        // 1. tts:stop 消息已经收到 (意味着不会再有新的音频数据)
        // 2. 所有已调度的音频源都已播放完毕
        if (this.ttsStopReceived && this.activeSources.size === 0) {
            console.log('✅ 所有音频片段播放完成');
            this.isPlaying = false;
            this.ttsStopReceived = false; // 重置状态

            if (this.onPlaybackFinished) {
                this.onPlaybackFinished();
            }
        }
    }

    /**
     * 设置音量
     */
    setVolume(volume) {
        this.volume = Math.max(0, Math.min(1, volume));

        if (this.gainNode) {
            // 平滑音量变化
            const now = this.audioContext.currentTime;
            this.gainNode.gain.cancelScheduledValues(now);
            this.gainNode.gain.setValueAtTime(this.gainNode.gain.value, now);
            this.gainNode.gain.linearRampToValueAtTime(this.volume, now + 0.1);
        }

        console.log(`🔧 音量设置为: ${Math.round(this.volume * 100)}%`);
    }

    /**
     * 获取音量
     */
    getVolume() {
        return this.volume;
    }

    /**
     * 静音/取消静音
     */
    setMuted(muted) {
        if (this.gainNode) {
            const targetVolume = muted ? 0 : this.volume;
            const now = this.audioContext.currentTime;
            this.gainNode.gain.cancelScheduledValues(now);
            this.gainNode.gain.setValueAtTime(this.gainNode.gain.value, now);
            this.gainNode.gain.linearRampToValueAtTime(targetVolume, now + 0.05);
        }

        console.log(muted ? '🔇 已静音' : '🔊 已取消静音');
    }

    /**
     * 获取音频分析数据 (用于可视化)
     */
    getAudioAnalysis() {
        if (!this.analyserNode) {
            return null;
        }

        const bufferLength = this.analyserNode.frequencyBinCount;
        const frequencyData = new Uint8Array(bufferLength);
        const timeDomainData = new Uint8Array(bufferLength);

        this.analyserNode.getByteFrequencyData(frequencyData);
        this.analyserNode.getByteTimeDomainData(timeDomainData);

        return {
            frequency: frequencyData,
            timeDomain: timeDomainData,
            sampleRate: this.audioContext.sampleRate
        };
    }

    /**
     * 获取播放器状态
     */
    getPlayerState() {
        return {
            playing: this.isPlaying,
            volume: this.volume,
            scheduledTime: this.scheduledTime,
            currentTime: this.audioContext ? this.audioContext.currentTime : 0,
            audioContextState: this.audioContext ? this.audioContext.state : 'not-initialized'
        };
    }

    /**
     * 获取播放统计信息
     */
    getStats() {
        return {
            ...this.stats,
            dropRate: this.stats.totalFrames > 0 ? (this.stats.droppedFrames / this.stats.totalFrames) * 100 : 0
        };
    }

    /**
     * 重置统计信息
     */
    resetStats() {
        this.stats = {
            totalFrames: 0,
            droppedFrames: 0,
            avgLatency: 0,
            bufferUnderruns: 0
        };
        console.log('📊 播放统计信息已重置');
    }

    /**
     * 获取解码器信息
     */
    getDecoderInfo() {
        if (!this.decoder) {
            return { error: '解码器未初始化' };
        }

        return {
            sampleRate: this.decoder.sampleRate,
            numberOfChannels: this.decoder.numberOfChannels,
            ready: this.decoder.ready,
            state: this.getPlayerState()
        };
    }

    /**
     * 检查播放延迟
     */
    checkLatency() {
        if (!this.audioContext) {
            return null;
        }

        const now = this.audioContext.currentTime;
        const latency = this.scheduledTime - now;

        return {
            currentTime: now,
            scheduledTime: this.scheduledTime,
            latency: latency,
            bufferHealth: latency > 0.1 ? 'good' : latency > 0.05 ? 'warning' : 'critical'
        };
    }

    /**
     * 清理资源
     */
    async cleanup() {
        console.log('🧹 清理 Opus 播放器资源...');
        await this.stop();

        if (this.decoder) {
            this.decoder.free();
            this.decoder = null;
        }

        if (this.audioContext) {
            await this.audioContext.close();
            this.audioContext = null;
        }

        console.log('✅ Opus 播放器资源已清理');
    }

    /**
     * 获取支持的解码格式
     */
    static getSupportedFormats() {
        const formats = [];

        // 检查 opus-decoder 支持
        try {
            formats.push({
                name: 'Opus',
                mimeType: 'audio/opus',
                extension: '.opus',
                quality: 'high',
                compression: 'excellent',
                realtime: true
            });
        } catch (e) {
            console.warn('opus-decoder 不可用');
        }

        // 检查 Web Audio API 支持的格式
        if (window.AudioContext || window.webkitAudioContext) {
            formats.push({
                name: 'PCM',
                mimeType: 'audio/pcm',
                quality: 'highest',
                compression: 'none',
                realtime: true
            }, {
                name: 'Float32',
                mimeType: 'audio/float32',
                quality: 'highest',
                compression: 'none',
                realtime: true
            });
        }

        return formats;
    }
}