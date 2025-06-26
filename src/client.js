import { WebSocketProtocol } from './websocket.js';
import { MicrophoneOpusRecorder } from './voice.js';
import { NodeAudioPlayer } from './player.js';

/**
 * 设备状态枚举
 */
const DeviceState = {
    IDLE: 'idle',
    CONNECTING: 'connecting',
    LISTENING: 'listening',
    SPEAKING: 'speaking'
};

/**
 * 监听模式枚举
 */
const ListeningMode = {
    ALWAYS_ON: 'realtime',
    AUTO_STOP: 'auto',
    MANUAL: 'manual'
};

/**
 * 小智客户端，支持完整的语音聊天功能
 */
export class Client {
    constructor(websocketUrl, accessToken, deviceId, clientId) {
        const config = {
            websocketUrl,
            accessToken,
            deviceId,
            clientId
        };

        this.protocol = new WebSocketProtocol(config);
        this.setupEventListeners();

        // 状态管理
        this.deviceState = DeviceState.IDLE;
        this.keepListening = false;
        this.aborted = false;

        // 麦克风录音器
        this.micRecorder = null;
        this.isRecordingFromMic = false;

        // 音频播放器
        this.audioPlayer = new NodeAudioPlayer();

        // 添加播放完成回调
        this.audioPlayer.onPlaybackFinished = () => {
            this.handlePlaybackFinished();
        };
    }

    /**
     * 设置事件监听器
     */
    setupEventListeners() {
        // 连接成功
        this.protocol.on('connected', () => {
            console.log('✅ WebSocket 连接成功');
            this.setDeviceState(DeviceState.IDLE);
        });

        // 音频通道打开
        this.protocol.on('audioChannelOpened', () => {
            console.log('🎵 音频通道已打开');
        });

        // 音频通道关闭
        this.protocol.on('audioChannelClosed', () => {
            console.log('🔇 音频通道已关闭');
            this.setDeviceState(DeviceState.IDLE);
        });

        // 接收到音频数据
        this.protocol.on('incomingAudio', (audioData) => {
            if (this.deviceState === DeviceState.SPEAKING && audioData.length > 0) {
                this.audioPlayer.processAudioData(audioData);
            }
        });

        // 接收到 JSON 消息
        this.protocol.on('incomingJson', (jsonData) => {
            this.handleIncomingJson(jsonData);
        });

        // 网络错误
        this.protocol.on('networkError', (error) => {
            console.error('❌ 网络错误:', error);
            this.setDeviceState(DeviceState.IDLE);
        });
    }

    /**
     * 处理接收到的JSON消息
     */
    handleIncomingJson(jsonData) {
        console.log('📨 接收到消息:', jsonData);

        const msgType = jsonData.type;

        switch (msgType) {
            case 'tts':
                this.handleTtsMessage(jsonData);
                break;
            case 'stt':
                this.handleSttMessage(jsonData);
                break;
            case 'llm':
                this.handleLlmMessage(jsonData);
                break;
            case 'error':
                console.warn('⚠️ 服务器错误:', jsonData.message);
                break;
            default:
                console.log('📋 其他消息:', jsonData);
        }
    }

    /**
     * 处理TTS消息
     */
    handleTtsMessage(data) {
        const state = data.state;

        if (state === 'start') {
            console.log('🗣️ 开始播放AI回复');
            this.setDeviceState(DeviceState.SPEAKING);
            this.aborted = false;
        } else if (state === 'stop') {
            console.log('🔇 AI回复播放完成');
            this.handleTtsStop();
        } else if (state === 'sentence_start') {
            const text = data.text;
            if (text) {
                console.log('💬 AI回复内容:', text);
            }
        }
    }

    /**
     * 处理TTS停止事件
     */
    async handleTtsStop() {
        // 等待音频播放完成
        await this.waitForAudioPlaybackComplete();

        if (this.keepListening && !this.aborted) {
            // 自动开启下一轮监听
            console.log('🎤 自动开启下一轮监听...');
            await this.startListening(ListeningMode.AUTO_STOP);
        } else {
            this.setDeviceState(DeviceState.IDLE);
        }
    }

    /**
     * 等待音频播放完成
     */
    async waitForAudioPlaybackComplete() {
        return new Promise((resolve) => {
            const checkInterval = setInterval(() => {
                if (!this.audioPlayer.isPlaying) {
                    clearInterval(checkInterval);
                    // 额外等待一点时间确保播放完全结束
                    setTimeout(resolve, 200);
                }
            }, 100);
        });
    }

    /**
     * 处理STT消息
     */
    handleSttMessage(data) {
        const text = data.text;
        if (text) {
            console.log('🎤 用户语音识别结果:', text);
        }
    }

    /**
     * 处理LLM消息
     */
    handleLlmMessage(data) {
        const emotion = data.emotion;
        if (emotion) {
            console.log('😊 AI情感状态:', emotion);
        }
    }

