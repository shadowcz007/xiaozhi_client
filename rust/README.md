# 小智语音助手 Rust 客户端

🤖 这是小智语音助手的 Rust 实现版本，提供完整的语音对话功能，包括实时音频录制、WebSocket 通信、Opus 音频编解码和智能对话管理。


# 小智客户端命令行使用说明

## 命令行使用方法

### 生成许可证
```bash
# 生成许可证密钥
./target/release/license_generator joy-client-mixlab-by-shadow shadow-mixlab-password123
```

### 运行客户端
```bash
# 基本运行命令
./target/release/xiaozhi_client --key <许可证密钥>

# 完整命令示例
./target/release/xiaozhi_client \
  --key <许可证密钥> \
  --device-id <设备ID> \
  --device-name <设备名称>

# 开发环境测试示例
./target/release/xiaozhi_client \
  --key eyJsaWNlbnNlIjoidGVzdC1saWNlbnNlIiwicGFzc3dvcmQiOiJ0ZXN0LXBhc3N3b3JkIn0=
```

### 交互命令
程序运行后可用的命令：
```bash
start   # 开始语音对话
stop    # 停止语音对话
quit    # 退出程序
exit    # 退出程序
``` 


## ✨ 功能特性

- 🎙️ **实时语音录制** - 使用 CPAL 进行跨平台音频采集
- 🎵 **音频编解码** - Opus 格式的高质量音频压缩
- 🌐 **WebSocket 通信** - 与服务器进行实时双向通信
- 🧠 **智能对话管理** - 自动处理语音识别、文本生成和语音合成
- 🔄 **状态管理** - 完整的设备状态跟踪和回调机制
- ⚡ **异步架构** - 基于 Tokio 的高性能异步处理
- 🛡️ **类型安全** - 充分利用 Rust 的类型系统保证程序安全

## 🚀 快速开始

### 系统要求

- Rust 1.70+ 
- 操作系统: Windows, macOS, Linux
- 音频设备: 麦克风和扬声器

### 安装依赖

```bash
# 克隆项目
cd rust

# 安装依赖
cargo build
```

### 基本使用

```rust
use xiaozhi_client::{DeviceStatusChecker, Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    xiaozhi_client::init_logging();
    
    let device_id = "your-device-id";
    
    // 检查设备状态
    let checker = DeviceStatusChecker::new();
    let status = checker.check_device_status(device_id).await?;
    
    if let Some(status_response) = status {
        // 创建配置
        let config = Config::new(
            status_response.websocket.url,
            status_response.websocket.token,
            device_id.to_string(),
            status_response.mqtt.client_id,
        );
        
        // 创建客户端
        let mut client = Client::new(config)?;
        
        // 设置状态回调
        client.set_state_change_callback(|state| {
            println!("状态变化: {:?}", state);
        });
        
        // 开始语音聊天
        client.start_voice_chat().await?;
        
        // 发送文本消息
        client.send_text_message("你好").await?;
        
        // 保持运行...
        tokio::signal::ctrl_c().await?;
        
        // 清理
        client.stop_voice_chat().await?;
        client.disconnect().await?;
    }
    
    Ok(())
}
```

## 📋 运行示例

### 方式一：直接运行主程序

```bash
# 使用默认设备ID
cargo run

# 使用指定设备ID
cargo run -- df:52:34:be:fa:38
```

### 方式二：运行示例程序

```bash
# 简单对话示例
cargo run --example simple_chat
```

### 方式三：编译可执行文件

```bash
# 编译发布版本
cargo build --release

# 运行可执行文件
./target/release/xiaozhi_client
```

## 🏗️ 项目结构

```
rust/
├── Cargo.toml                 # 项目配置和依赖
├── src/
│   ├── main.rs               # 主程序入口
│   ├── lib.rs                # 库入口
│   ├── types.rs              # 类型定义
│   ├── config.rs             # 配置管理
│   ├── device_status.rs      # 设备状态检查
│   ├── websocket.rs          # WebSocket协议
│   ├── client.rs             # 客户端核心逻辑
│   └── voice/                # 音频处理模块
│       ├── mod.rs            # 模块入口
│       ├── recorder.rs       # 音频录制
│       └── player.rs         # 音频播放
├── examples/
│   └── simple_chat.rs        # 使用示例
└── README.md                 # 项目文档
```

## 🔧 核心组件

### DeviceStatusChecker
负责检查设备激活状态，确保设备已正确注册。

