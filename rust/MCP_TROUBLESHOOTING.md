# MCP协议问题排查指南

## 🐛 问题描述：tools/list请求未收到

### 症状
- MCP初始化成功，服务器响应了`initialize`请求
- 但是客户端没有发送`tools/list`请求
- 导致工具列表无法获取

### 🔍 根本原因
MCP协议响应格式不符合JSON-RPC 2.0标准，导致客户端认为握手未完成。

### 📋 MCP协议标准握手流程
```
1. 客户端 → 服务器：initialize 请求
2. 服务器 → 客户端：initialize 响应 (JSON-RPC 2.0格式)
3. 客户端 → 服务器：initialized 通知 (确认握手完成)
4. 客户端 → 服务器：tools/list 请求
5. 服务器 → 客户端：tools/list 响应 (包含工具列表)
```

### 🛠️ 修复内容

#### 1. 统一响应格式
将所有MCP响应改为标准JSON-RPC 2.0格式：
```rust
// 修复前 (使用MCPResponse包装)
let response = MCPResponse {
    id: serde_json::Value::String(id),
    result: Some(result),
    error: None,
};

// 修复后 (标准JSON-RPC 2.0)
let response = serde_json::json!({
    "jsonrpc": "2.0",
    "id": id,
    "result": result
});
```

#### 2. 修复的方法
- ✅ `handle_initialize` - 初始化响应
- ✅ `handle_initialized` - 初始化确认响应
- ✅ `handle_tools_list` - 工具列表响应
- ✅ `handle_tools_call` - 工具调用响应
- ✅ `handle_resources_list` - 资源列表响应
- ✅ `handle_resources_read` - 资源读取响应

#### 3. 错误处理改进
```rust
// 修复前
let error_response: MCPResponse<T> = MCPResponse {
    id: serde_json::Value::String(id),
    result: None,
    error: Some(MCPError { ... }),
};

// 修复后
let error_response = serde_json::json!({
    "jsonrpc": "2.0",
    "id": id,
    "error": {
        "code": -32601,
        "message": "错误信息"
    }
});
```

### 🎯 预期效果
修复后，MCP协议握手流程应该正常完成：
1. ✅ 初始化请求/响应成功
2. ✅ 客户端发送initialized通知
3. ✅ 客户端发送tools/list请求
4. ✅ 服务器返回11个注册的工具

### 📝 日志输出示例
```
🔧 MCP初始化请求: 协议版本 2024-11-05
🎯 MCP工具和资源初始化完成，工具数量: 11, 资源数量: 4
📤 发送MCP初始化响应，等待客户端发送initialized通知
✅ MCP协议握手完成！客户端已确认初始化
📋 服务器已准备就绪，注册工具数量: 11
🎯 现在客户端可以发送tools/list和其他请求了
📋 收到工具列表请求, id: xxx
📤 发送工具列表响应，工具数量: 11
```

### 🔧 支持的工具列表
1. `hello_world` - Hello World示例工具
2. `send_message` - 发送文本消息
3. `get_device_state` - 获取设备状态
4. `get_device_info` - 获取设备信息
5. `start_listening` - 开始语音监听
6. `stop_listening` - 停止语音监听
7. `interrupt_conversation` - 打断对话
8. `set_led` - 控制LED灯
9. `read_sensor` - 读取传感器数据
10. `get_wifi_status` - 获取WiFi状态
11. `system_info` - 获取系统信息

### 🚀 验证方法
1. 启动Rust客户端
2. 检查日志中是否包含完整的握手流程
3. 确认收到`tools/list`请求
4. 验证工具列表响应包含11个工具 