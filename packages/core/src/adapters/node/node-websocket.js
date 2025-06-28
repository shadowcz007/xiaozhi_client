import { IWebSocketProtocol } from '../../interfaces/audio-interfaces.js';
import { EventEmitter } from 'events';
import WebSocket from 'ws';
import crypto from 'crypto';

/**
 * Node.js 环境下的 WebSocket 协议实现
 * 基于 ws 库，封装原有的 WebSocketProtocol 功能
 */
export class NodeWebSocketProtocol extends IWebSocketProtocol {
    constructor(config = {}) {
        super(config);

        // 继承 EventEmitter 功能
        this.eventEmitter = new EventEmitter();

        // 音频配置常量
        this.AudioConfig = {
            INPUT_SAMPLE_RATE: 16000,
            CHANNELS: 1,
            FRAME_DURATION: 20 // ms
        };

        // 连接状态
        this.websocket = null;
        this.helloReceived = false;

        // 配置信息
        this.config = {
            websocketUrl: config.websocketUrl || 'wss://api.tenclass.net/xiaozhi/v1/',
            accessToken: config.accessToken || 'test-token',
            deviceId: config.deviceId || this.generateDeviceId(),
            clientId: config.clientId || this.generateClientId(),
            ...config
        };

        // WebSocket 请求头
        this.headers = {
            'Authorization': `Bearer ${this.config.accessToken}`,
            'Protocol-Version': '1',
            'Device-Id': this.config.deviceId,
            'Client-Id': this.config.clientId
        };

        // 连接超时时间
        this.connectTimeout = 15000; // 增加到15秒

        // 绑定事件处理方法
        this.setupEventHandlers();
    }

    /**
     * 生成设备ID
     */
    generateDeviceId() {
        return crypto.randomBytes(16).toString('hex');
    }

