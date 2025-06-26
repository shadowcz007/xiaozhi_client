import WebSocket from 'ws';
import { EventEmitter } from 'events';
import crypto from 'crypto';

/**
 * WebSocket 协议类 - JavaScript 版本
 * 基于小智项目的 WebSocket 通信协议
 */
class WebSocketProtocol extends EventEmitter {
    constructor(config = {}) {
        super();

        // 音频配置常量
        this.AudioConfig = {
            INPUT_SAMPLE_RATE: 16000,
            CHANNELS: 1,
            FRAME_DURATION: 20 // ms
        };

        // 连接状态
        this.websocket = null;
        this.connected = false;
        this.helloReceived = false;

        // 配置信息（从您提供的配置中获取）
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
        this.on('error', (error) => {
            console.error('WebSocket协议错误:', error);
        });

        this.on('networkError', (error) => {
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
                                this.emit('connected');
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
                    this.emit('audioChannelClosed');
                });

                this.websocket.on('error', (error) => {
                    clearTimeout(connectTimeout);
                    console.error('WebSocket 错误:', error);
                    this.emit('networkError', `连接错误: ${error.message}`);
                    reject(error);
                });
            });

        } catch (error) {
            console.error('WebSocket 连接失败:', error);
            this.emit('networkError', `无法连接服务: ${error.message}`);
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
                        this.emit('incomingAudio', data);
                        return;
                    }
                } else {
                    messageStr = data;
                }
                console.log('收到消息:', messageStr);
                // 处理 JSON 消息
                try {
                    const jsonData = JSON.parse(messageStr);
                    const msgType = jsonData.type;
                    // console.log('收到消息:', msgType);
                    if (msgType === 'hello') {
                        this.handleServerHello(jsonData);
                    } else {
                        this.emit('incomingJson', jsonData);
                    }
                } catch (parseError) {
                    console.error('JSON 解析失败:', parseError.message);
                    console.error('原始消息:', messageStr);
                }
            } else {
                // 二进制音频数据
                this.emit('incomingAudio', data);
            }

        } catch (error) {
            console.error('处理消息时出错:', error);
            this.emit('error', error);
        }
    }

    /**
     * 处理服务器的 hello 消息
     * @param {Object} data 服务器 hello 消息数据
     */
    handleServerHello(data) {
        try {
            // 验证传输方式 - 放宽验证条件
            const transport = data.transport;
            if (transport && transport !== 'websocket') {
                console.warn('传输方式不匹配，但继续处理:', transport);
            };

            console.log('收到服务器 hello 消息:', data);
            // 设置 hello 接收状态
            this.helloReceived = true;

            this.audio_params = data.audio_params;
            this.session_id = data.session_id;

            // 通知音频通道已打开
            this.emit('audioChannelOpened');

        } catch (error) {
            console.error('处理服务器 hello 消息时出错:', error);
            this.emit('networkError', `处理服务器响应失败: ${error.message}`);
        }
    }

    /**
     * 发送音频数据
     * @param {Buffer} data 音频数据
     */
    async sendAudio(data) {
        if (!this.isAudioChannelOpened()) {
            console.warn('音频通道未打开，无法发送音频数据');
            return false;
        }

        try {
            this.websocket.send(data);
            return true;
        } catch (error) {
            console.error('发送音频数据失败:', error);
            this.emit('networkError', `发送音频数据失败: ${error.message}`);
            return false;
        }
    }

    /**
     * 发送文本消息
     * @param {string} message 文本消息
     */
    async sendText(message) {
        if (!this.websocket) {
            console.error('WebSocket 连接不存在');
            return false;
        }

        try {
            this.websocket.send(message);
            return true;
        } catch (error) {
            console.error('发送文本消息失败:', error);
            await this.closeAudioChannel();
            this.emit('networkError', '客户端已关闭');
            return false;
        }
    }

    /**
     * 检查音频通道是否打开
     * @returns {boolean} 音频通道是否打开
     */
    isAudioChannelOpened() {
        return this.websocket !== null &&
            this.connected &&
            this.websocket.readyState === WebSocket.OPEN;
    }

    /**
     * 打开音频通道
     * @returns {Promise<boolean>} 是否成功打开
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
        if (this.websocket) {
            try {
                this.websocket.close();
                this.websocket = null;
                this.connected = false;
                this.helloReceived = false;
                this.emit('audioChannelClosed');
            } catch (error) {
                console.error('关闭 WebSocket 连接失败:', error);
            }
        }
    }

    /**
     * 获取连接状态
     * @returns {boolean} 是否已连接
     */
    isConnected() {
        return this.connected;
    }

    /**
     * 更新配置
     * @param {Object} newConfig 新的配置
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
        this.closeAudioChannel();
        this.removeAllListeners();
    }
}

export { WebSocketProtocol };