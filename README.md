# 小智语音助手 (XiaoZhi Client)

基于 Rust 开发的智能语音助手客户端，支持实时语音对话、WebSocket 通信、Opus 音频编解码和多设备管理。

## 功能特性

- **实时语音对话** - 麦克风采集 + AI 交互 + 语音播报
- **WebSocket 双向通信** - 与服务器实时传输音频和指令
- **Opus 音频编解码** - 高效压缩，低延迟传输
- **多设备管理** - 支持虚拟设备创建、切换、激活
- **跨平台支持** - Windows (WASAPI)、macOS (CoreAudio)、Linux (ALSA)
- **彩色交互界面** - 设备管理菜单，友好的命令行体验
- **MCP 插件系统** - 支持外部工具扩展

## 项目结构

```
rust/
├── src/
│   ├── main.rs              # 入口，设备管理菜单
│   ├── client.rs             # 核心客户端逻辑
│   ├── websocket.rs          # WebSocket 协议
│   ├── voice/
│   │   ├── recorder.rs       # 麦克风录制 + Opus 编码
│   │   └── player.rs        # Opus 解码 + 播放
│   ├── device_manager.rs     # 多设备管理
│   ├── fingerprint.rs        # 设备指纹收集
│   ├── device_status.rs      # 设备状态检查 + 激活
│   ├── config.rs            # 配置管理
│   ├── ui/
│   │   ├── color.rs          # 彩色输出
│   │   └── menu.rs           # 交互式菜单
│   └── mcp/                  # MCP 插件协议
├── examples/                 # 示例程序
└── Cargo.toml
```

## 快速开始

### 安装依赖

```bash
# macOS
brew install cmake opus

# Ubuntu/Debian
sudo apt install cmake libopus-dev

# Windows (with vcpkg)
vcpkg install opus cmake
```

### 编译

```bash
# 调试模式
cargo build

# 发布模式
cargo build --release
```

### 设备管理

```bash
# 进入交互式设备管理菜单
./target/release/xiaozhi_client --manage

# 或调试模式
cargo run -- --manage
```

菜单功能：
- 列出所有设备
- 创建虚拟设备
- 切换当前设备
- 激活指定设备
- 查看设备详情
- 删除设备

### 启动语音助手

```bash
# 使用设备ID启动
./target/release/xiaozhi_client --device-id de:f3:0e:b5:ff:89 --device-name "设备名称"

# 激活当前设备
./target/release/xiaozhi_client --activate
```

### 交互模式命令

- `start` - 开始语音对话
- `stop` - 停止语音对话
- `quit` / `exit` - 退出程序

## 设备存储

设备信息存储在：
- **macOS**: `~/Library/Application Support/com.xiaozhi.client/devices.json`
- **Linux**: `~/.config/xiaozhi/client/devices.json`
- **Windows**: `%APPDATA%\com.xiaozhi.client\devices.json`

## 音频处理流程

```
录制: 麦克风 → 16位 PCM → 重采样(16kHz) → Opus 编码 → WebSocket 发送
播放: WebSocket 接收 → Opus 解码 → 重采样 → 播放设备
```

## MCP 插件

支持通过 stdio 通信的外部插件程序。插件放在 `executable_dir/plugins/` 目录。

示例插件见 `examples/` 目录。

## License

MIT