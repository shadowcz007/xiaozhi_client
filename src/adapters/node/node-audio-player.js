import { IAudioPlayer } from '../../interfaces/audio-interfaces.js';
import pkg from 'audify';
const { RtAudio, RtAudioFormat, OpusDecoder } = pkg;

/**
 * Node.js 环境下的音频播放器实现
 * 基于 audify 库，封装原有的 NodeAudioPlayer 功能
 */
export class NodeAudioPlayer extends IAudioPlayer {
    constructor() {
        super();

        // 初始化 Opus 解码器
        this.decoder = new OpusDecoder(48000, 1);

        // 初始化 RtAudio 实例
        this.rtAudio = new RtAudio();

        this.streamOpened = false;
        this.audioBuffer = [];
        this.bufferSize = 5; // 初始缓冲帧数：5
        this.minBufferThreshold = 0; // 最小缓冲阈值：0（改为动态管理）  
        this.maxBufferSize = 50; // 添加最大缓冲区限制，防止内存积累

        // 音频配置
        this.sampleRate = 48000;
        this.channels = 1;
        this.frameSize = Math.floor(this.sampleRate * 0.02); // 20ms frames

        // 播放状态监控
        this.lastDataTime = Date.now();
        this.playbackMonitor = null;
        this.isPlaying = false;
    }

    /**
     * 获取默认输出设备
     */
    getDefaultOutputDevice() {
        try {
            return this.rtAudio.getDefaultOutputDevice();
        } catch (error) {
            console.error('获取默认输出设备失败:', error);
            return null;
        }
    }

    /**
     * 初始化音频输出流
     */
    initializeAudioStream() {
        if (this.streamOpened) {
            return true;
        }

        try {
            // 获取默认输出设备
            const outputDevices = this.rtAudio.getDevices();
            const defaultOutputDevice = outputDevices.find(device => device.isDefaultOutput);

            if (!defaultOutputDevice) {
                throw new Error('找不到默认音频输出设备');
            }

            // 打开音频流
            this.rtAudio.openStream({
                    deviceId: defaultOutputDevice.id,
                    nChannels: this.channels,
                    firstChannel: 0
                },
                null,
                RtAudioFormat.RTAUDIO_SINT16,
                this.sampleRate,
                this.frameSize,
                'MyStream',
                null,
                null
            );

            this.streamOpened = true;
            console.log(`🔊 音频输出流已初始化，采样率: ${this.sampleRate}Hz`);

            return true;
        } catch (error) {
            console.error('🔊 初始化音频输出流失败:', error);
            this.streamOpened = false;
            return false;
        }
    }

    /**
     * 处理音频数据
     */
    async processAudioData(opusData) {
        try {
            // 验证输入数据
            if (!opusData || opusData.length === 0) {
                console.warn('🔊 收到空的Opus数据');
                return;
            }

            // 验证解码器状态
            if (!this.decoder) {
                console.error('🔊 解码器未初始化');
                return;
            }

            // 解码 Opus 数据为 PCM
            const frameSize = this.frameSize;
            const pcmData = this.decoder.decode(opusData, frameSize);

            if (!pcmData || pcmData.length === 0) {
                console.warn('🔊 解码返回空PCM数据');
                return;
            }

            // 检查缓冲区是否过满，防止内存积累
            if (this.audioBuffer.length >= this.maxBufferSize) {
                console.warn('🔊 音频缓冲区过满，丢弃旧数据');
                this.audioBuffer.shift(); // 移除最旧的数据
            }

            // 将音频数据添加到缓冲区
            this.audioBuffer.push(pcmData);
            this.lastDataTime = Date.now(); // 更新最后接收数据时间

            // 如果还没开始播放且缓冲区足够大，开始播放
            if (!this.isPlaying && this.audioBuffer.length >= this.bufferSize) {
                this.startPlayback();
            }
            // 如果正在播放，继续写入数据
            else if (this.isPlaying) {
                this.flushBuffer();
            }
            // 如果播放暂停但有足够数据，重新开始播放
            else if (!this.isPlaying && this.streamOpened && this.audioBuffer.length >= this.minBufferThreshold) {
                this.resumePlayback();
            }

        } catch (error) {
            console.error('🔊 音频处理错误:', error.message);
            console.error('🔍 错误详情:', {
                opusDataLength: opusData ? opusData.length : 'null',
                frameSize: this.frameSize,
                decoderExists: !!this.decoder,
                sampleRate: this.sampleRate,
                channels: this.channels
            });
        }
    }

    /**
     * 开始播放
     */
    startPlayback() {
        if (this.isPlaying) {
            return;
        }

        if (!this.initializeAudioStream()) {
            console.error('🔊 无法初始化音频流，播放失败');
            return;
        }

        try {
            this.rtAudio.start();
            this.isPlaying = true;
            console.log('🔊 开始音频播放');
            this.startPlaybackMonitor(); // 启动播放监控
            this.flushBuffer();
        } catch (error) {
            console.error('🔊 启动音频播放失败:', error);
            this.isPlaying = false;
        }
    }