    /**
     * 设置设备状态
     */
    setDeviceState(newState) {
        if (this.deviceState !== newState) {
            console.log(`🔄 状态变化: ${this.deviceState} -> ${newState}`);
            this.deviceState = newState;

            // 根据状态更新UI或触发回调
            this.onStateChanged && this.onStateChanged(newState);
        }
    }

    /**
     * 发送文字消息（模拟唤醒词检测）
     */
    async sendTextMessage(text) {
        if (!this.protocol.isAudioChannelOpened()) {
            await this.protocol.connect();
        }

        const message = {
            session_id: this.protocol.sessionId || '',
            type: 'listen',
            state: 'detect',
            text: text
        };

        console.log('💬 发送文字消息:', text);
        await this.protocol.sendText(JSON.stringify(message));

        // 设置为持续监听模式
        this.keepListening = true;
        await this.startListening(ListeningMode.AUTO_STOP);
    }

    /**
     * 开始监听
     */
    async startListening(mode = ListeningMode.MANUAL) {
        const modeMap = {
            [ListeningMode.ALWAYS_ON]: 'realtime',
            [ListeningMode.AUTO_STOP]: 'auto',
            [ListeningMode.MANUAL]: 'manual'
        };

        const message = {
            session_id: this.protocol.sessionId || '',
            type: 'listen',
            state: 'start',
            mode: modeMap[mode]
        };

        await this.protocol.sendText(JSON.stringify(message));
        this.setDeviceState(DeviceState.LISTENING);

        // 启动麦克风录音
        if (!this.isRecordingFromMic) {
            await this.startMicrophoneRecording();
        }
    }

    /**
     * 停止监听
     */
    async stopListening() {
        const message = {
            session_id: this.protocol.sessionId || '',
            type: 'listen',
            state: 'stop'
        };

        await this.protocol.sendText(JSON.stringify(message));
        this.stopMicrophoneRecording();
        this.setDeviceState(DeviceState.IDLE);
    }

    /**
     * 启动麦克风录音
     */
    async startMicrophoneRecording() {
        try {
            if (this.isRecordingFromMic) {
                return;
            }

            console.log('🎤 启动麦克风录音...');

            this.micRecorder = new MicrophoneOpusRecorder({
                sampleRate: 16000,
                channels: 1,
                frameSize: 320
            });

            this.micRecorder.onOpusData = (opusData) => {
                if (this.protocol.isAudioChannelOpened() && this.deviceState === DeviceState.LISTENING) {
                    this.protocol.sendAudio(opusData);
                }
            };

            this.micRecorder.onError = (error) => {
                console.error('❌ 麦克风录音错误:', error);
                this.stopMicrophoneRecording();
            };

            this.micRecorder.startRecording();
            this.isRecordingFromMic = true;
            console.log('🎤 麦克风录音已启动');

        } catch (error) {
            console.error('❌ 启动麦克风录音失败:', error);
        }
    }

    /**
     * 停止麦克风录音
     */
    stopMicrophoneRecording() {
        if (this.micRecorder && this.isRecordingFromMic) {
            this.micRecorder.stopRecording();
            this.micRecorder = null;
            this.isRecordingFromMic = false;
            console.log('🔇 已停止麦克风录音');
        }
    }

    /**
     * 开始语音聊天
     */
    async startVoiceChat() {
        try {
            console.log('🚀 开始语音聊天...');
            this.setDeviceState(DeviceState.CONNECTING);

            const success = await this.protocol.connect();
            if (success) {
                this.keepListening = true;
                // 发送一个欢迎消息开始对话
                await this.sendTextMessage('hi');
            } else {
                console.error('❌ 连接失败');
                this.setDeviceState(DeviceState.IDLE);
            }
        } catch (error) {
            console.error('❌ 启动语音聊天失败:', error);
            this.setDeviceState(DeviceState.IDLE);
        }
    }

    /**
     * 停止语音聊天
     */
    async stopVoiceChat() {
        console.log('🛑 停止语音聊天...');
        this.keepListening = false;
        this.aborted = true;

        await this.stopListening();
        this.setDeviceState(DeviceState.IDLE);
    }

    /**
     * 打断对话
     */
    async interruptConversation() {
        if (this.deviceState === DeviceState.SPEAKING) {
            console.log('⚡ 打断AI播放...');
            this.aborted = true;

            // 发送中止消息
            const message = {
                session_id: this.protocol.sessionId || '',
                type: 'abort',
                reason: 'user_interruption'
            };

            await this.protocol.sendText(JSON.stringify(message));

            // 停止音频播放
            this.audioPlayer.stop();

            // 立即开始监听
            await this.startListening(ListeningMode.AUTO_STOP);
        }
    }

    /**
     * 完全断开连接
     */
    async disconnect() {
        console.log('🛑 断开连接...');
        this.keepListening = false;
        this.stopMicrophoneRecording();
        await this.protocol.closeAudioChannel();
        this.protocol.destroy();
        this.setDeviceState(DeviceState.IDLE);
        console.log('✅ 已完全断开连接');
    }
}

// await activator.start(deviceId)
// await activator.start(deviceId)