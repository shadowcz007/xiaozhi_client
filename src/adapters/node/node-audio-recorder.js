import { IAudioRecorder } from '../../interfaces/audio-interfaces.js';
import pkg from 'audify';
const { RtAudio, RtAudioFormat, OpusEncoder, OpusApplication } = pkg;

/**
 * Node.js 环境下的音频录音器实现
 * 基于 audify 库，封装原有的 MicrophoneOpusRecorder 功能
 */
export class NodeAudioRecorder extends IAudioRecorder {
    constructor(options = {}) {
        super(options);

        this.bitDepth = 16; // 16-bit PCM

        // 初始化 RtAudio 实例
        this.rtAudio = new RtAudio();

        // 初始化 Opus 编码器
        this.opusEncoder = new OpusEncoder(this.sampleRate, this.channels, OpusApplication.OPUS_APPLICATION_AUDIO);

        this.recording = false;
        this.streamOpened = false;
        this.actualSampleRate = this.sampleRate;
        this.resampler = null;
    }

    /**
     * 获取可用的音频设备列表
     */
    getDevices() {
        try {
            return this.rtAudio.getDevices();
        } catch (error) {
            console.error('获取设备列表失败:', error);
            return [];
        }
    }

    /**
     * 获取默认输入设备
     */
    getDefaultInputDevice() {
        try {
            return this.rtAudio.getDefaultInputDevice();
        } catch (error) {
            console.error('获取默认输入设备失败:', error);
            return null;
        }
    }

