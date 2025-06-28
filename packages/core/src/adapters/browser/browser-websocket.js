import { IWebSocketProtocol } from '../../interfaces/audio-interfaces.js';

/**
 * 浏览器环境下的 WebSocket 协议实现
 * 基于原生 WebSocket API
 */
export class BrowserWebSocket extends IWebSocketProtocol {
    constructor(config) {
        super();
        this.config = config;
        this.ws = null;
        this._isConnected = false;
        this.isAudioOpen = false;
        this.eventHandlers = new Map();
        this.helloReceived = false;
        this.connectTimeout = 15000; // 15秒连接超时
    }

    /**
     * 检查 WebSocket 是否已连接并准备好通信
     * @returns {boolean}
     */
    isConnected() {
        return this._isConnected && this.ws && this.ws.readyState === WebSocket.OPEN;
    }

    async connect() {
        return new Promise((resolve, reject) => {
            const connectTimeoutTimer = setTimeout(() => {
                if (this.ws) {
                    this.ws.close();
                }
                reject(new Error('连接超时'));
            }, this.connectTimeout);

            try {
                // 在浏览器环境中，我们通过URL参数传递token
                // 为了与 Node.js 实现保持一致，这里的参数名和格式尽量模拟 Node.js 的 Headers
                const url = new URL(this.config.websocketUrl);
                url.searchParams.append('Authorization', `Bearer ${this.config.accessToken}`);
                url.searchParams.append('Device-Id', this.config.deviceId);
                url.searchParams.append('Client-Id', this.config.clientId);
                url.searchParams.append('Protocol-Version', '1');

                this.ws = new WebSocket(url.toString());
                this.helloReceived = false;

                this.ws.onopen = async() => {
                    try {
                        // 发送客户端 hello 消息
                        await this.sendHelloMessage();

                        // 等待服务器 hello 响应
                        const helloTimeout = setTimeout(() => {
                            console.error('等待服务器 hello 响应超时');
                            if (this.ws) {
                                this.ws.close();
                            }
                            reject(new Error('等待服务器 hello 响应超时'));
                        }, this.connectTimeout);

                        const checkHello = () => {
                            if (this.helloReceived) {
                                clearTimeout(helloTimeout);
                                clearTimeout(connectTimeoutTimer);
                                this._isConnected = true;
                                console.log('✅ WebSocket 连接成功，握手完成');
                                this.emit('connected');
                                resolve(true);
                            } else {
                                setTimeout(checkHello, 100);
                            }
                        };
                        checkHello();

                    } catch (error) {
                        clearTimeout(connectTimeoutTimer);
                        reject(error);
                    }
                };

                this.ws.onclose = () => {
                    clearTimeout(connectTimeoutTimer);
                    this._isConnected = false;
                    this.isAudioOpen = false;
                    console.log('WebSocket连接已关闭');
                    this.emit('disconnected');
                };

                this.ws.onerror = (error) => {
                    clearTimeout(connectTimeoutTimer);
                    console.error('WebSocket错误:', error);
                    this.emit('error', error);
                    reject(error);
                };

                this.ws.onmessage = async(event) => {
                    // console.log('接收到消息:', event);
                    // 浏览器 WebSocket API 直接返回数据
                    const data = event.data;
                    if (typeof data === 'string') {
                        try {
                            const message = JSON.parse(data);
                            this.handleMessage(message);
                        } catch (error) {
                            console.error('处理JSON消息错误:', error);
                        }
                    } else if (data instanceof Blob) {
                        const arrayBuffer = await data.arrayBuffer();
                        // console.log('接收到音频数据 (Blob):', arrayBuffer.byteLength);
                        this.emit('incomingAudio', arrayBuffer);
                    } else if (data instanceof ArrayBuffer) {
                        // 处理二进制数据
                        // console.log('接收到音频数据 (ArrayBuffer):', data.byteLength);
                        this.emit('incomingAudio', data);
                    }
                };

            } catch (error) {
                clearTimeout(connectTimeoutTimer);
                console.error('创建WebSocket连接失败:', error);
                reject(error);
            }
        });
    }

