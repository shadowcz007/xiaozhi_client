# 小智 WebSocket 通信协议 - 前端 JavaScript API 文档

## 概述

小智AI语音助手使用WebSocket协议进行实时双向通信，支持文本消息和二进制音频数据传输。本文档提供了完整的前端JavaScript API实现。

## 基础配置

```javascript
// 音频配置常量
const AudioConfig = {
    INPUT_SAMPLE_RATE: 16000,    // 输入采样率 16kHz
    OUTPUT_SAMPLE_RATE: 24000,   // 输出采样率 24kHz (官方服务器) 或 16kHz
    CHANNELS: 1,                 // 单声道
    FRAME_DURATION: 60,          // 帧长度 60ms
    FORMAT: 'opus'               // 音频格式 Opus
};

// 设备状态枚举
const DeviceState = {
    IDLE: 'idle',               // 空闲
    CONNECTING: 'connecting',   // 连接中
    LISTENING: 'listening',     // 监听中
    SPEAKING: 'speaking'        // 说话中
};

// 监听模式枚举
const ListeningMode = {
    ALWAYS_ON: 'realtime',      // 实时模式
    AUTO_STOP: 'auto',          // 自动停止
    MANUAL: 'manual'            // 手动模式
};

// 中止原因枚举
const AbortReason = {
    WAKE_WORD_DETECTED: 'wake_word_detected',
    USER_INTERRUPTION: 'user_interruption'
};
```

## WebSocket 客户端类

