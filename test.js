import { DeviceActivator } from './device-activator.js';
import { WebSocketProtocol } from './websocket.js';


const activator = new DeviceActivator();
let deviceId = 'e7:40:56:7a:13:9f';
let result = await activator.start(deviceId, false);
console.log('result', result);


/**
 * WebSocket 协议使用示例
 */
class WebSocketExample {
    constructor(websocketUrl, accessToken, deviceId, clientId) {
        // 使用您提供的配置信息
        const config = {
            websocketUrl,
            accessToken,
            deviceId,
            clientId
        };

        this.protocol = new WebSocketProtocol(config);
        this.setupEventListeners();
    }

    /**
     * 设置事件监听器
     */
    setupEventListeners() {
        // 连接成功
        this.protocol.on('connected', () => {
            console.log('✅ WebSocket 连接成功');
        });

        // 音频通道打开
        this.protocol.on('audioChannelOpened', () => {
            console.log('🎵 音频通道已打开，可以开始发送音频数据');
        });

        // 音频通道关闭
        this.protocol.on('audioChannelClosed', () => {
            console.log('🔇 音频通道已关闭');
        });

        // 接收到音频数据
        this.protocol.on('incomingAudio', (audioData) => {
            console.log(`🔊 接收到音频数据: ${audioData.length} 字节`);
            // 在这里处理接收到的音频数据
        });

        // 接收到 JSON 消息
        this.protocol.on('incomingJson', (jsonData) => {
            console.log('📨 接收到 JSON 消息:', jsonData);

            // 检查是否是错误消息
            if (jsonData.type === 'error') {
                console.warn('⚠️ 服务器返回错误:', jsonData.message);
                if (jsonData.session_id) {
                    console.log('🔗 会话ID:', jsonData.session_id);
                }
                // 这是服务器端的处理错误，通常不影响客户端功能
                console.log('💡 提示：这可能是服务器处理测试数据时的正常错误，不影响基本连接功能');
            }

            // 在这里处理接收到的 JSON 消息
        });

        // 网络错误
        this.protocol.on('networkError', (error) => {
            console.error('❌ 网络错误:', error);
        });

        // 其他错误
        this.protocol.on('error', (error) => {
            console.error('❌ 协议错误:', error);
        });
    }

    /**
     * 启动示例
     */
    async start() {
        try {
            console.log('🚀 开始连接 WebSocket 服务器...');

            const success = await this.protocol.connect();

            if (success) {
                console.log('✅ 连接建立成功！');

                // 示例：发送一些测试消息
                await this.sendTestMessages();

                // 示例：发送音频数据（模拟）
                await this.sendTestAudio();

            } else {
                console.error('❌ 连接失败');
            }

        } catch (error) {
            console.error('❌ 启动失败:', error);
        }
    }

    /**
     * 发送测试消息
     */
    async sendTestMessages() {
        console.log('📤 发送测试消息...');

        // 发送测试 JSON 消息
        const testMessage = {
            type: 'test',
            message: 'Hello from JavaScript client!',
            timestamp: Date.now()
        };

        await this.protocol.sendText(JSON.stringify(testMessage));
        console.log('📤 已发送测试 JSON 消息');
    }

    /**
     * 发送测试音频数据
     */
    async sendTestAudio() {
        console.log('🎵 发送测试音频数据...');

        // 发送更简单的测试数据，而不是生成完整音频
        // 实际项目中应该发送 Opus 编码的音频数据
        const testAudioData = Buffer.from([0x01, 0x02, 0x03, 0x04]); // 简单的测试数据

        console.log('🔧 使用简化的测试音频数据，避免格式不匹配');

        const success = await this.protocol.sendAudio(testAudioData);
        if (success) {
            console.log('🎵 已发送测试音频数据');
        } else {
            console.error('❌ 发送音频数据失败');
        }
    }

    /**
     * 停止示例
     */
    async stop() {
        console.log('🛑 停止 WebSocket 连接...');
        await this.protocol.closeAudioChannel();
        this.protocol.destroy();
        console.log('✅ 已停止');
    }
}



if (result) {
    let statusResponse = await activator.checkDeviceStatus();
    console.log('statusResponse', statusResponse);

    const example = new WebSocketExample(
        statusResponse.websocket.url,
        statusResponse.websocket.token,
        deviceId,
        statusResponse.mqtt.client_id);

    example.start().catch(console.error);

    setTimeout(async() => {
        console.log('10秒后自动停止测试...');
        await example.stop();
    }, 10000);
}