    handleMessage(message) {
        if (message.type === 'hello') {
            this.handleServerHello(message);
            return;
        }

        if (!this.helloReceived) {
            console.warn('收到非hello消息，但尚未完成握手，已忽略:', message);
            return;
        }

        // 兼容旧版消息格式
        if (message.type && !this.eventHandlers.has(message.type)) {
            this.emit('incomingJson', message);
            return;
        }

        switch (message.type) {
            case 'text':
                this.emit('text', message.content);
                break;
            case 'audio':
                if (message.data) {
                    this.emit('audio', message.data);
                }
                break;
            case 'status':
                this.emit('status', message.status);
                break;
            case 'tts':
            case 'stt':
            case 'llm':
            case 'error':
                this.emit('incomingJson', message);
                break;
            default:
                this.emit('incomingJson', message);
        }
    }

    /**
     * 处理服务器的 hello 消息
     * @param {Object} data 服务器 hello 消息数据
     */
    handleServerHello(data) {
        try {

            this.helloReceived = true;
            this.audio_params = data.audio_params;
            this.sessionId = data.session_id;
            console.log('收到服务器 hello 消息:', data, this);
            // 触发 audioChannelOpened 以兼容 client.js 逻辑
            this.emit('audioChannelOpened');

        } catch (error) {
            console.error('处理服务器 hello 消息失败:', error);
        }
    }

    /**
     * 发送客户端 hello 消息
     */
    async sendHelloMessage() {
        // 与 Node.js 端对齐，使用相同的音频参数
        const useOpus = this.config.audioPlayerOptions && this.config.audioPlayerOptions.useOpus;
        const helloMessage = {
            type: 'hello',
            version: 1,
            transport: 'websocket',
            audio_params: {
                format: useOpus ? 'opus' : 'pcm',
                sample_rate: useOpus ? 16000 : 24000, // Opus 标准采样率通常为 16000，与 Node 对齐
                channels: 1,
                frame_duration: 20
            }
        };

        const messageString = JSON.stringify(helloMessage);
        await this.sendText(messageString);
    }

    /**
     * 发送音频数据
     * @param {ArrayBuffer} audioData - 音频数据
     */
    sendAudio(audioData) {
        if (this.isAudioChannelOpened() && audioData && audioData.byteLength > 0) {
            // console.log(`[WebSocket] Sending audio data: ${audioData.byteLength} bytes.`);
            this.ws.send(audioData);
        }
    }

    async sendText(message) {
        if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket is not connected or not in OPEN state.');
        }
        this.ws.send(message);
    }

    isAudioChannelOpened() {
        return this.isAudioOpen;
    }

    async openAudioChannel() {
        if (this.helloReceived) {
            this.isAudioOpen = true;
            // 保持静默，因为 hello 消息中已经触发了 audioChannelOpened
        } else {
            console.warn("尚未收到服务器 'hello' 确认，无法打开音频通道");
        }
    }

    async closeAudioChannel() {
        this.isAudioOpen = false;
        if (this.isConnected()) {
            const message = {
                type: 'control',
                action: 'close_audio'
            };
            this.ws.send(JSON.stringify(message));
        }
        this.emit('audioChannelClosed');
    }

    destroy() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this._isConnected = false;
        this.isAudioOpen = false;
        this.eventHandlers.clear();
    }

    on(event, callback) {
        if (!this.eventHandlers.has(event)) {
            this.eventHandlers.set(event, []);
        }
        this.eventHandlers.get(event).push(callback);
    }

    emit(event, ...args) {
        const handlers = this.eventHandlers.get(event);
        if (handlers) {
            handlers.forEach(handler => handler(...args));
        }
    }
}