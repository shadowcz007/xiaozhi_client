import { checkDeviceStatus } from './src/device-status.js';
import { Client } from './src/client.js';

let deviceId = 'df:52:34:be:fa:38';
async function main() {

    let statusResponse = await checkDeviceStatus(deviceId);
    console.log('statusResponse', statusResponse);
    if (!statusResponse) {
        //需要激活
        console.error('需要激活');
        return
    }

    const example = new Client(
        statusResponse.websocket.url,
        statusResponse.websocket.token,
        deviceId,
        statusResponse.mqtt.client_id);

    // 监听状态变化
    example.onStateChanged = (state) => {
        console.log('当前状态:', state);
        // 根据状态更新UI
    };

    // 直接发送文字消息
    // await example.sendTextMessage('今天天气怎么样？');

    // 开始语音聊天（会发送hi并开启自动录音循环）
    await example.startVoiceChat();

    // // 打断对话
    // await example.interruptConversation();

    // // 停止语音聊天
    // await example.stopVoiceChat();
}

main();