    /**
     * 开始录音并实时输出 Opus 数据
     */
    async startRecording() {
        if (this.recording) {
            console.log('录音已在进行中');
            return;
        }

        try {
            console.log('🎤 开始录音并编码为 Opus 格式...');

            const defaultInputDevice = this.getDefaultInputDevice();
            if (defaultInputDevice === null) {
                throw new Error('未找到可用的输入设备');
            }

            console.log(`🔧 使用设备: ${defaultInputDevice} (采样率: ${this.sampleRate}Hz, 声道: ${this.channels}, 位深: ${this.bitDepth}bit)`);

            // 尝试不同的采样率，如果16000不支持就尝试常见的采样率
            const sampleRates = [this.sampleRate, 48000, 44100, 22050, 16000];
            let actualSampleRate = this.sampleRate;
            let streamOpened = false;

            for (const testRate of sampleRates) {
                try {
                    // 计算对应采样率的帧大小
                    const frameSize = Math.floor(testRate * 0.02); // 20ms

                    // 打开音频流
                    this.rtAudio.openStream(
                        null, // 不需要输出流
                        {
                            deviceId: defaultInputDevice,
                            nChannels: this.channels,
                            firstChannel: 0
                        },
                        RtAudioFormat.RTAUDIO_SINT16, // 16-bit signed integer PCM
                        testRate,
                        frameSize, // 帧大小
                        "XiaozhiRecorder", // 流名称
                        (pcmData) => {
                            if (this.recording) {
                                this.processAudioData(pcmData, testRate, frameSize);
                            }
                        }
                    );

                    actualSampleRate = testRate;
                    streamOpened = true;
                    console.log(`✅ 成功使用采样率: ${actualSampleRate}Hz`);
                    break;
                } catch (err) {
                    console.log(`⚠️ 采样率 ${testRate}Hz 不支持，尝试下一个...`);
                    if (this.rtAudio) {
                        try {
                            this.rtAudio.closeStream();
                        } catch (e) {
                            // 忽略关闭错误
                        }
                    }
                }
            }

            if (!streamOpened) {
                throw new Error('无法找到支持的采样率');
            }

            this.streamOpened = true;
            this.actualSampleRate = actualSampleRate;

            // 开始音频流
            this.rtAudio.start();
            this.recording = true;

            console.log('✅ 录音已开始，实时输出 Opus 数据');

        } catch (error) {
            console.error('启动录音失败:', error);
            this.recording = false;
            this.streamOpened = false;
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 处理音频数据，进行 Opus 编码
     */
    processAudioData(pcmData, sampleRate = null, frameSize = null) {
        try {
            // 使用实际的采样率和帧大小
            const actualSampleRate = sampleRate || this.actualSampleRate || this.sampleRate;
            const actualFrameSize = frameSize || this.frameSize;

            // 如果采样率不是16000，需要进行重采样或创建新的编码器
            if (actualSampleRate !== this.sampleRate) {
                if (!this.resampler || this.resampler.sampleRate !== actualSampleRate) {
                    // 创建新的编码器用于实际采样率
                    this.resampler = {
                        encoder: new OpusEncoder(actualSampleRate, this.channels, OpusApplication.OPUS_APPLICATION_AUDIO),
                        sampleRate: actualSampleRate
                    };
                    console.log(`🔄 创建新编码器，采样率: ${actualSampleRate}Hz`);
                }

                // 使用新编码器编码
                const opusData = this.resampler.encoder.encode(pcmData, actualFrameSize);

                // 触发回调
                if (this.onOpusData && opusData) {
                    this.onOpusData(opusData);
                }
            } else {
                // 直接使用原编码器
                const opusData = this.opusEncoder.encode(pcmData, actualFrameSize);

                // 触发回调
                if (this.onOpusData && opusData) {
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
    async stopRecording() {
        if (!this.recording) {
            console.log('录音未在进行中');
            return;
        }

        try {
            console.log('🛑 停止录音...');
            this.recording = false;

            if (this.streamOpened) {
                this.rtAudio.stop();
                this.rtAudio.closeStream();
                this.streamOpened = false;
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
        if (!this.recording) {
            console.log('录音未在进行中，无法暂停');
            return;
        }

        try {
            console.log('⏸️ 暂停录音...');
            this.recording = false; // 停止处理音频数据，但保持流开启
            console.log('✅ 录音已暂停');
        } catch (error) {
            console.error('暂停录音失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 恢复录音
     */
    resumeRecording() {
        if (this.recording) {
            console.log('录音已在进行中');
            return;
        }

        if (!this.streamOpened) {
            console.log('音频流未开启，请先开始录音');
            return;
        }

        try {
            console.log('▶️ 恢复录音...');
            this.recording = true;
            console.log('✅ 录音已恢复');
        } catch (error) {
            console.error('恢复录音失败:', error);
            if (this.onError) {
                this.onError(error);
            }
        }
    }

    /**
     * 是否正在录音
     */
    isRecording() {
        return this.recording;
    }

    /**
     * 获取音频配置信息
     */
    getAudioConfig() {
        return {
            sampleRate: this.actualSampleRate || this.sampleRate,
            channels: this.channels,
            frameSize: this.frameSize,
            bitDepth: this.bitDepth,
            recording: this.recording,
            streamOpened: this.streamOpened
        };
    }

    /**
     * 获取设备信息
     */
    getDeviceInfo() {
        try {
            const devices = this.getDevices();
            const defaultDevice = this.getDefaultInputDevice();

            return {
                availableDevices: devices,
                defaultInputDevice: defaultDevice,
                currentConfig: this.getAudioConfig()
            };
        } catch (error) {
            console.error('获取设备信息失败:', error);
            return null;
        }
    }

    /**
     * 清理资源
     */
    cleanup() {
        try {
            if (this.recording) {
                this.stopRecording();
            }

            if (this.opusEncoder) {
                this.opusEncoder = null;
            }

            if (this.resampler && this.resampler.encoder) {
                this.resampler.encoder = null;
                this.resampler = null;
            }

            if (this.rtAudio) {
                this.rtAudio = null;
            }

            console.log('✅ Node.js 录音器资源已清理');
        } catch (error) {
            console.error('清理录音器资源失败:', error);
        }
    }
}