```javascript
class XiaozhiWebSocketClient {
    constructor(config) {
        this.websocketUrl = config.websocketUrl;
        this.deviceId = config.deviceId;
        this.clientId = config.clientId || config.deviceId;
        this.accessToken = config.accessToken;
        
        this.websocket = null;
        this.connected = false;
        this.sessionId = '';
        this.helloReceived = false;
        
        // 事件回调
        this.onConnected = null;
        this.onDisconnected = null;
        this.onError = null;
        this.onTextMessage = null;
        this.onAudioData = null;
        this.onTTSMessage = null;
        this.onSTTMessage = null;
        this.onLLMMessage = null;
        this.onIOTMessage = null;
    }

    /**
     * 连接到WebSocket服务器
     */
    async connect() {
        try {
            // 创建WebSocket连接
            this.websocket = new WebSocket(this.websocketUrl);

            // 设置事件监听器
            this.websocket.onopen = this._onOpen.bind(this);
            this.websocket.onmessage = this._onMessage.bind(this);
            this.websocket.onclose = this._onClose.bind(this);
            this.websocket.onerror = this._onError.bind(this);

            return new Promise((resolve, reject) => {
                const timeout = setTimeout(() => {
                    reject(new Error('连接超时'));
                }, 10000);

                this.websocket.onopen = () => {
                    clearTimeout(timeout);
                    this._onOpen();
                    resolve();
                };

                this.websocket.onerror = (error) => {
                    clearTimeout(timeout);
                    reject(error);
                };
            });

        } catch (error) {
            console.error('WebSocket连接失败:', error);
            if (this.onError) this.onError(error.message);
            throw error;
        }
    }

    /**
     * 断开连接
     */
    disconnect() {
        if (this.websocket) {
            this.websocket.close();
            this.websocket = null;
            this.connected = false;
            this.helloReceived = false;
        }
    }

    /**
     * 发送Hello消息
     */
    sendHello() {
        const helloMessage = {
            type: 'hello',
            version: 1,
            transport: 'websocket',
            audio_params: {
                format: AudioConfig.FORMAT,
                sample_rate: AudioConfig.INPUT_SAMPLE_RATE,
                channels: AudioConfig.CHANNELS,
                frame_duration: AudioConfig.FRAME_DURATION
            }
        };
        
        this.sendTextMessage(JSON.stringify(helloMessage));
    }

    /**
     * 开始监听
     * @param {string} mode - 监听模式 ('realtime', 'auto', 'manual')
     */
    startListening(mode = ListeningMode.AUTO_STOP) {
        const message = {
            session_id: this.sessionId,
            type: 'listen',
            state: 'start',
            mode: mode
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 停止监听
     */
    stopListening() {
        const message = {
            session_id: this.sessionId,
            type: 'listen',
            state: 'stop'
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 发送唤醒词检测消息
     * @param {string} wakeWord - 检测到的唤醒词
     */
    sendWakeWordDetected(wakeWord) {
        const message = {
            session_id: this.sessionId,
            type: 'listen',
            state: 'detect',
            text: wakeWord
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 发送文本消息（用于文本对话）
     * @param {string} text - 文本内容
     */
    sendTextQuery(text) {
        const message = {
            session_id: this.sessionId,
            type: 'listen',
            state: 'detect',
            text: text,
            source: 'text'
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 发送中止消息
     * @param {string} reason - 中止原因
     */
    sendAbort(reason = AbortReason.USER_INTERRUPTION) {
        const message = {
            session_id: this.sessionId,
            type: 'abort',
            reason: reason
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 发送IoT设备描述符
     * @param {Object} descriptors - 设备描述符对象
     */
    sendIOTDescriptors(descriptors) {
        const message = {
            session_id: this.sessionId,
            type: 'iot',
            descriptors: descriptors
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 发送IoT设备状态
     * @param {Object} states - 设备状态对象
     */
    sendIOTStates(states) {
        const message = {
            session_id: this.sessionId,
            type: 'iot',
            states: states
        };
        
        this.sendTextMessage(JSON.stringify(message));
    }

    /**
     * 发送文本消息
     * @param {string} message - 要发送的消息
     */
    sendTextMessage(message) {
        if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
            this.websocket.send(message);
        } else {
            console.error('WebSocket未连接，无法发送消息');
        }
    }

    /**
     * 发送二进制音频数据
     * @param {ArrayBuffer|Uint8Array} audioData - 音频数据
     */
    sendAudioData(audioData) {
        if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
            this.websocket.send(audioData);
        } else {
            console.error('WebSocket未连接，无法发送音频数据');
        }
    }

    /**
     * 检查连接状态
     */
    isConnected() {
        return this.connected && this.websocket && this.websocket.readyState === WebSocket.OPEN;
    }

    // 私有方法
    _onOpen() {
        console.log('WebSocket连接已建立');
        this.connected = true;
        
        // 发送认证信息（如果需要）
        if (this.accessToken) {
            this.sendTextMessage(`Authorization: Bearer ${this.accessToken}`);
            this.sendTextMessage(`Device-ID: ${this.deviceId}`);
        }
        
        // 发送Hello消息
        setTimeout(() => {
            this.sendHello();
        }, 100);
        
        if (this.onConnected) {
            this.onConnected();
        }
    }

    _onMessage(event) {
        if (typeof event.data === 'string') {
            // 处理文本消息
            this._handleTextMessage(event.data);
        } else {
            // 处理二进制音频数据
            this._handleAudioData(event.data);
        }
    }

    _onClose(event) {
        console.log('WebSocket连接已关闭:', event.code, event.reason);
        this.connected = false;
        this.helloReceived = false;
        
        if (this.onDisconnected) {
            this.onDisconnected(event);
        }
    }

    _onError(error) {
        console.error('WebSocket错误:', error);
        if (this.onError) {
            this.onError(error);
        }
    }

    _handleTextMessage(messageText) {
        try {
            const data = JSON.parse(messageText);
            const messageType = data.type;

            // 更新会话ID
            if (data.session_id) {
                this.sessionId = data.session_id;
            }

            // 根据消息类型分发处理
            switch (messageType) {
                case 'hello':
                    this._handleHelloMessage(data);
                    break;
                case 'tts':
                    this._handleTTSMessage(data);
                    break;
                case 'stt':
                    this._handleSTTMessage(data);
                    break;
                case 'llm':
                    this._handleLLMMessage(data);
                    break;
                case 'iot':
                    this._handleIOTMessage(data);
                    break;
                default:
                    console.warn('收到未知类型的消息:', messageType, data);
            }

            // 通用文本消息回调
            if (this.onTextMessage) {
                this.onTextMessage(data);
            }

        } catch (error) {
            console.error('解析JSON消息失败:', error, messageText);
        }
    }

    _handleHelloMessage(data) {
        console.log('收到服务器Hello消息:', data);
        this.helloReceived = true;
        
        // 验证传输方式
        if (data.transport !== 'websocket') {
            console.error('不支持的传输方式:', data.transport);
            return;
        }
        
        // 连接完全建立
        console.log('WebSocket连接完全建立');
    }

    _handleTTSMessage(data) {
        console.log('收到TTS消息:', data);
        
        const state = data.state;
        const text = data.text;
        
        if (state === 'sentence_start' && text) {
            console.log('AI回复:', text);
        }
        
        if (this.onTTSMessage) {
            this.onTTSMessage(data);
        }
    }

    _handleSTTMessage(data) {
        console.log('收到STT消息:', data);
        
        const text = data.text;
        if (text) {
            console.log('用户语音识别:', text);
        }
        
        if (this.onSTTMessage) {
            this.onSTTMessage(data);
        }
    }

    _handleLLMMessage(data) {
        console.log('收到LLM消息:', data);
        
        const emotion = data.emotion;
        if (emotion) {
            console.log('情感状态:', emotion);
        }
        
        if (this.onLLMMessage) {
            this.onLLMMessage(data);
        }
    }

    _handleIOTMessage(data) {
        console.log('收到IoT消息:', data);
        
        if (this.onIOTMessage) {
            this.onIOTMessage(data);
        }
    }

    _handleAudioData(audioData) {
        // 处理接收到的音频数据（通常是Opus编码的TTS音频）
        if (this.onAudioData) {
            this.onAudioData(audioData);
        }
    }
}
```

