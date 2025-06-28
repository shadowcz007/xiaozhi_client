import { Client, PlatformFactory, checkDeviceStatus } from '@xiaozhi/core';

// Opus 配置
const opusConfig = {
    audioRecorderOptions: {
        useOpus: true, // 启用 Opus 录音器
        sampleRate: 16000,
        numberOfChannels: 1,
        frameSize: 320, // 20ms at 16kHz
        bitRate: 32000, // 32kbps
    },
    audioPlayerOptions: {
        useOpus: true, // 启用 Opus 播放器
        sampleRate: 16000,
        numberOfChannels: 1,
    }
};

/**
 * Node.js 环境客户端
 */
async function nodeClient(name, deviceId) {
    console.log('🖥️  Starting Node.js client...');

    try {
        const platformInfo = PlatformFactory.getPlatformInfo();
        console.log('ℹ️ Platform:', platformInfo);

        if (!platformInfo.support) {
            console.error('❌ Platform not supported:', platformInfo.reason);
            return;
        }

        console.log('ℹ️ Checking device status...');
        const statusResponse = await checkDeviceStatus(name, deviceId);
        if (!statusResponse) {
            console.error('❌ Device needs activation. Please run `npm run manage:devices`.');
            return;
        }
        console.log('✅ Device status is OK.');

        const client = new Client(
            statusResponse.websocket.url,
            statusResponse.websocket.token,
            deviceId,
            statusResponse.mqtt.client_id,
            opusConfig
        );
        await client.init();

        client.onStateChanged = (newState) => {
            console.log(`📊 State changed: ${newState}`);
        };

        await client.startVoiceChat();

        console.log('✅ Node.js client started successfully.');

    } catch (error) {
        console.error('❌ Node.js client failed:', error);
    }
}

// --- 启动 ---
const deviceId = '9b:9b:f3:50:dc:17';
const name = 'goodmate';
nodeClient(name, deviceId);