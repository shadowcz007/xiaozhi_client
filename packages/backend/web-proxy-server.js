// https-server.js
import { checkDeviceStatus } from '@xiaozhi/core';

import http from 'http';
import { WebSocketServer } from 'ws';

import pkg from 'audify';
const { OpusEncoder, OpusApplication } = pkg;

// 导入我们已经验证可用的 Node.js WebSocket 客户端
import { NodeWebSocketProtocol } from '../core/src/adapters/node/node-websocket.js';

const PORT = 3000;

// 创建一个 HTTP 服务器, 主要用于 WebSocket 的升级(upgrade)请求
const server = http.createServer((req, res) => {
    // 我们的服务器现在只处理 WebSocket, 对于普通的 HTTP 请求可以直接响应成功或提示信息
    res.writeHead(200, { 'Content-Type': 'text/plain' });
    res.end('WebSocket Proxy Server is running. Please connect via WebSocket.');
});

// 创建 WebSocket 服务器并将其附加到 HTTP 服务器
const wss = new WebSocketServer({ server });

wss.on('connection', async(browserWs, req) => {
    console.log('✅ 浏览器客户端已连接');

    // 为每个连接创建一个独立的Opus编码器实例
    // 假设从浏览器传来的是 16000Hz, 单声道的 PCM 数据
    let opusEncoder = new OpusEncoder(16000, 1, OpusApplication.OPUS_APPLICATION_AUDIO);

    let pcmBuffer = Buffer.alloc(0); // 为每个连接创建一个PCM数据缓冲区


    try {
        // --- 1. 服务器端认证 ---
        // 代理服务器负责处理与后端的认证，使用固定的设备ID
        const name = 'goodmate';
        const deviceId = '9b:9b:f3:50:dc:17';

        console.log('ℹ️ 正在检查设备状态...');
        const statusResponse = await checkDeviceStatus(name, deviceId);
        if (!statusResponse) {
            console.error('❌ 设备需要激活，关闭连接');
            browserWs.close(1008, 'Device not activated');
            return;
        }
        console.log('✅ 设备状态正常');

        // --- 2. 建立到后端服务的连接 ---
        console.log('🔗 正在连接到后端服务...');
        const xiaozhiClient = new NodeWebSocketProtocol({
            accessToken: statusResponse.websocket.token,
            deviceId: deviceId,
            clientId: statusResponse.mqtt.client_id,
            websocketUrl: statusResponse.websocket.url
        });

        // --- 3. 设置消息转发规则 ---

        // 规则 A: 后端 -> 浏览器
        // 当代理与后端握手成功（收到 hello 响应），将该响应转发给浏览器，完成浏览器的握手。
        xiaozhiClient.on('audioChannelOpened', (helloData) => {
            console.log('🔊 后端音频通道已打开，转发 hello 响应给浏览器');
            if (browserWs.readyState === browserWs.OPEN) {
                browserWs.send(JSON.stringify(helloData));
            }
        });

        // 转发后端发来的其他JSON消息
        xiaozhiClient.on('incomingJson', (json) => {
            if (browserWs.readyState === browserWs.OPEN) {
                browserWs.send(JSON.stringify(json));
                if (json.type == 'tts') {
                    console.log('🎧 收到 TTS 消息:', json);

                }
            }
        });

        // 转发后端发来的音频数据
        xiaozhiClient.on('incomingAudio', (audioData) => {
            // console.log(xiaozhiClient);
            // console.log('接收到音频数据:', sample_rate, browserWs.readyState === browserWs.OPEN, audioData.length);
            if (browserWs.readyState === browserWs.OPEN) {
                if (audioData && audioData.length > 0) {
                    browserWs.send(audioData);
                } else {
                    console.log('🤔 收到空的音频数据包，跳过转发。');
                }
            }
        });

        // 如果后端连接关闭，则也关闭浏览器连接
        xiaozhiClient.on('audioChannelClosed', () => {
            if (browserWs.readyState === browserWs.OPEN) {
                console.log('🔌 后端连接已关闭，正在关闭浏览器连接...');
                browserWs.close();
            }
        });

        // 规则 B: 浏览器 -> 后端
        browserWs.on('message', (message, isBinary) => {
            // 转发所有消息到后端
            if (xiaozhiClient.isConnected() && xiaozhiClient.websocket) {
                if (isBinary) {
                    // 1. 将收到的 PCM 数据块附加到缓冲区
                    pcmBuffer = Buffer.concat([pcmBuffer, message]);

                    // 2. 定义 Opus 编码器期望的帧大小
                    // 对于 16kHz 采样率, 20ms 帧 = 320 个采样点
                    const OPUS_FRAME_SAMPLES = 320;
                    // 对于 16-bit PCM (每个采样点2字节), 帧的字节大小为:
                    const OPUS_FRAME_BYTES = OPUS_FRAME_SAMPLES * 2;

                    // 3. 循环处理缓冲区中所有完整的帧
                    while (pcmBuffer.length >= OPUS_FRAME_BYTES) {
                        try {
                            // 从缓冲区切出一个完整的帧
                            const pcmFrame = pcmBuffer.subarray(0, OPUS_FRAME_BYTES);

                            // 编码这一帧
                            const opusData = opusEncoder.encode(pcmFrame, OPUS_FRAME_SAMPLES);
                            // console.log(`📤 转发 PCM 数据 (大小: ${pcmFrame.length} 字节) -> 编码为 Opus (大小: ${opusData.length} 字节)`);
                            xiaozhiClient.websocket.send(opusData);

                            // 从缓冲区移除已处理的数据
                            pcmBuffer = pcmBuffer.subarray(OPUS_FRAME_BYTES);

                        } catch (e) {
                            console.error('❌ Opus 编码失败:', e);
                            // 如果发生错误，跳出循环以避免无限错误
                            break;
                        }
                    }
                } else {
                    // 如果是文本消息，进行JSON解析和处理
                    const messageStr = message.toString();
                    try {
                        const parsed = JSON.parse(messageStr);

                        // 依然要忽略来自浏览器的 hello 消息
                        if (parsed.type === 'hello') {
                            console.log('👋 收到并忽略来自浏览器的 hello 消息。');
                            return;
                        }

                        console.log('📤 转发文本消息到后端:', JSON.stringify(parsed, null, 2));
                        xiaozhiClient.websocket.send(messageStr);
                    } catch (e) {
                        console.error('❌ 解析文本消息失败，可能不是有效的JSON:', messageStr, e);
                    }
                }
            }
        });

        // --- 4. 设置清理逻辑 ---
        browserWs.on('close', () => {
            console.log('🔌 浏览器客户端已断开，销毁到后端服务的连接');
            xiaozhiClient.destroy();
            // 清理编码器和缓冲区资源
            opusEncoder = null;
            pcmBuffer = null;
        });

        browserWs.on('error', (error) => {
            console.error('浏览器 WebSocket 错误:', error);
            xiaozhiClient.destroy();
            // 清理编码器和缓冲区资源
            opusEncoder = null;
            pcmBuffer = null;
        });

        // --- 5. 启动连接 ---
        // 连接到后端，这会触发第一个 'hello' 消息的发送
        await xiaozhiClient.connect();
        console.log('✅ 后端服务连接流程已启动');

    } catch (error) {
        console.error('处理新连接时出错:', error.message);
        browserWs.close(1011, 'Internal server error');
    }
});

server.listen(PORT, () => {
    console.log(`请在浏览器中打开 http://localhost:${PORT}`);
});