## 使用示例

```javascript
// 创建客户端实例
const client = new XiaozhiWebSocketClient({
    websocketUrl: 'wss://ws.xiaozhi.ai',  // 或自定义服务器地址
    deviceId: 'your-device-id',           // 设备ID (通常是MAC地址)
    accessToken: 'your-access-token'      // 访问令牌
});

// 设置事件回调
client.onConnected = () => {
    console.log('已连接到小智服务器');
    
    // 连接后可以开始监听
    client.startListening(ListeningMode.AUTO_STOP);
};

client.onDisconnected = (event) => {
    console.log('与服务器断开连接');
};

client.onError = (error) => {
    console.error('连接错误:', error);
};

client.onTTSMessage = (data) => {
    if (data.state === 'sentence_start' && data.text) {
        // 显示AI回复文本
        displayAIResponse(data.text);
    }
};

client.onSTTMessage = (data) => {
    if (data.text) {
        // 显示用户语音识别结果
        displayUserInput(data.text);
    }
};

client.onAudioData = (audioData) => {
    // 播放接收到的TTS音频
    playTTSAudio(audioData);
};

// 连接到服务器
client.connect().then(() => {
    console.log('WebSocket连接成功');
}).catch((error) => {
    console.error('连接失败:', error);
});

// 发送文本消息
function sendTextMessage(text) {
    client.sendTextQuery(text);
}

// 开始语音输入
function startVoiceInput() {
    client.startListening(ListeningMode.MANUAL);
    // 开始录音并发送音频数据
    startRecording((audioData) => {
        client.sendAudioData(audioData);
    });
}

// 停止语音输入
function stopVoiceInput() {
    client.stopListening();
    stopRecording();
}

// 中断AI说话
function interruptAI() {
    client.sendAbort(AbortReason.USER_INTERRUPTION);
}
```

## 消息协议详解

### 1. Hello消息
**客户端发送:**
```json
{
    "type": "hello",
    "version": 1,
    "transport": "websocket",
    "audio_params": {
        "format": "opus",
        "sample_rate": 16000,
        "channels": 1,
        "frame_duration": 60
    }
}
```

**服务器响应:**
```json
{
    "type": "hello",
    "session_id": "unique-session-id",
    "transport": "websocket",
    "version": 1
}
```

### 2. 监听控制消息
**开始监听:**
```json
{
    "session_id": "session-id",
    "type": "listen",
    "state": "start",
    "mode": "auto"
}
```

**停止监听:**
```json
{
    "session_id": "session-id",
    "type": "listen",
    "state": "stop"
}
```

### 3. 文本查询消息
```json
{
    "session_id": "session-id",
    "type": "listen",
    "state": "detect",
    "text": "用户输入的文本",
    "source": "text"
}
```

### 4. TTS消息（服务器发送）
```json
{
    "type": "tts",
    "state": "sentence_start",
    "text": "AI回复的文本内容"
}
```

