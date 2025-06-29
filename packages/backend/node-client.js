import {
    Client,
    checkDeviceStatus,
    KeyboardAwakener,
    HttpAwakener
} from '@xiaozhi/core';

// Opus 配置
const opusConfig = {
    audioRecorderOptions: {
        useOpus: true,
        sampleRate: 16000,
        numberOfChannels: 1,
        frameSize: 320, // 20ms at 16kHz
        bitRate: 32000,
    },
    audioPlayerOptions: {
        useOpus: true,
        sampleRate: 16000,
        numberOfChannels: 1,
    }
};

/**
 * Node.js 环境客户端主程序
 */
async function main() {
    console.log('🖥️  启动小智 Node.js 客户端...');

    // --- 配置 ---
    // 建议从环境变量或配置文件中读取
    const deviceId = process.env.DEVICE_ID || '9b:9b:f3:50:dc:17';
    const deviceName = process.env.DEVICE_NAME || 'goodmate';
    const httpPort = 8088;

    if (deviceId === 'your_device_id_here' || deviceName === 'your_device_name_here') {
        console.warn('⚠️  警告: 请设置您的 deviceId 和 deviceName。');
        // 在实际使用中，这里可能需要退出程序
        return;
    }

    try {
        console.log('ℹ️ 正在检查设备状态...');
        const statusResponse = await checkDeviceStatus(deviceName, deviceId);
        if (!statusResponse) {
            console.error('❌ 设备需要激活。请运行 `npm run manage:devices`。');
            return;
        }
        console.log('✅ 设备状态正常。');

        const client = new Client(
            statusResponse.websocket.url,
            statusResponse.websocket.token,
            deviceId,
            statusResponse.mqtt.client_id,
            opusConfig
        );
        await client.init();

        client.onStateChanged = (newState) => {
            console.log(`📊 状态变更: ${newState}`);
        };

        // --- 初始化并启动唤醒器 ---
        const keyboardAwakener = new KeyboardAwakener();
        const httpAwakener = new HttpAwakener(httpPort);

        // 将唤醒器附加到客户端
        keyboardAwakener.attach(client);
        httpAwakener.attach(client);

        // 启动唤醒器
        keyboardAwakener.start();
        httpAwakener.start();

        console.log('\n✅ 客户端已准备就绪，等待唤醒指令...');

        // --- 设置优雅退出 ---
        const cleanup = async() => {
            console.log('\n🛑 正在停止客户端和唤醒器...');
            keyboardAwakener.stop();
            httpAwakener.stop();
            await client.disconnect();
            console.log('👋 已清理资源，安全退出。');
            process.exit(0);
        };

        process.on('SIGINT', cleanup); // 捕获 Ctrl+C
        process.on('SIGTERM', cleanup); // 捕获 kill 命令

    } catch (error) {
        console.error('❌ 客户端主程序运行失败:', error);
        process.exit(1);
    }
}

// --- 启动 ---
main();