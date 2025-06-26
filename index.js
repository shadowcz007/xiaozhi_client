import { DeviceActivator } from './src/device-activator.js';
import { Client } from './src/client.js';

const activator = new DeviceActivator();
let deviceId = 'f2:68:cb:b2:c5:93';


let statusResponse = await activator.checkDeviceStatus(deviceId);
console.log('statusResponse', statusResponse);

const example = new Client(
    statusResponse.websocket.url,
    statusResponse.websocket.token,
    deviceId,
    statusResponse.mqtt.client_id);

// 检查命令行参数，确定是否启用麦克风录音
const enableMicrophone = true;

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