### 5. STT消息（服务器发送）
```json
{
    "type": "stt",
    "text": "用户语音识别结果"
}
```

### 6. 中止消息
```json
{
    "session_id": "session-id",
    "type": "abort",
    "reason": "user_interruption"
}
```

### 7. IoT设备消息
**发送设备描述符:**
```json
{
    "session_id": "session-id",
    "type": "iot",
    "descriptors": {
        "devices": [
            {
                "id": "light_1",
                "name": "客厅灯",
                "type": "light",
                "capabilities": ["on_off", "brightness"]
            }
        ]
    }
}
```

**发送设备状态:**
```json
{
    "session_id": "session-id",
    "type": "iot",
    "states": {
        "light_1": {
            "power": true,
            "brightness": 80
        }
    }
}
```

## 音频处理

### 音频录制示例
```javascript
class AudioRecorder {
    constructor() {
        this.mediaRecorder = null;
        this.audioContext = null;
        this.stream = null;
    }

    async startRecording(onDataCallback) {
        try {
            this.stream = await navigator.mediaDevices.getUserMedia({
                audio: {
                    sampleRate: AudioConfig.INPUT_SAMPLE_RATE,
                    channelCount: AudioConfig.CHANNELS
                }
            });

            this.audioContext = new AudioContext({
                sampleRate: AudioConfig.INPUT_SAMPLE_RATE
            });

            const source = this.audioContext.createMediaStreamSource(this.stream);
            const processor = this.audioContext.createScriptProcessor(1024, 1, 1);

            processor.onaudioprocess = (event) => {
                const inputData = event.inputBuffer.getChannelData(0);
                // 将PCM数据编码为Opus格式
                const opusData = this.encodeAudioToOpus(inputData);
                onDataCallback(opusData);
            };

            source.connect(processor);
            processor.connect(this.audioContext.destination);

        } catch (error) {
            console.error('开始录音失败:', error);
            throw error;
        }
    }

    stopRecording() {
        if (this.stream) {
            this.stream.getTracks().forEach(track => track.stop());
        }
        if (this.audioContext) {
            this.audioContext.close();
        }
    }

    encodeAudioToOpus(pcmData) {
        // 实现Opus编码逻辑
        // 这里需要使用Opus编码库，如opus-recorder
        return new Uint8Array(pcmData.buffer);
    }
}
```

## 错误处理

```javascript
// 连接错误处理
client.onError = (error) => {
    console.error('WebSocket错误:', error);
    
    // 自动重连逻辑
    setTimeout(() => {
        if (!client.isConnected()) {
            console.log('尝试重新连接...');
            client.connect();
        }
    }, 3000);
};

// 断线重连
client.onDisconnected = (event) => {
    console.log('连接断开，准备重连...');
    
    setTimeout(() => {
        client.connect().catch(error => {
            console.error('重连失败:', error);
        });
    }, 1000);
};
```

## 最佳实践

1. **连接管理**: 实现自动重连机制，处理网络不稳定情况
2. **音频处理**: 使用Web Audio API进行高质量音频处理
3. **错误处理**: 完善的错误处理和用户提示
4. **性能优化**: 合理管理音频缓冲区，避免内存泄漏
5. **用户体验**: 提供清晰的连接状态和语音交互反馈

## 技术要点

### 协议特点
- **实时双向通信**: 基于WebSocket的全双工通信
- **混合数据传输**: 同时支持JSON文本消息和二进制音频数据
- **会话管理**: 通过session_id管理对话会话
- **音频编码**: 使用Opus格式进行高效音频压缩
- **设备认证**: 通过Bearer Token和设备ID进行身份验证

### 状态流转
```
IDLE → CONNECTING → LISTENING → SPEAKING → IDLE
  ↑                                          ↓
  ←――――――――――――― 完成播放 ←―――――――――――――――――
```

### 音频参数
- **输入采样率**: 16kHz
- **输出采样率**: 24kHz (官方服务器) 或 16kHz
- **声道数**: 1 (单声道)
- **帧长度**: 60ms
- **编码格式**: Opus

这个API文档提供了完整的小智WebSocket通信协议的JavaScript实现，您可以基于此构建功能完整的Web端语音AI助手。 