    /**
     * 生成客户端ID
     */
    generateClientId() {
        return `client_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }

    /**
     * 设置事件处理器
     */
    setupEventHandlers() {
        // 可以在这里设置默认的事件处理器
        this.eventEmitter.on('error', (error) => {
            console.error('WebSocket协议错误:', error);
        });

        this.eventEmitter.on('networkError', (error) => {
            console.error('网络错误:', error);
        });
    }

    /**
     * 连接到 WebSocket 服务器
     * @returns {Promise<boolean>} 连接是否成功
     */
    async connect() {
        try {
            // 重置状态
            this.helloReceived = false;
            this.connected = false;

            // 创建 WebSocket 连接
            return new Promise((resolve, reject) => {
                const connectTimeout = setTimeout(() => {
                    reject(new Error('连接超时'));
                }, this.connectTimeout);

                this.websocket = new WebSocket(this.config.websocketUrl, {
                    headers: this.headers
                });

                this.websocket.on('open', async() => {
                    clearTimeout(connectTimeout);

                    try {
                        // 发送客户端 hello 消息
                        await this.sendHelloMessage();

                        // 等待服务器 hello 响应
                        const helloTimeout = setTimeout(() => {
                            console.error('等待服务器 hello 响应超时');
                            reject(new Error('等待服务器 hello 响应超时'));
                        }, this.connectTimeout);

                        const checkHello = () => {
                            if (this.helloReceived) {
                                clearTimeout(helloTimeout);
                                this.connected = true;
                                this.eventEmitter.emit('connected');
                                resolve(true);
                            } else {
                                setTimeout(checkHello, 100);
                            }
                        };
                        checkHello();

                    } catch (error) {
                        clearTimeout(connectTimeout);
                        reject(error);
                    }
                });

                this.websocket.on('message', (data) => {
                    this.handleMessage(data);
                });

                this.websocket.on('close', (code, reason) => {
                    clearTimeout(connectTimeout);
                    this.connected = false;
                    this.eventEmitter.emit('audioChannelClosed');
                });

                this.websocket.on('error', (error) => {
                    clearTimeout(connectTimeout);
                    console.error('WebSocket 错误:', error);
                    this.eventEmitter.emit('networkError', `连接错误: ${error.message}`);
                    reject(error);
                });
            });

        } catch (error) {
            console.error('WebSocket 连接失败:', error);
            this.eventEmitter.emit('networkError', `无法连接服务: ${error.message}`);
            return false;
        }
    }

    /**
     * 发送客户端 hello 消息
     */
    async sendHelloMessage() {
        const helloMessage = {
            type: 'hello',
            version: 1,
            transport: 'websocket',
            audio_params: {
                format: 'opus',
                sample_rate: this.AudioConfig.INPUT_SAMPLE_RATE,
                channels: this.AudioConfig.CHANNELS,
                frame_duration: this.AudioConfig.FRAME_DURATION
            }
        };

        const messageString = JSON.stringify(helloMessage);
        await this.sendText(messageString);
    }

    /**
     * 处理接收到的消息
     * @param {Buffer|string} data 接收到的数据
     */
    handleMessage(data) {
        try {
            // 判断是文本消息还是二进制数据
            if (typeof data === 'string' || Buffer.isBuffer(data)) {
                let messageStr;
                if (Buffer.isBuffer(data)) {
                    // 尝试将 Buffer 转换为字符串
                    try {
                        messageStr = data.toString('utf8');
                        // 验证是否为有效的 JSON
                        JSON.parse(messageStr);
                    } catch (jsonError) {
                        // 如果不是有效的 JSON，则当作音频数据处理
                        this.eventEmitter.emit('incomingAudio', data);
                        return;
                    }
                } else {
                    messageStr = data;
                }
                // console.log('收到消息:', messageStr);
                // 处理 JSON 消息
                try {
                    const jsonData = JSON.parse(messageStr);
                    const msgType = jsonData.type;

                    if (msgType === 'hello') {
                        this.handleServerHello(jsonData);
                    } else {
                        this.eventEmitter.emit('incomingJson', jsonData);
                    }
                } catch (parseError) {
                    console.error('解析 JSON 消息失败:', parseError);
                }
            } else {
                // 其他类型的数据当作音频数据处理
                this.eventEmitter.emit('incomingAudio', data);
            }
        } catch (error) {
            console.error('处理消息失败:', error);
        }
    }

    /**
     * 处理服务器 hello 消息
     */
    handleServerHello(data) {
        console.log('收到服务器 hello 响应:', data);

        if (data.type == 'hello') {
            this.helloReceived = true;
            this.sessionId = data.session_id;

            // 触发音频通道打开事件
            this.eventEmitter.emit('audioChannelOpened', data);

            console.log('✅ 服务器 hello 成功，会话 ID:', this.sessionId);
        } else {
            console.error('❌ 服务器 hello 失败:', data.message);
            this.eventEmitter.emit('networkError', `服务器拒绝连接: ${data.message}`);
        }
    }

    /**
     * 发送音频数据
     */
    async sendAudio(data) {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket 连接未建立');
        }

        try {
            // console.log(`[WebSocket] Sending audio data: ${data.byteLength} bytes.`);
            this.websocket.send(data);
        } catch (error) {
            console.error('发送音频数据失败:', error);
            throw error;
        }
    }

    /**
     * 发送文本消息
     */
    async sendText(message) {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket 连接未建立');
        }

        try {
            this.websocket.send(message);
        } catch (error) {
            console.error('发送文本消息失败:', error);
            throw error;
        }
    }

    /**
     * 检查音频通道是否打开
     */
    isAudioChannelOpened() {
        return this.connected && this.helloReceived &&
            this.websocket && this.websocket.readyState === WebSocket.OPEN;
    }

    /**
     * 打开音频通道
     */
    async openAudioChannel() {
        if (!this.connected) {
            return await this.connect();
        }
        return true;
    }

    /**
     * 关闭音频通道
     */
    async closeAudioChannel() {
        try {
            if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
                this.websocket.close();
            }
            this.connected = false;
            this.helloReceived = false;
            this.sessionId = null;
            console.log('✅ 音频通道已关闭');
        } catch (error) {
            console.error('关闭音频通道失败:', error);
        }
    }

    /**
     * 检查是否已连接
     */
    isConnected() {
        return this.connected && this.websocket && this.websocket.readyState === WebSocket.OPEN;
    }

    /**
     * 更新配置
     */
    updateConfig(newConfig) {
        this.config = {...this.config, ...newConfig };

        // 更新请求头
        this.headers = {
            'Authorization': `Bearer ${this.config.accessToken}`,
            'Protocol-Version': '1',
            'Device-Id': this.config.deviceId,
            'Client-Id': this.config.clientId
        };
    }

    /**
     * 销毁连接
     */
    destroy() {
        try {
            this.closeAudioChannel();

            if (this.websocket) {
                this.websocket.removeAllListeners();
                this.websocket = null;
            }

            if (this.eventEmitter) {
                this.eventEmitter.removeAllListeners();
            }

            console.log('✅ WebSocket 协议已销毁');
        } catch (error) {
            console.error('销毁 WebSocket 协议失败:', error);
        }
    }

    /**
     * 事件监听 (EventEmitter 风格)
     */
    on(event, callback) {
        this.eventEmitter.on(event, callback);
    }

    /**
     * 移除事件监听
     */
    off(event, callback) {
        this.eventEmitter.off(event, callback);
    }

    /**
     * 触发事件
     */
    emit(event, ...args) {
        this.eventEmitter.emit(event, ...args);
    }
}