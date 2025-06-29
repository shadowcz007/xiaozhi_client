import { PlatformFactory } from './platform-factory.js';

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
    constructor(websocketUrl, accessToken, deviceId, clientId, options = {}) {
        this.config = {
            websocketUrl,
            accessToken,
            deviceId,
            clientId,
            ...options
        };

        // 组件将在 init 方法中异步初始化
        this.protocol = null;
        this.audioPlayer = null;

        // 状态管理
        this.deviceState = DeviceState.IDLE;
        this.keepListening = false;
        this.aborted = false;

        // 麦克风录音器 (动态创建)
        this.micRecorder = null;
        this.isRecordingFromMic = false;
        this.audioBuffer = []; // 用于在监听状态完全就绪前缓存音频
        this.isStartingToListen = false; // 用于标记正在启动监听过程

        this.onTtsStart = (data) => {};
        this.onTtsStop = (data) => {};
        this.onTtsSentenceStart = (data) => {};
        this.onStt = (data) => {};
        this.onLlm = (data) => {};
        this.onStateChanged = (state) => {};
    }

    /**
     * 异步初始化客户端
     */
    async init() {
        // 使用平台工厂创建适合当前环境的实现
        this.protocol = await PlatformFactory.createWebSocketProtocol(this.config);
        this.setupEventListeners();

        // 音频播放器
        this.audioPlayer = await PlatformFactory.createAudioPlayer(this.config.audioPlayerOptions);

        // 添加播放完成回调
        this.audioPlayer.onPlaybackFinished = this.handlePlaybackFinished.bind(this);

        // 记录平台信息
        // console.log('📱 客户端平台信息:', PlatformFactory.getPlatformInfo());
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
            console.log('🎵 音频通道已打开', this.protocol.sessionId);
        });

        // 音频通道关闭
        this.protocol.on('audioChannelClosed', () => {
            console.log('🔇 音频通道已关闭');
            this.setDeviceState(DeviceState.IDLE);
        });

        // 接收到音频数据
        this.protocol.on('incomingAudio', (audioData) => {
            const audioSize = audioData.byteLength || audioData.length || audioData.size;
            // console.log('🎤 接收到音频数据:', audioSize);
            if (this.deviceState === DeviceState.SPEAKING && !this.aborted && audioSize > 0) {
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
        // console.log('📨 接收到消息:', jsonData);

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
            // console.log('🗣️ 开始播放AI回复');
            this.setDeviceState(DeviceState.SPEAKING);
            this.aborted = false;
            this.onTtsStart && this.onTtsStart(data);
        } else if (state === 'stop') {
            // console.log('🔇 AI回复播放完成');
            this.handleTtsStop();
            this.onTtsStop && this.onTtsStop(data);
        } else if (state === 'sentence_start') {
            const text = data.text;
            this.onTtsSentenceStart && this.onTtsSentenceStart(data);
            if (text) {
                console.log('💬 AI回复内容:', text);
            }
        }
    }

    /**
     * 处理TTS停止事件
     */
    handleTtsStop() {
        // console.log('TTS 流结束，通知播放器准备检查播放完成状态。');
        // 当收到TTS停止消息时，我们通知播放器。
        // 播放器将负责在所有已缓冲的音频播放完毕后，调用 onPlaybackFinished。
        // 这也处理了没有收到任何音频的边缘情况（播放器会立即回调）。
        if (this.audioPlayer) {
            this.audioPlayer.signalTtsStop();
        }
    }

    /**
     * 处理播放完成事件
     */
    async handlePlaybackFinished() {
        if (this.audioPlayer) {
            this.audioPlayer.stop();
        }

        if (this.keepListening && !this.aborted) {
            // 先停止当前录音
            await this.stopListening();

            // 自动开启下一轮监听
            // console.log('🎤 播放完成，自动开启下一轮监听...');
            await this.startListening(ListeningMode.AUTO_STOP);
        } else {
            // 如果不是因为被打断而停止，则将状态设置为空闲
            // 在打断场景下，状态由 interruptConversation 方法管理
            if (!this.aborted) {
                this.setDeviceState(DeviceState.IDLE);
            }
        }
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
            // console.log('😊 AI情感状态:', emotion);
            this.onLlm && this.onLlm(data);
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
     * 发送音频数据，包含缓冲逻辑
     * @param {ArrayBuffer} audioData - PCM 或 Opus 音频数据
     */
    sendAudio(audioData) {
        const isAudioChannelOpen = this.protocol.isAudioChannelOpened();
        const isListening = this.deviceState === DeviceState.LISTENING;

        if (isAudioChannelOpen) {
            if (isListening) {
                // 状态就绪，直接发送
                this.protocol.sendAudio(audioData);
            } else if (this.isStartingToListen) {
                // 正在启动，先缓冲
                this.audioBuffer.push(audioData);
            }
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
        if (this.isRecordingFromMic) {
            console.warn('🎤 录音器已在运行，无法再次启动');
            return;
        }

        // 确保 WebSocket 连接和音频通道都已就绪
        try {
            if (!this.protocol || !this.protocol.isConnected()) {
                console.log('🔌 WebSocket 未连接，正在尝试连接...');
                await this.protocol.connect();
            }
            if (!this.protocol.isAudioChannelOpened()) {
                console.log('🎵 音频通道未打开，正在尝试打开...');
                await this.protocol.openAudioChannel();
            }
        } catch (error) {
            console.error('❌ 无法建立监听所需的连接:', error);
            this.setDeviceState(DeviceState.IDLE); // 连接失败，设置为空闲
            return;
        }

        // 设置启动标志并清空缓冲区
        this.isStartingToListen = true;
        this.audioBuffer = [];

        // 1. 启动麦克风录音，此时 onOpusData 会开始缓冲音频
        await this.startMicrophoneRecording();

        // 2. 检查录音是否成功启动
        if (this.isRecordingFromMic) {

            // 3. 发送 "listen:start" 消息
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

            // console.log('🎤 麦克风已就绪，通知服务器开始监听...', message);

            // 4. 设置最终状态
            this.setDeviceState(DeviceState.LISTENING);

            // 5. 停止缓冲，并发送所有已缓冲的音频
            this.isStartingToListen = false;
            if (this.audioBuffer.length > 0) {
                this.audioBuffer = []; // 清空缓冲区
            }
            console.log('✅ 监听状态完全就绪，开始实时发送音频。');

        } else {
            // 如果麦克风启动失败，重置状态
            this.isStartingToListen = false;
            this.audioBuffer = [];
            console.warn('⚠️ 麦克风启动失败，监听流程中止。');
        }
    }

    /**
     * 停止监听
     * 这个方法现在只负责停止录音和通知服务器，不再改变客户端状态。
     */
    async stopListening() {
        // 停止麦克风录音
        await this.stopMicrophoneRecording();

        // 如果会话ID无效，则无需发送消息
        if (!this.protocol.sessionId) {
            console.warn('⚠️ 会话ID无效，无法发送停止监听消息');
            return;
        }

        const message = {
            session_id: this.protocol.sessionId,
            type: 'listen',
            state: 'stop'
        };

        await this.protocol.sendText(JSON.stringify(message));
    }

    /**
     * 停止监听并设置状态为空闲
     */
    async stopListeningAndSetIdle() {
        await this.stopListening();
        this.setDeviceState(DeviceState.IDLE);
    }

    /**
     * 开始从麦克风录音
     */
    async startMicrophoneRecording() {
        if (this.isRecordingFromMic) return;

        console.log('🎤 准备启动麦克风录音...');
        try {
            // 使用平台工厂创建录音机实例
            this.micRecorder = await PlatformFactory.createAudioRecorder(this.config.audioRecorderOptions);

            // 设置数据回调
            const useOpus = this.config.audioRecorderOptions && this.config.audioRecorderOptions.useOpus;
            if (useOpus) {
                // Node.js 环境或明确使用 Opus 的浏览器
                this.micRecorder.onOpusData = (opusData) => {
                    this.sendAudio(opusData);
                };
            } else {
                // 浏览器环境发送 PCM
                this.micRecorder.onPcmData = (pcmData) => {
                    this.sendAudio(pcmData);
                };
            }


            // 设置错误回调
            this.micRecorder.onError = (error) => {
                console.error('❌ 麦克风录音错误:', error);
                this.stopListeningAndSetIdle(); // 录音出错时停止监听
            };

            await this.micRecorder.startRecording();
            this.isRecordingFromMic = true;
            console.log('✅ 麦克风录音已启动');

        } catch (error) {
            console.error('❌ 启动麦克风录音失败:', error);
            throw error;
        }
    }

    /**
     * 停止麦克风录音
     */
    async stopMicrophoneRecording() {
        if (!this.isRecordingFromMic) {
            return; // Nothing to do
        }

        if (this.micRecorder) {
            try {
                await this.micRecorder.stopRecording();
                this.micRecorder.cleanup();
            } catch (error) {
                console.error('停止录音器时出错:', error);
            } finally {
                this.micRecorder = null;
                this.isRecordingFromMic = false;
                console.log('🎤 停止麦克风录音');
            }
        } else {
            // just in case micRecorder is null but isRecordingFromMic is true
            this.isRecordingFromMic = false;
        }
    }


    /**
     * 开始语音聊天
     */
    async startVoiceChat() {
        // 首先停止之前可能存在的录音和播放
        if (this.isRecordingFromMic) {
            await this.stopMicrophoneRecording();
        }

        // 停止音频播放器
        if (this.audioPlayer && this.audioPlayer.isPlaying) {
            this.audioPlayer.stop();
        }

        // 重置所有状态
        this.keepListening = true;
        this.aborted = false;
        this.isStartingToListen = false;
        this.audioBuffer = [];
        this.setDeviceState(DeviceState.CONNECTING);

        if (!this.protocol || !this.protocol.isConnected()) {
            await this.protocol.connect();
        }
        await this.protocol.openAudioChannel();

        console.log('🎤 开始语音聊天，进入监听状态...', this.protocol.sessionId);
        await this.startListening(ListeningMode.AUTO_STOP);
    }

    /**
     * 停止语音聊天
     */
    async stopVoiceChat() {
        console.log('🛑 停止语音聊天...');
        this.keepListening = false;
        this.aborted = true;

        await this.stopListeningAndSetIdle();

        if (this.protocol.isAudioChannelOpened()) {
            await this.protocol.closeAudioChannel();
        }
    }

    /**
     * 打断对话
     */
    async interruptConversation() {
        if (this.deviceState !== DeviceState.SPEAKING) {
            console.warn('⚠️ AI 未在说话，无需打断');
            return;
        }

        console.log('⚡ 打断 AI 对话');
        this.aborted = true;

        // 停止播放
        if (this.audioPlayer && this.audioPlayer.isPlaying) {
            this.audioPlayer.stop();
        }

        // 停止上一个监听（如果有），但不要改变状态
        await this.stopListening();

        // 立即开始新一轮监听
        console.log('🎤 AI 已被打断，立即开始新一轮监听...');
        await this.startListening(ListeningMode.AUTO_STOP);
    }

    /**
     * 断开连接
     */
    async disconnect() {
        await this.stopVoiceChat();
        if (this.protocol) {
            await this.protocol.destroy();
        }
        console.log('👋 客户端已断开连接');
    }
}

// await activator.start(deviceId)
// await activator.start(deviceId)