import { DeviceActivator } from './device-activator.js';
import { WebSocketProtocol } from './websocket.js';
import { MicrophoneOpusRecorder } from './voice.js';


const activator = new DeviceActivator();
let deviceId = 'e7:40:56:7a:13:9f';
let result = await activator.start(deviceId, false);
console.log('result', result);


/**
 * WebSocket 协议使用示例 - 增强版，支持麦克风录音
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

        // 麦克风录音器
        this.micRecorder = null;
        this.isRecordingFromMic = false;
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

            // 处理语音识别结果
            if (jsonData.type === 'stt') {
                console.log('🎤 语音识别结果:', jsonData.text || jsonData.message);
            }

            // 处理语音合成消息
            if (jsonData.type === 'tts') {
                console.log('🔊 收到语音合成消息');
                if (jsonData.text) {
                    console.log('🗣️ 合成内容:', jsonData.text);
                }
            }

            // 处理会话相关消息
            if (jsonData.type === 'session') {
                console.log('📋 会话消息:', jsonData);
            }

            // 在这里处理接收到的其他 JSON 消息
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
     * 启动麦克风录音
     */
    async startMicrophoneRecording() {
        try {
            if (this.isRecordingFromMic) {
                console.log('🎤 麦克风录音已在进行中');
                return;
            }

            console.log('🎤 初始化麦克风录音器...');

            this.micRecorder = new MicrophoneOpusRecorder({
                sampleRate: 16000,
                channels: 1,
                frameSize: 160
            });

            // 设置 Opus 数据回调
            this.micRecorder.onOpusData = (opusData) => {
                // 实时发送 Opus 数据到服务器
                if (this.protocol.isAudioChannelOpened()) {
                    this.protocol.sendAudio(opusData);
                    console.log('🎵 发送音频帧:', opusData.length, '字节');
                }
            };

            // 设置错误回调
            this.micRecorder.onError = (error) => {
                console.error('❌ 麦克风录音错误:', error);
                this.stopMicrophoneRecording();
            };

            // 开始录音
            this.micRecorder.startRecording();
            this.isRecordingFromMic = true;

            console.log('🎤 麦克风录音已启动，开始实时语音识别...');
            console.log('💡 请对着麦克风说话，语音将被实时识别');

        } catch (error) {
            console.error('❌ 启动麦克风录音失败:', error);
            console.log('💡 请确保已安装必要的依赖：npm install @discordjs/opus node-mic');
            console.log('💡 node-mic 会自动下载并安装所需的音频工具');
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
        } else {
            console.log('🔇 麦克风录音未在进行中');
        }
    }

    /**
     * 启动示例
     */
    async start(enableMicrophone = false) {
        try {
            console.log('🚀 开始连接 WebSocket 服务器...');

            const success = await this.protocol.connect();

            if (success) {
                console.log('✅ 连接建立成功！');

                // 发送测试消息
                await this.sendTestMessages();

                if (enableMicrophone) {
                    // 等待连接稳定后启动麦克风
                    console.log('🎤 准备启动麦克风录音模式...');
                    setTimeout(() => {
                        this.startMicrophoneRecording();
                    }, 1000);
                } else {
                    // 发送测试音频数据
                    await this.sendTestAudio();
                }

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
     * 停止麦克风录音（保持 WebSocket 连接）
     */
    async stop() {
        console.log('🛑 停止麦克风录音...');

        // 只停止麦克风录音，保持 WebSocket 连接
        this.stopMicrophoneRecording();
        console.log('✅ 已停止麦克风录音，WebSocket 连接保持开启');

        // 停止麦克风后，发送测试音频数据查看服务器响应
        console.log('🎵 发送测试音频数据以查看服务器响应...');
        await this.sendTestAudio();
    }

    /**
     * 完全断开连接
     */
    async disconnect() {
        console.log('🛑 断开 WebSocket 连接...');

        // 先停止麦克风录音
        this.stopMicrophoneRecording();

        // 然后关闭 WebSocket 连接
        await this.protocol.closeAudioChannel();
        this.protocol.destroy();
        console.log('✅ 已完全断开连接');
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

    // 检查命令行参数，确定是否启用麦克风录音
    const enableMicrophone = process.argv.includes('--mic') || process.argv.includes('--microphone');

    if (enableMicrophone) {
        console.log('🎤 启用麦克风录音模式');
        console.log('💡 使用方法：对着麦克风说话，语音将被实时识别');
        console.log('💡 测试将在30秒后自动停止');

        example.start(true).catch(console.error);

        // 30秒后自动停止（给足够时间测试语音）
        setTimeout(async() => {
            console.log('\n⏰ 3秒测试时间结束，自动停止...');
            await example.stop();
        }, 3000);

    } else {
        console.log('🧪 使用测试数据模式');
        console.log('💡 如需测试麦克风录音，请运行：node test.js --mic');

        example.start(false).catch(console.error);

        // 10秒后自动停止
        setTimeout(async() => {
            console.log('\n⏰ 10秒后自动停止测试...');
            await example.disconnect();
        }, 10000);
    }
}