# ESP32 MCP服务器实现

本文档介绍了基于Rust的ESP32 MCP (Model Context Protocol) 服务器实现，支持完整的MCP协议流程。

## 🎯 概述

这个实现将ESP32设备作为MCP服务器，后台API作为MCP客户端。ESP32设备通过WebSocket连接接收并响应后台的MCP调用请求，实现设备控制、状态查询、传感器读取等功能。

## 🔄 MCP协议流程

```
ESP32设备 (MCP Server)          后台API (MCP Client)
         │                              │
         │◄────── WebSocket连接 ─────────│
         │                              │
         │◄───── MCP Initialize ────────│
         │                              │
         │────── Initialize Response ──►│
         │                              │
         │◄───── MCP Tools List ────────│
         │                              │
         │────── Tools List Response ──►│
         │                              │
         │◄───── MCP Tool Call ─────────│
         │                              │
         │────── Tool Call Response ───►│
         │                              │
         │◄───── 更多MCP调用... ────────│
         │                              │
```

## 🛠️ 支持的MCP工具

### 基础通信工具

#### 1. hello_world
**描述**: Hello World示例工具，返回问候消息  
**参数**:
- `name` (可选): 要问候的名字，默认为"World"

**示例调用**:
```json
{
    "method": "tools/call",
    "params": {
        "name": "hello_world",
        "arguments": {
            "name": "ESP32"
        }
    }
}
```

**响应**:
```json
{
    "content": [
        {
            "type": "text",
            "text": "Hello, ESP32! 这是来自ESP32设备的问候！🎉"
        }
    ]
}
```

#### 2. send_message
**描述**: 发送文本消息给小智AI  
**参数**:
- `message` (必需): 要发送的消息内容

**示例调用**:
```json
{
    "method": "tools/call",
    "params": {
        "name": "send_message",
        "arguments": {
            "message": "你好，我是ESP32设备"
        }
    }
}
```

### 设备状态工具

#### 3. get_device_state
**描述**: 获取ESP32设备当前状态  
**参数**: 无

**响应示例**:
```json
{
    "content": [
        {
            "type": "text",
            "text": "{\n  \"device_state\": \"Idle\",\n  \"is_recording\": false,\n  \"keep_listening\": true,\n  \"timestamp\": 1701234567\n}"
        }
    ]
}
```

#### 4. get_device_info
**描述**: 获取ESP32设备信息  
**参数**: 无

**响应示例**:
```json
{
    "content": [
        {
            "type": "text",
            "text": "{\n  \"device_type\": \"ESP32\",\n  \"firmware_version\": \"1.0.0\",\n  \"mcp_version\": \"2024-11-05\",\n  \"capabilities\": [\"audio\", \"gpio\", \"wifi\", \"sensors\"],\n  \"uptime\": 3600,\n  \"memory\": {\n    \"total\": \"4MB\",\n    \"available\": \"2.5MB\"\n  }\n}"
        }
    ]
}
```

### 音频控制工具

#### 5. start_listening
**描述**: 开始语音监听  
**参数**:
- `mode` (可选): 监听模式，可选值：`always_on`、`auto_stop`、`manual`，默认为`always_on`

#### 6. stop_listening
**描述**: 停止语音监听  
**参数**: 无

#### 7. interrupt_conversation
**描述**: 打断当前对话  
**参数**: 无

### ESP32特定工具

#### 8. set_led
**描述**: 控制ESP32设备上的LED灯  
**参数**:
- `pin` (必需): LED连接的GPIO引脚号 (0-39)
- `state` (必需): LED状态，true为开启，false为关闭

**示例调用**:
```json
{
    "method": "tools/call",
    "params": {
        "name": "set_led",
        "arguments": {
            "pin": 2,
            "state": true
        }
    }
}
```

#### 9. read_sensor
**描述**: 读取ESP32设备上的传感器数据  
**参数**:
- `sensor_type` (必需): 传感器类型，可选值：`temperature`、`humidity`、`light`、`analog`
- `pin` (必需): 传感器连接的引脚号 (0-39)

**示例调用**:
```json
{
    "method": "tools/call",
    "params": {
        "name": "read_sensor",
        "arguments": {
            "sensor_type": "temperature",
            "pin": 34
        }
    }
}
```

**响应示例**:
```json
{
    "content": [
        {
            "type": "text",
            "text": "{\n  \"type\": \"temperature\",\n  \"value\": 25.6,\n  \"unit\": \"°C\",\n  \"pin\": 34,\n  \"timestamp\": 1701234567\n}"
        }
    ]
}
```

#### 10. get_wifi_status
**描述**: 获取ESP32的WiFi连接状态  
**参数**: 无

#### 11. system_info
**描述**: 获取ESP32系统信息（内存、CPU等）  
**参数**: 无

## 📄 支持的MCP资源

### 1. esp32://device/status
设备状态资源，包含设备类型、在线状态和最后更新时间。

### 2. esp32://audio/config
音频配置资源，包含采样率、声道数等音频参数。

### 3. esp32://gpio/config
GPIO配置资源，包含数字引脚、模拟引脚和PWM引脚的配置信息。

### 4. esp32://wifi/status
WiFi状态资源，包含连接状态、信号强度、IP地址等信息。

## 🚀 使用示例

### 运行ESP32 MCP服务器示例

```bash
cd xiaozhi_client/rust
cargo run --example esp32_mcp_server
```

### 编程使用

```rust
use std::sync::Arc;
use xiaozhi_client::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 创建ESP32客户端
    let client = create_client(
        "ws://localhost:8080/ws".to_string(),
        "esp32_token_123".to_string(),
        "esp32_device_001".to_string(),
        "xiaozhi_client_esp32".to_string()
    ).await?;
    
    // 获取MCP工具列表
    let tools = client.get_mcp_tools().await?;
    println!("可用工具数量: {}", tools.len());
    
    // 调用Hello World工具
    let mut args = std::collections::HashMap::new();
    args.insert("name".to_string(), serde_json::Value::String("ESP32".to_string()));
    let result = client.call_mcp_tool("hello_world", Some(args)).await?;
    println!("调用结果: {}", serde_json::to_string_pretty(&result)?);
    
    // 启动WebSocket连接
    client.start_voice_chat(Some("ESP32设备已准备就绪")).await?;
    
    Ok(())
}
```

## 🔧 实现细节

### MCP协议集成

1. **MCPProtocol结构体**: 管理MCP协议的实现，包含工具和资源的定义
2. **Client集成**: MCPProtocol通过弱引用与Client实例集成，可以调用实际的设备功能
3. **消息处理**: 支持完整的MCP消息类型处理，包括初始化、工具调用、资源访问等

### 错误处理

所有MCP工具调用都包含适当的错误处理，返回标准的MCP错误响应格式。

### 扩展性

可以轻松添加新的工具和资源：

1. 在`initialize_tools()`方法中添加新工具定义
2. 在`handle_tools_call()`方法中添加工具路由
3. 实现对应的工具处理方法

## 📋 待办事项

- [ ] 实际的GPIO控制功能
- [ ] 真实的传感器数据读取
- [ ] WiFi管理功能
- [ ] OTA固件更新支持
- [ ] 设备配置管理
- [ ] 安全认证机制

## 🤝 贡献

欢迎提交Issue和Pull Request来改进这个MCP实现！

## 📄 许可证

本项目采用MIT许可证。 