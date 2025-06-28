/**
 * 小智客户端使用示例
 * 展示如何在 Node.js 和浏览器环境下使用统一的 API
 */

import { Client } from './client.js';
import { PlatformFactory } from './platform-factory.js';
import { checkDeviceStatus } from './device-status.js';

// Opus 配置
const opusConfig = {
    audioRecorderOptions: {
        useOpus: true, // 启用 Opus 录音器
        encoderPath: '/node_modules/opus-recorder/dist/encoderWorker.min.js', // Worker 文件路径
        sampleRate: 16000,
        numberOfChannels: 1,
        encoderBitRate: 32000, // 32kbps
        encoderComplexity: 8, // 0-10, 8 is a good balance
        recordingGain: 1.0
    },
    audioPlayerOptions: {
        useOpus: true, // 启用 Opus 播放器
        decoderPath: '/node_modules/opus-decoder/dist/opus-decoder.min.js',
        sampleRate: 16000,
        numberOfChannels: 1,
        volume: 1.0
    }
};

/**
 * Node.js 环境示例
 */
async function nodeClient(name, deviceId) {
    console.log('🖥️ Node.js 环境示例');

    try {
        // 检查平台支持
        const platformInfo = PlatformFactory.getPlatformInfo();
        console.log('平台信息:', platformInfo);

        if (!platformInfo.support) {
            console.error('❌ 当前平台不支持:', platformInfo.reason);
            return;
        }

        // 检查设备状态

        const statusResponse = await checkDeviceStatus(name, deviceId);
        if (!statusResponse) {
            console.error('❌ 设备需要激活');
            return;
        }

        // 创建客户端
        const client = new Client(
            statusResponse.websocket.url,
            statusResponse.websocket.token,
            deviceId,
            statusResponse.mqtt.client_id,
            opusConfig
        );
        await client.init();

        // 设置状态变化回调
        client.onStateChanged = (newState) => {
            console.log(`📊 状态变化: ${newState}`);
        };

        // 开始语音聊天
        await client.startVoiceChat();

        // 等待用户交互或其他事件...
        console.log('✅ Node.js 客户端启动成功');

        // 模拟运行一段时间后断开连接
        // await client.disconnect();

    } catch (error) {
        console.error('❌ Node.js 示例失败:', error);
    }
}

// 导出示例函数
export {
    nodeClient
};