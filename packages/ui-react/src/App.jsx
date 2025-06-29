import React, { useState, useEffect, useRef } from 'react';
import { Client } from '@xiaozhi/core/src/client.js';
import { PlatformFactory } from '@xiaozhi/core/src/platform-factory.js';
import './App.css';

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

function App() {
    // 状态管理
    const [status, setStatus] = useState('空闲');
    const [error, setError] = useState(null);
    const [isInitialized, setIsInitialized] = useState(false);
    const [platformInfo, setPlatformInfo] = useState(null);
    
    // 客户端引用
    const clientRef = useRef(null);
    
    // 浏览器环境配置
    const browserConfig = {
        websocketUrl: 'ws://localhost:3000',
        accessToken: 'test-token-browser',
        deviceId: `browser-device-${Date.now()}`,
        clientId: `browser-client-${Date.now()}`,
        useOpus: true,
    };

    // 错误处理函数
    const showError = (title, message) => {
        setError({ title, message });
    };

    const clearError = () => {
        setError(null);
    };

    // 初始化客户端
    useEffect(() => {
        const initClient = async () => {
            try {
                console.log('--- 浏览器环境示例 ---');
                
                // 检查浏览器支持
                const platformInfo = PlatformFactory.getPlatformInfo();
                setPlatformInfo(platformInfo);

                if (!platformInfo.support) {
                    const reason = '浏览器不支持 Web Audio 或相关 API。';
                    console.error('❌', reason);
                    showError('浏览器不支持', `${reason}请使用现代浏览器并确保允许麦克风权限。`);
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
                clientRef.current = client;

                // 设置状态变化监听器
                client.onStateChanged = (newState) => {
                    console.log(`📊 状态变化: ${newState}`);
                    setStatus(newState);
                };

                setIsInitialized(true);
                console.log('✅ 浏览器客户端初始化成功');

            } catch (error) {
                console.error('❌ 浏览器示例失败:', error);
                showError('初始化失败', error.message);
            }
        };

        initClient();

        // 清理函数
        return () => {
            if (clientRef.current) {
                // 这里可以添加客户端清理逻辑
            }
        };
    }, []);

    // 开始语音聊天
    const handleStartVoiceChat = async () => {
        if (!clientRef.current) {
            showError('客户端错误', '客户端未初始化');
            return;
        }

        try {
            clearError();
            await clientRef.current.startVoiceChat();
        } catch (error) {
            console.error(`❌ 启动失败: ${error.message}`);
            showError('启动失败', error.message);
        }
    };

    // 打断对话
    const handleInterruptConversation = async () => {
        if (!clientRef.current) {
            showError('客户端错误', '客户端未初始化');
            return;
        }

        try {
            clearError();
            await clientRef.current.interruptConversation();
        } catch (error) {
            console.error(`❌ 打断失败: ${error.message}`);
            showError('打断失败', error.message);
        }
    };

    return (
        <div className="app-container">
            <h1>小智语音助手</h1>
            
            {/* 错误信息显示 */}
            {error && (
                <div className="error-container">
                    <h3>{error.title}</h3>
                    <p>{error.message}</p>
                    <button 
                        onClick={clearError}
                        style={{ 
                            background: 'transparent', 
                            border: '1px solid #d93025', 
                            color: '#d93025',
                            padding: '4px 8px',
                            fontSize: '12px'
                        }}
                    >
                        关闭
                    </button>
                </div>
            )}

            {/* 状态显示 */}
            <div className="status-bar">
                <span className="status-text">状态: {status}</span>
            </div>

            {/* 控制按钮 */}
            <div className="controls">
                <button 
                    onClick={handleStartVoiceChat}
                    disabled={!isInitialized}
                    style={{ backgroundColor: '#007bff' }}
                >
                    开始语音聊天
                </button>
                
                <button 
                    onClick={handleInterruptConversation}
                    disabled={!isInitialized}
                    style={{ backgroundColor: '#ffc107', color: 'black' }}
                >
                    打断对话
                </button>
            </div>

            {/* 平台信息（隐藏，仅用于调试） */}
            {platformInfo && (
                <div style={{ display: 'none' }}>
                    <h3>平台信息:</h3>
                    <pre>{JSON.stringify(platformInfo, null, 2)}</pre>
                </div>
            )}
        </div>
    );
}

export default App; 