/**
 * 小智客户端使用示例
 * 展示如何在 Node.js 和浏览器环境下使用统一的 API
 */

import { Client } from './client.js';
import { PlatformFactory, PlatformDetector } from './platform-factory.js';
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

// PCM 配置 (用于浏览器，服务器端编码)
const pcmConfig = {
    audioRecorderOptions: {
        useOpus: false, // 禁用 Opus 录音器, 使用原始 PCM
        sampleRate: 16000,
        channels: 1,
        bufferSize: 4096 // 适用于 ScriptProcessorNode 的缓冲区大小
    },
    audioPlayerOptions: {
        useOpus: true, // 播放仍然使用 Opus，因为服务器会返回 Opus 
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

/**
 * 浏览器环境示例
 */
async function browserClient() {
    console.log('--- 浏览器环境示例 ---');

    // 浏览器环境配置
    const browserConfig = {
        // 使用本地代理服务器的地址
        websocketUrl: 'ws://localhost:3000',
        accessToken: 'test-token-browser',
        deviceId: `browser-device-${Date.now()}`,
        clientId: `browser-client-${Date.now()}`,
        useOpus: true, // 在浏览器中启用 Opus
    };

    const errorContainer = document.getElementById('error-container');
    const showError = (title, message) => {
        if (errorContainer) {
            errorContainer.style.display = 'block';
            errorContainer.innerHTML = `<h3>${title}</h3><p>${message}</p>`;
        }
    };

    try {
        // 检查浏览器支持
        const platformInfo = PlatformFactory.getPlatformInfo();
        const platformInfoEl = document.getElementById('platformInfo');
        if (platformInfoEl) {
            platformInfoEl.textContent = JSON.stringify(platformInfo, null, 2);
        }

        if (!platformInfo.support) {
            const reason = '浏览器不支持 Web Audio 或相关 API。';
            console.error('❌', reason);
            showError('浏览器不支持', `${reason}<br>请使用现代浏览器并确保允许麦克风权限。`);
            return;
        }

        // 创建客户端实例
        const client = new Client(
            browserConfig.websocketUrl,
            browserConfig.accessToken,
            browserConfig.deviceId,
            browserConfig.clientId,
            pcmConfig
        );
        await client.init();

        // --- UI 交互逻辑 ---

        const logsEl = document.getElementById('logs');
        const addLog = (message) => {
            if (logsEl) {
                const timestamp = new Date().toLocaleTimeString();
                logsEl.innerHTML += `<div>[${timestamp}] ${message}</div>`;
                logsEl.scrollTop = logsEl.scrollHeight;
            }
        };

        const originalLog = console.log;
        console.log = function(...args) {
            originalLog.apply(console, args);
            addLog(args.join(' '));
        };

        client.onStateChanged = (newState) => {
            console.log(`📊 状态变化: ${newState}`);
            const statusElement = document.getElementById('status');
            if (statusElement) {
                statusElement.textContent = `状态: ${newState}`;
            }
        };

        const startBtn = document.getElementById('startBtn');
        const stopBtn = document.getElementById('stopBtn');
        const interruptBtn = document.getElementById('interruptBtn');

        if (startBtn) {
            startBtn.onclick = async() => {
                try {
                    await client.startVoiceChat();
                    startBtn.disabled = true;
                    stopBtn.disabled = false;
                    interruptBtn.disabled = false;
                } catch (error) {
                    console.error(`❌ 启动失败: ${error.message}`);
                    showError('启动失败', error.message);
                }
            };
        }

        if (stopBtn) {
            stopBtn.onclick = async() => {
                try {
                    await client.stopVoiceChat();
                    startBtn.disabled = false;
                    stopBtn.disabled = true;
                    interruptBtn.disabled = true;
                } catch (error) {
                    console.error(`❌ 停止失败: ${error.message}`);
                    showError('停止失败', error.message);
                }
            };
        }

        if (interruptBtn) {
            interruptBtn.onclick = async() => {
                try {
                    await client.interruptConversation();
                } catch (error) {
                    console.error(`❌ 打断失败: ${error.message}`);
                    showError('打断失败', error.message);
                }
            };
        }

        console.log('✅ 浏览器客户端初始化成功');

    } catch (error) {
        console.error('❌ 浏览器示例失败:', error);
        showError('初始化失败', error.message);
    }
}

// 导出示例函数
export {
    nodeClient,
    browserClient
};