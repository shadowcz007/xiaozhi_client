import opus from '@discordjs/opus';
import Speaker from 'speaker';

export class NodeAudioPlayer {
    constructor() {
        const OpusEncoder = opus.OpusEncoder;
        this.decoder = new OpusEncoder(24000, 1);

        this.speaker = null;
        this.isPlaying = false;
        this.audioBuffer = [];
        this.bufferSize = 5; // 缓冲5帧音频再开始播放
        this.minBufferThreshold = 2; // 最小缓冲阈值
    }

    initializeSpeaker() {
        if (this.speaker && !this.speaker.destroyed) {
            return;
        }

        this.speaker = new Speaker({
            channels: 1,
            bitDepth: 16,
            sampleRate: 24000,
            signed: true,
            highWaterMark: 1024 * 16 // 增加内部缓冲区大小
        });

        this.speaker.on('error', (err) => {
            console.error('🔊 音频播放错误:', err);
            this.isPlaying = false;
        });

        this.speaker.on('drain', () => {
            // 当 Speaker 需要更多数据时的回调
            this.flushBuffer();
        });
    }

    processAudioData(opusData) {
        try {
            const pcmData = this.decoder.decode(opusData);

            if (!pcmData || pcmData.length === 0) {
                return;
            }

            // 将音频数据添加到缓冲区
            this.audioBuffer.push(pcmData);

            // 如果还没开始播放且缓冲区足够大，开始播放
            if (!this.isPlaying && this.audioBuffer.length >= this.bufferSize) {
                this.startPlayback();
            }
            // 如果正在播放，继续写入数据
            else if (this.isPlaying) {
                this.flushBuffer();
            }

        } catch (error) {
            console.error('🔊 音频处理错误:', error);
        }
    }

    startPlayback() {
        if (this.isPlaying) {
            return;
        }

        this.initializeSpeaker();
        this.isPlaying = true;
        // console.log('🔊 开始音频播放');
        this.flushBuffer();
    }

    flushBuffer() {
        if (!this.speaker || this.speaker.destroyed || this.audioBuffer.length === 0) {
            return;
        }

        // 批量写入缓冲的音频数据
        while (this.audioBuffer.length > 0) {
            const pcmData = this.audioBuffer.shift();

            if (!this.speaker.write(pcmData)) {
                // 如果 Speaker 缓冲区满了，重新放回数据等待下次
                this.audioBuffer.unshift(pcmData);
                break;
            }
        }

        // 如果缓冲区数据不足，暂停播放等待更多数据
        if (this.audioBuffer.length < this.minBufferThreshold && this.isPlaying) {
            // console.log('🔊 音频缓冲不足，等待更多数据...');
            this.isPlaying = false;
        }
    }

    stop() {
        console.log('🔊 停止音频播放');
        this.isPlaying = false;
        this.audioBuffer = [];

        if (this.speaker && !this.speaker.destroyed) {
            this.speaker.end();
            this.speaker = null;
        }
    }

    // 获取缓冲状态信息
    getBufferStatus() {
        return {
            isPlaying: this.isPlaying,
            bufferLength: this.audioBuffer.length,
            bufferSize: this.bufferSize,
            threshold: this.minBufferThreshold
        };
    }
}