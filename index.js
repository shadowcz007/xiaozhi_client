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

// 监听状态变化
example.onStateChanged = (state) => {
    console.log('当前状态:', state);
    // 根据状态更新UI
};

// 开始语音聊天（会发送hi并开启自动录音循环）
await example.startVoiceChat();

// 直接发送文字消息
await example.sendTextMessage('今天天气怎么样？');

// 打断对话
await example.interruptConversation();

// 停止语音聊天
await example.stopVoiceChat();