    /**
     * 刷新缓冲区
     */
    flushBuffer() {
        if (!this.streamOpened || !this.rtAudio || this.audioBuffer.length === 0) {
            return;
        }

        // 批量写入缓冲的音频数据
        let writeCount = 0;
        const maxWritePerFlush = 3; // 每次最多写入3帧，减少消耗速度

        while (this.audioBuffer.length > 0 && writeCount < maxWritePerFlush) {
            const pcmData = this.audioBuffer.shift();

            try {
                // 检查 PCM 数据大小是否正确
                const expectedSize = this.frameSize * this.channels * 2; // 16-bit = 2 bytes per sample

                if (!pcmData || pcmData.length === 0) {
                    console.warn('🔊 跳过空的PCM数据');
                    continue;
                }

                // 如果数据大小不匹配，进行调整
                let dataToWrite = pcmData;
                if (pcmData.length !== expectedSize) {
                    if (pcmData.length > expectedSize) {
                        // 如果数据过长，截断
                        dataToWrite = pcmData.slice(0, expectedSize);
                        console.warn(`🔊 PCM数据过长，已截断至${expectedSize}字节`);
                    } else {
                        // 如果数据过短，填充静音
                        dataToWrite = new Float32Array(expectedSize);
                        dataToWrite.set(pcmData);
                        console.warn(`🔊 PCM数据过短，已填充至${expectedSize}字节`);
                    }
                }

                // 使用 RtAudio 写入音频数据
                this.rtAudio.write(dataToWrite);
                writeCount++;
            } catch (error) {
                console.error('🔊 写入音频数据失败:', error);
                // 如果写入失败，将数据放回缓冲区
                this.audioBuffer.unshift(pcmData);
                break;
            }
        }
    }

    /**
     * 启动播放监控
     */
    startPlaybackMonitor() {
        if (this.playbackMonitor) {
            return;
        }

        this.playbackMonitor = setInterval(() => {
            const now = Date.now();
            const timeSinceLastData = now - this.lastDataTime;

            // 如果超过1秒没有新数据且缓冲区为空，停止播放
            if (timeSinceLastData > 1000 && this.audioBuffer.length === 0) {
                console.log('🔊 播放完成，自动停止');
                this.handlePlaybackFinished();
            }
        }, 100); // 每100ms检查一次
    }

    /**
     * 停止播放监控
     */
    stopPlaybackMonitor() {
        if (this.playbackMonitor) {
            clearInterval(this.playbackMonitor);
            this.playbackMonitor = null;
        }
    }

    /**
     * 处理播放完成
     */
    handlePlaybackFinished() {
        this.isPlaying = false;
        this.stopPlaybackMonitor();

        if (this.onPlaybackFinished) {
            this.onPlaybackFinished();
        }
    }

    /**
     * 暂停播放
     */
    pausePlayback() {
        if (!this.isPlaying) {
            return;
        }

        try {
            this.isPlaying = false;
            this.stopPlaybackMonitor();
            console.log('⏸️ 音频播放已暂停');
        } catch (error) {
            console.error('🔊 暂停播放失败:', error);
        }
    }

    /**
     * 恢复播放
     */
    resumePlayback() {
        if (this.isPlaying || !this.streamOpened) {
            return;
        }

        try {
            this.isPlaying = true;
            this.startPlaybackMonitor();
            this.flushBuffer();
            console.log('▶️ 音频播放已恢复');
        } catch (error) {
            console.error('🔊 恢复播放失败:', error);
            this.isPlaying = false;
        }
    }

    /**
     * 停止播放
     */
    async stop() {
        try {
            console.log('🛑 停止音频播放...');
            this.isPlaying = false;
            this.stopPlaybackMonitor();

            // 清空缓冲区
            this.audioBuffer = [];

            if (this.streamOpened) {
                this.rtAudio.stop();
                this.rtAudio.closeStream();
                this.streamOpened = false;
            }

            console.log('✅ 音频播放已停止');
        } catch (error) {
            console.error('🔊 停止播放失败:', error);
        }
    }

    /**
     * 获取缓冲状态
     */
    getBufferStatus() {
        return {
            bufferLength: this.audioBuffer.length,
            bufferSize: this.bufferSize,
            minThreshold: this.minBufferThreshold,
            maxSize: this.maxBufferSize,
            isPlaying: this.isPlaying,
            streamOpened: this.streamOpened,
            lastDataTime: this.lastDataTime
        };
    }

    /**
     * 强制重启播放
     */
    forceRestart() {
        console.log('🔄 强制重启音频播放...');
        this.stop().then(() => {
            // 等待一点时间再重新开始
            setTimeout(() => {
                if (this.audioBuffer.length > 0) {
                    this.startPlayback();
                }
            }, 100);
        });
    }

    /**
     * 获取音频配置
     */
    getAudioConfig() {
        return {
            sampleRate: this.sampleRate,
            channels: this.channels,
            frameSize: this.frameSize,
            bufferStatus: this.getBufferStatus()
        };
    }

    /**
     * 清理资源
     */
    cleanup() {
        try {
            this.stop();
            this.stopPlaybackMonitor();

            if (this.decoder) {
                this.decoder = null;
            }

            if (this.rtAudio) {
                this.rtAudio = null;
            }

            this.audioBuffer = [];
            console.log('✅ Node.js 播放器资源已清理');
        } catch (error) {
            console.error('清理播放器资源失败:', error);
        }
    }
}