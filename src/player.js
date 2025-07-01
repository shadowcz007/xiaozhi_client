import pkg from 'audify';
const { RtAudio, RtAudioFormat, OpusDecoder } = pkg;

export class NodeAudioPlayer {
    constructor() {
        // 初始化 Opus 解码器 (注意：这里应该用 OpusDecoder，不是 OpusEncoder)
        this.decoder = new OpusDecoder(24000, 1);

        // 初始化 RtAudio 实例
        this.rtAudio = new RtAudio();

        this.isPlaying = false;
        this.streamOpened = false;
        this.audioBuffer = [];
        this.bufferSize = 5; // 初始缓冲帧数：5
        this.minBufferThreshold = 0; // 最小缓冲阈值：0（改为动态管理）  
        this.maxBufferSize = 50; // 添加最大缓冲区限制，防止内存积累

        // 音频配置
        this.sampleRate = 24000;
        this.channels = 1;
        this.frameSize = Math.floor(this.sampleRate * 0.02); // 20ms frames

        // 播放状态监控
        this.lastDataTime = Date.now();
        this.playbackMonitor = null;
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
            const defaultOutputDevice = this.getDefaultOutputDevice();
            if (defaultOutputDevice === null) {
                throw new Error('未找到可用的输出设备');
            }

            // 优先使用原始采样率，减少重新创建解码器的需要
            const sampleRates = [this.sampleRate, 48000, 44100, 22050];
            let actualSampleRate = this.sampleRate;
            let streamOpened = false;

            for (const testRate of sampleRates) {
                try {
                    const frameSize = Math.floor(testRate * 0.02); // 20ms

                    this.rtAudio.openStream({
                            deviceId: defaultOutputDevice,
                            nChannels: this.channels,
                            firstChannel: 0
                        },
                        null, // 不需要输入流
                        RtAudioFormat.RTAUDIO_SINT16,
                        testRate,
                        frameSize,
                        "XiaozhiPlayer",
                        null // 播放模式不需要回调
                    );

                    actualSampleRate = testRate;
                    streamOpened = true;
                    console.log(`🔊 音频输出流已初始化，采样率: ${actualSampleRate}Hz`);
                    break;
                } catch (err) {
                    console.log(`⚠️ 输出采样率 ${testRate}Hz 不支持，尝试下一个...`);
                    try {
                        this.rtAudio.closeStream();
                    } catch (e) {
                        // 忽略关闭错误
                    }
                }
            }

            if (!streamOpened) {
                throw new Error('无法初始化音频输出流');
            }

            this.streamOpened = true;
            this.actualSampleRate = actualSampleRate;

            // 如果采样率不同，需要重新创建解码器
            if (actualSampleRate !== this.sampleRate) {
                // 清空现有缓冲区，避免采样率不匹配的数据混合
                this.audioBuffer = [];
                this.decoder = new OpusDecoder(actualSampleRate, this.channels);
                this.frameSize = Math.floor(actualSampleRate * 0.02); // 更新frameSize
                console.log(`🔄 重新创建解码器，采样率: ${actualSampleRate}Hz, frameSize: ${this.frameSize}`);
                console.log(`⚠️ 已清空音频缓冲区以避免采样率不匹配`);
            }

            return true;
        } catch (error) {
            console.error('🔊 初始化音频输出流失败:', error);
            this.streamOpened = false;
            return false;
        }
    }

    processAudioData(opusData) {
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
            // console.log(`🎵 解码音频: opusSize=${opusData.length}, frameSize=${frameSize}, sampleRate=${this.actualSampleRate || this.sampleRate}`);

            let pcmData;
            try {
                pcmData = this.decoder.decode(opusData, frameSize);
            } catch (decodeError) {
                console.error('🔊 Opus解码失败:', decodeError);
                // 解码失败时，生成静音数据避免音频中断
                const expectedSize = frameSize * this.channels * 2; // 16-bit = 2 bytes per sample
                pcmData = Buffer.alloc(expectedSize, 0); // 填充静音
                console.warn('🔊 使用静音数据替代解码失败的帧');
            }

            if (!pcmData || pcmData.length === 0) {
                console.warn('🔊 解码返回空PCM数据，生成静音帧');
                // 生成一帧静音数据保持连续性
                const expectedSize = frameSize * this.channels * 2;
                pcmData = Buffer.alloc(expectedSize, 0);
            }

            // console.log(`✅ 解码成功: PCM长度=${pcmData.length}`);

            // 检查缓冲区是否过满，防止内存积累
            if (this.audioBuffer.length >= this.maxBufferSize) {
                console.warn('🔊 音频缓冲区过满，丢弃旧数据');
                // 丢弃一些旧数据，但不要一次丢弃太多
                const dropCount = Math.min(5, this.audioBuffer.length - this.maxBufferSize + 1);
                for (let i = 0; i < dropCount; i++) {
                    this.audioBuffer.shift();
                }
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
                sampleRate: this.actualSampleRate || this.sampleRate,
                channels: this.channels,
                bufferLength: this.audioBuffer.length,
                isPlaying: this.isPlaying,
                streamOpened: this.streamOpened
            });
        }
    }

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

                if (pcmData.length !== expectedSize) {
                    // 如果数据大小不匹配，尝试调整数据而不是直接跳过
                    console.warn(`🔊 PCM数据大小不匹配: 期望${expectedSize}字节，实际${pcmData.length}字节，尝试调整`);

                    let adjustedData;
                    if (pcmData.length > expectedSize) {
                        // 数据过长，截断
                        adjustedData = pcmData.slice(0, expectedSize);
                        console.warn(`🔊 截断音频数据至${expectedSize}字节`);
                    } else {
                        // 数据过短，用静音填充
                        adjustedData = Buffer.alloc(expectedSize);
                        pcmData.copy(adjustedData, 0);
                        // 剩余部分已经是0（静音）
                        console.warn(`🔊 音频数据用静音填充至${expectedSize}字节`);
                    }

                    // 使用调整后的数据
                    this.rtAudio.write(adjustedData);
                    writeCount++;
                    continue;
                }

                // 使用 RtAudio 写入音频数据
                this.rtAudio.write(pcmData);
                writeCount++;
            } catch (error) {
                // 如果写入失败，重新放回数据
                this.audioBuffer.unshift(pcmData);
                console.warn('🔊 音频写入失败:', error.message);
                break;
            }
        }

        // 只有在缓冲区完全空了且没有新数据到达时才暂停播放
        // 注意：这里移除了立即暂停的逻辑，改为由外部控制
    }

    /**
     * 启动播放监控
     */
    startPlaybackMonitor() {
        if (this.playbackMonitor) {
            clearInterval(this.playbackMonitor);
        }

        this.playbackMonitor = setInterval(() => {
            // 如果缓冲区为空且超过1秒没有新数据，停止播放
            if (this.audioBuffer.length === 0 &&
                Date.now() - this.lastDataTime > 1000 &&
                this.isPlaying) {
                console.log('🔊 缓冲区为空且超时，自动停止播放');
                this.pausePlayback();
            }
            // 如果缓冲区有数据但播放停止了，重新开始播放
            else if (this.audioBuffer.length > 0 && !this.isPlaying && this.streamOpened) {
                console.log('🔊 检测到缓冲区有数据，重新开始播放');
                this.resumePlayback();
            }
        }, 200); // 每200ms检查一次
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
     * 暂停播放（不关闭流）
     */
    pausePlayback() {
        if (!this.isPlaying) {
            return;
        }

        try {
            this.rtAudio.stop();
            this.isPlaying = false;
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

        if (this.audioBuffer.length >= this.minBufferThreshold) {
            try {
                this.rtAudio.start();
                this.isPlaying = true;
                console.log('▶️ 音频播放已恢复');
                this.flushBuffer();
            } catch (error) {
                console.error('🔊 恢复播放失败:', error);
            }
        }
    }

    stop() {
        console.log('🔊 停止音频播放');
        this.isPlaying = false;
        this.audioBuffer = [];
        this.stopPlaybackMonitor(); // 停止播放监控

        if (this.streamOpened) {
            try {
                this.rtAudio.stop();
                this.rtAudio.closeStream();
                this.streamOpened = false;
            } catch (error) {
                console.error('🔊 关闭音频流失败:', error);
            }
        }
    }

    // 获取缓冲状态信息
    getBufferStatus() {
        return {
            isPlaying: this.isPlaying,
            streamOpened: this.streamOpened,
            bufferLength: this.audioBuffer.length,
            bufferSize: this.bufferSize,
            threshold: this.minBufferThreshold,
            sampleRate: this.actualSampleRate || this.sampleRate,
            channels: this.channels
        };
    }

    // 强制重启播放的方法
    forceRestart() {
        if (this.audioBuffer.length >= this.minBufferThreshold && !this.isPlaying) {
            console.log('🔊 强制重启音频播放');
            this.startPlayback();
        }
    }

    /**
     * 获取音频配置信息
     */
    getAudioConfig() {
        return {
            sampleRate: this.actualSampleRate || this.sampleRate,
            channels: this.channels,
            frameSize: this.frameSize,
            mode: 'audify',
            library: 'RtAudio'
        };
    }

    /**
     * 清理资源
     */
    cleanup() {
        this.stopPlaybackMonitor(); // 确保停止监控
        this.stop();
        console.log('✅ 音频播放器资源已清理');
    }
}