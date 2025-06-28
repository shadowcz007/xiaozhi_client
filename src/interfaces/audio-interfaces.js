/**
 * 音频录音器接口
 */
export class IAudioRecorder {
    constructor(options = {}) {
        this.sampleRate = options.sampleRate || 16000;
        this.channels = options.channels || 1;
        this.frameSize = options.frameSize || 320;

        // 事件回调
        this.onOpusData = null;
        this.onError = null;
    }

    /**
     * 开始录音
     */
    async startRecording() {
        throw new Error('startRecording method must be implemented');
    }

    /**
     * 停止录音
     */
    async stopRecording() {
        throw new Error('stopRecording method must be implemented');
    }

    /**
     * 暂停录音
     */
    pauseRecording() {
        throw new Error('pauseRecording method must be implemented');
    }

    /**
     * 恢复录音
     */
    resumeRecording() {
        throw new Error('resumeRecording method must be implemented');
    }

    /**
     * 是否正在录音
     */
    isRecording() {
        throw new Error('isRecording method must be implemented');
    }

    /**
     * 清理资源
     */
    cleanup() {
        throw new Error('cleanup method must be implemented');
    }
}

/**
 * 音频播放器接口
 */
export class IAudioPlayer {
    constructor() {
        this.isPlaying = false;
        this.onPlaybackFinished = null;
    }

    /**
     * 处理音频数据
     */
    async processAudioData(opusData) {
        throw new Error('processAudioData method must be implemented');
    }

    /**
     * 停止播放
     */
    async stop() {
        throw new Error('stop method must be implemented');
    }

    /**
     * 接收到TTS停止信号，表明不会再有新的音频数据
     */
    signalTtsStop() {
        // 默认实现为空，具体实现由子类提供
    }

    /**
     * 获取缓冲状态
     */
    getBufferStatus() {
        throw new Error('getBufferStatus method must be implemented');
    }

    /**
     * 清理资源
     */
    cleanup() {
        throw new Error('cleanup method must be implemented');
    }
}

/**
 * WebSocket协议接口
 */
export class IWebSocketProtocol {
    constructor(config = {}) {
        this.config = config;
        this.connected = false;
        this.sessionId = null;
    }

    /**
     * 连接到服务器
     */
    async connect() {
        throw new Error('connect method must be implemented');
    }

    /**
     * 发送音频数据
     */
    async sendAudio(data) {
        throw new Error('sendAudio method must be implemented');
    }

    /**
     * 发送文本消息
     */
    async sendText(message) {
        throw new Error('sendText method must be implemented');
    }

    /**
     * 检查音频通道是否打开
     */
    isAudioChannelOpened() {
        throw new Error('isAudioChannelOpened method must be implemented');
    }

    /**
     * 打开音频通道
     */
    async openAudioChannel() {
        throw new Error('openAudioChannel method must be implemented');
    }

    /**
     * 关闭音频通道
     */
    async closeAudioChannel() {
        throw new Error('closeAudioChannel method must be implemented');
    }

    /**
     * 检查是否已连接
     */
    isConnected() {
        throw new Error('isConnected method must be implemented');
    }

    /**
     * 销毁连接
     */
    destroy() {
        throw new Error('destroy method must be implemented');
    }

    /**
     * 事件监听 (EventEmitter 风格)
     */
    on(event, callback) {
        throw new Error('on method must be implemented');
    }

    /**
     * 触发事件
     */
    emit(event, ...args) {
        throw new Error('emit method must be implemented');
    }
}