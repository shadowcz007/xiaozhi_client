// 麦克风录音和 Opus 编码模块
// 安装依赖：npm install @discordjs/opus node-mic

import NodeMic from 'node-mic';
import opusPkg from '@discordjs/opus';
const { OpusEncoder } = opusPkg;

class MicrophoneOpusRecorder {
    constructor(options = {}) {
        this.sampleRate = options.sampleRate || 16000; // 与小智项目保持一致
        this.channels = options.channels || 1;
        this.frameSize = options.frameSize || 160; // 10ms @ 16kHz
        this.bitDepth = 16; // 16-bit PCM

        // 初始化 Opus 编码器
        this.opusEncoder = new OpusEncoder(this.sampleRate, this.channels);

        this.recording = false;
        this.micInstance = null;
        this.micStream = null;

        // 音频缓冲区，用于帧对齐
        this.audioBuffer = Buffer.alloc(0);
        this.frameBytes = this.frameSize * 2; // 16-bit samples = 2 bytes per sample

        // 事件回调
        this.onOpusData = null;
        this.onError = null;
    }

    /**
     * 开始录音并实时输出 Opus 数据
     */
    startRecording() {
        if (this.recording) {
            console.log('录音已在进行中');
            return;
        }

        try {
            console.log('🎤 开始录音并编码为 Opus 格式...');
            this.recording = true;

            // 创建麦克风实例
            this.micInstance = new NodeMic({
                rate: this.sampleRate,
                channels: this.channels,
                bitwidth: this.bitDepth,
                encoding: 'signed-integer',
                endian: 'little',
                debug: false
            });

            // 获取音频流
            this.micStream = this.micInstance.getAudioStream();

            // 处理音频数据
            this.micStream.on('data', (data) => {
                if (this.recording) {
                    this.processAudioData(data);
                }
            });

            // 处理错误
            this.micStream.on('error', (error) => {
                console.error('麦克风流错误:', error);
                this.recording = false;
                if (this.onError) {
                    this.onError(error);
                }
            });

            // 处理开始事件
            this.micStream.on('started', () => {
                console.log('✅ 录音已开始，实时输出 Opus 数据');
                console.log(`🔧 采样率: ${this.sampleRate}Hz, 声道: ${this.channels}, 位深: ${this.bitDepth}bit`);
            });

            // 处理停止事件
            this.micStream.on('stopped', () => {
                console.log('✅ 录音已停止');
                this.recording = false;
            });

            // 处理暂停事件
            this.micStream.on('paused', () => {
                console.log('⏸️ 录音已暂停');
            });

            // 处理恢复事件
            this.micStream.on('unpaused', () => {
                console.log('▶️ 录音已恢复');
            });

            // 处理静音事件
            this.micStream.on('silence', () => {
                console.log('🔇 检测到静音');
            });

            // 处理退出事件
            this.micStream.on('exit', (code) => {
                console.log(`🚪 录音进程退出，代码: ${code}`);
                this.recording = false;
            });

            // 开始录音
            this.micInstance.start();

        } catch (error) {
            console.error('启动录音失败:', error);
            this.recording = false;
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 处理音频数据，进行帧对齐和 Opus 编码
     */
    processAudioData(data) {
        try {
            // 将新数据添加到缓冲区
            this.audioBuffer = Buffer.concat([this.audioBuffer, data]);

            // 处理完整的音频帧
            while (this.audioBuffer.length >= this.frameBytes) {
                // 提取一帧数据
                const frame = this.audioBuffer.slice(0, this.frameBytes);
                this.audioBuffer = this.audioBuffer.slice(this.frameBytes);

                // 编码为 Opus
                const opusData = this.opusEncoder.encode(frame);

                // 触发回调
                if (this.onOpusData) {
                    this.onOpusData(opusData);
                }
            }

        } catch (error) {
            console.error('处理音频数据失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 停止录音
     */
    stopRecording() {
        if (!this.recording) {
            console.log('录音未在进行中');
            return;
        }

        try {
            console.log('🛑 停止录音...');
            this.recording = false;

            if (this.micInstance) {
                this.micInstance.stop();
                this.micInstance = null;
            }

            if (this.micStream) {
                this.micStream.removeAllListeners();
                this.micStream = null;
            }

            // 清空缓冲区
            this.audioBuffer = Buffer.alloc(0);

        } catch (error) {
            console.error('停止录音时出错:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 检查是否正在录音
     */
    isRecording() {
        return this.recording;
    }

    /**
     * 获取音频配置信息
     */
    getAudioConfig() {
        return {
            sampleRate: this.sampleRate,
            channels: this.channels,
            frameSize: this.frameSize,
            bitDepth: this.bitDepth,
            frameBytes: this.frameBytes,
            mode: 'microphone' // 标识这是真实麦克风模式
        };
    }
}

export { MicrophoneOpusRecorder };