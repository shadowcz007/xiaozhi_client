import opus from '@discordjs/opus';
import Speaker from 'speaker';

export class NodeAudioPlayer {
    constructor() {
        const OpusEncoder = opus.OpusEncoder;
        this.decoder = new OpusEncoder(24000, 1);

        this.speaker = new Speaker({
            channels: 1,
            bitDepth: 16,
            sampleRate: 24000,
            signed: true
        });

        this.speaker.on('error', (err) => {
            console.error('🔊 音频播放错误:', err);
        });
    }

    processAudioData(opusData) {
        try {
            const pcmData = this.decoder.decode(opusData);

            if (this.speaker && !this.speaker.destroyed) {
                this.speaker.write(pcmData);
            }
        } catch (error) {
            console.error('🔊 音频处理错误:', error);
        }
    }

    stop() {
        if (this.speaker && !this.speaker.destroyed) {
            this.speaker.end();
        }
    }
}