```rust
let checker = DeviceStatusChecker::new();
let status = checker.check_device_status("device-id").await?;
```

### WebSocketProtocol
处理与服务器的 WebSocket 通信，包括消息发送、接收和协议处理。

```rust
let protocol = WebSocketProtocol::new(config);
let event_receiver = protocol.connect().await?;
```

### MicrophoneOpusRecorder
实时录制麦克风音频并编码为 Opus 格式。

```rust
let mut recorder = MicrophoneOpusRecorder::new(16000, 1, 320)?;
let opus_receiver = recorder.start_recording()?;
```

### NodeAudioPlayer
解码 Opus 音频数据并播放到扬声器。

```rust
let mut player = NodeAudioPlayer::new(24000, 1)?;
player.process_audio_data(opus_data)?;
```

### Client
主要的客户端接口，整合所有组件提供高级API。

```rust
let client = Client::new(config)?;
client.start_voice_chat().await?;
```

## 🎮 API 参考

### Client 方法

| 方法 | 描述 |
|------|------|
| `new(config)` | 创建新客户端实例 |
| `start_voice_chat()` | 开始语音聊天 |
| `stop_voice_chat()` | 停止语音聊天 |
| `send_text_message(text)` | 发送文本消息 |
| `start_listening(mode)` | 开始监听 |
| `stop_listening()` | 停止监听 |
| `interrupt_conversation()` | 打断当前对话 |
| `disconnect()` | 断开连接 |
| `get_device_state()` | 获取当前状态 |

### 设备状态

```rust
pub enum DeviceState {
    Idle,       // 空闲
    Connecting, // 连接中
    Listening,  // 监听中
    Speaking,   // 播放中
}
```

### 监听模式

```rust
pub enum ListeningMode {
    AlwaysOn,   // 持续监听
    AutoStop,   // 自动停止
    Manual,     // 手动控制
}
```

## ⚙️ 配置选项

```rust
let config = Config::new(websocket_url, access_token, device_id, client_id)
    .with_audio_config(AudioConfig {
        input_sample_rate: 16000,
        output_sample_rate: 24000,
        channels: 1,
        frame_duration: 20,
    })
    .with_connect_timeout(15000)
    .with_max_reconnect_attempts(3);
```

## 🔍 调试和日志

```rust
// 普通日志级别
xiaozhi_client::init_logging();

// 调试日志级别
xiaozhi_client::init_debug_logging();
```

日志输出示例：
```
🌐 连接到WebSocket服务器: wss://api.tenclass.net/xiaozhi/v1/
✅ WebSocket连接成功
🎤 启动麦克风录音...
🗣️ 开始播放AI回复
💬 AI回复内容: 你好！我是小智，很高兴为您服务。
```

## 🚨 错误处理

所有异步操作都返回 `Result<T, ClientError>`，主要错误类型：

- `WebSocketError` - WebSocket 连接错误
- `AudioError` - 音频处理错误
- `OpusError` - Opus 编解码错误
- `DeviceNotActivated` - 设备未激活
- `ConnectionTimeout` - 连接超时

```rust
match client.start_voice_chat().await {
    Ok(()) => println!("语音聊天已启动"),
    Err(ClientError::DeviceNotActivated) => {
        eprintln!("设备需要先激活");
    }
    Err(ClientError::ConnectionTimeout) => {
        eprintln!("连接超时，请检查网络");
    }
    Err(e) => eprintln!("其他错误: {}", e),
}
```

## 🔧 系统要求和依赖

### 音频系统要求

- **Windows**: WASAPI
- **macOS**: CoreAudio
- **Linux**: ALSA

### 主要依赖库

- `tokio` - 异步运行时
- `tokio-tungstenite` - WebSocket 客户端
- `cpal` - 跨平台音频库
- `opus` - Opus 音频编解码
- `serde` - 序列化/反序列化
- `tracing` - 结构化日志

## 🤝 贡献指南

1. Fork 项目
2. 创建功能分支: `git checkout -b feature/new-feature`
3. 提交更改: `git commit -am 'Add new feature'`
4. 推送分支: `git push origin feature/new-feature`
5. 提交 Pull Request

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🔗 相关链接

- [Node.js 版本](../README.md)
- [API 文档](https://docs.rs/xiaozhi_client)
- [问题反馈](https://github.com/your-repo/issues)

---

�� **享受与小智的智能对话吧！** 