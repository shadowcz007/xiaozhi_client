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
        // 保存录制的音频数据
        this.recordedAudioBuffers = [];
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
            // 流式播放音频             
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

                // 分析错误类型
                if (jsonData.message === 'Error occurred while processing message') {
                    console.log('💡 这是服务器处理音频数据时的错误');
                    console.log('💡 可能的原因:');
                    console.log('   - 音频格式不正确（需要Opus编码）');
                    console.log('   - 音频数据损坏或不完整');
                    console.log('   - 服务器音频处理组件异常');
                } else {
                    console.log('💡 提示：这可能是服务器处理时的错误，不影响基本连接功能');
                }
            }

            // 处理语音识别结果
            if (jsonData.type === 'stt') {
                console.log('🎤 用户的语音识别结果:', jsonData.text || jsonData.message);
            }

            // 处理语音合成消息
            if (jsonData.type === 'tts') {
                console.log('🔊 收到xiaozhi的语音合成消息');
                if (jsonData.text) {
                    console.log('🗣️ 合成内容:', jsonData.text);
                }
            }

            if (jsonData.type === 'llm') {
                console.log('🎤 llm结果:', jsonData.text, jsonData.emotion);
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
                frameSize: 320 // 20ms @ 16kHz，与WebSocket协议的frame_duration保持一致
            });

            // 设置 Opus 数据回调
            this.micRecorder.onOpusData = (opusData) => {
                // 保存录制的音频数据
                this.recordedAudioBuffers.push(Buffer.from(opusData));

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
            console.log(`📊 总共录制了 ${this.recordedAudioBuffers.length} 个音频帧`);
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

        // 停止麦克风后，发送录制的音频数据查看服务器响应
        await this.sendRecordedAudio();
    }

    /**
     * 发送录制的音频数据
     */
    async sendRecordedAudio() {
        if (this.recordedAudioBuffers.length === 0) {
            console.log('⚠️ 没有录制到音频数据');
            return;
        }

        console.log(`🎵 发送录制的音频数据，共 ${this.recordedAudioBuffers.length} 个音频帧...`);

        // 发送开始监听消息，让服务端准备接收音频进行语音识别
        const startListeningMessage = {
            type: 'listen',
            state: 'start',
            mode: 'manual' // 手动模式
        };
        await this.protocol.sendText(JSON.stringify(startListeningMessage));
        console.log('📤 已发送开始监听消息');

        // 等待服务器准备好接收音频
        await new Promise(resolve => setTimeout(resolve, 500));

        // 逐帧发送录制的音频数据
        console.log('📡 逐帧发送音频数据...');
        let successCount = 0;

        for (let i = 0; i < this.recordedAudioBuffers.length; i++) {
            const audioFrame = this.recordedAudioBuffers[i];

            // 每10帧打印一次进度，避免输出过多
            if (i % 10 === 0 || i === this.recordedAudioBuffers.length - 1) {
                console.log(`🎵 发送进度: ${i + 1}/${this.recordedAudioBuffers.length} 帧，当前帧大小: ${audioFrame.length} 字节`);
            }

            const success = await this.protocol.sendAudio(audioFrame);
            if (success) {
                successCount++;
            } else {
                console.error(`❌ 发送第 ${i + 1} 帧失败`);
                // 继续发送其他帧，不要因为一个失败就停止
            }

            // 根据音频帧时长添加对应的延迟 (20ms 帧)
            await new Promise(resolve => setTimeout(resolve, 20));
        }

        console.log(`📊 发送完成: 成功 ${successCount}/${this.recordedAudioBuffers.length} 帧`);

        // 发送停止监听消息
        const stopListeningMessage = {
            type: 'listen',
            state: 'stop'
        };
        await this.protocol.sendText(JSON.stringify(stopListeningMessage));
        console.log('📤 已发送停止监听消息');

        console.log('✅ 所有音频帧发送完成，等待服务器响应...');

        // 清空录制的音频缓存
        this.recordedAudioBuffers = [];
        console.log('🗑️ 已清空音频缓存');
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