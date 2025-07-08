use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Weak;
use tokio::sync::Mutex;
use crate::types::{Result, ClientError, DeviceState, ListeningMode};
use serde_json::Value;

/// MCP协议版本
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// MCP消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum MCPMessage {
    #[serde(rename = "initialize")]
    Initialize {
        #[serde(default)]
        id: Value,
        #[serde(default)]
        jsonrpc: Option<String>,
        params: InitializeParams,
    },
    #[serde(rename = "initialized")]
    Initialized {
        id: String,
    },
    #[serde(rename = "notifications/initialized")]
    NotificationsInitialized {
        jsonrpc: Option<String>,
    },
    #[serde(rename = "tools/list")]
    ToolsList {
        id: String,
        params: Option<ToolsListParams>,
    },
    #[serde(rename = "tools/call")]
    ToolsCall {
        id: String,
        params: ToolsCallParams,
    },
    #[serde(rename = "resources/list")]
    ResourcesList {
        id: String,
        params: Option<ResourcesListParams>,
    },
    #[serde(rename = "resources/read")]
    ResourcesRead {
        id: String,
        params: ResourcesReadParams,
    },
    #[serde(rename = "notifications/progress")]
    NotificationsProgress {
        params: ProgressNotification,
    },
    #[serde(rename = "notifications/cancelled")]
    NotificationsCancelled {
        id: String,
        jsonrpc: Option<String>,
        params: CancelledNotification,
    },
    #[serde(rename = "logging/message")]
    LoggingMessage {
        params: LoggingMessage,
    },
}

/// MCP响应消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse<T> {
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

/// MCP错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// 初始化参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// 客户端能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
}

/// 采样能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// 客户端信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// 工具列表参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// 工具调用参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// 资源列表参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// 资源读取参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadParams {
    pub uri: String,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// 工具列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCallResult {
    pub content: Vec<Content>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// 内容类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: ResourceReference },
}

/// 资源引用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// 资源定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// 资源列表结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// 资源读取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadResult {
    pub contents: Vec<Content>,
}

/// 进度通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    #[serde(rename = "progressToken")]
    pub progress_token: String,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

/// 取消通知参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledNotification {
    pub reason: String,
}

/// 日志消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessage {
    pub level: LogLevel,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

/// MCP协议处理器
pub struct MCPProtocol {
    /// 是否已初始化
    initialized: bool,
    /// 可用工具列表
    tools: Vec<Tool>,
    /// 可用资源列表
    resources: Vec<Resource>,
    /// Client实例的弱引用，用于调用实际的设备功能
    client_ref: Option<Weak<crate::client::Client>>,
}

impl MCPProtocol {
    /// 创建新的MCP协议处理器
    pub fn new() -> Self {
        Self {
            initialized: false,
            tools: Vec::new(),
            resources: Vec::new(),
            client_ref: None,
        }
    }

    /// 设置Client实例引用
    pub fn set_client_ref(&mut self, client_ref: Weak<crate::client::Client>) {
        self.client_ref = Some(client_ref);
    }

    /// 处理MCP消息
    pub async fn handle_message(&mut self, message: MCPMessage) -> Result<Option<serde_json::Value>> {
        match message {
            MCPMessage::Initialize { id, jsonrpc: _, params } => {
                self.handle_initialize(id, params).await
            }
            MCPMessage::Initialized { id } => {
                self.handle_initialized(id).await
            }
            MCPMessage::NotificationsInitialized { jsonrpc: _ } => {
                tracing::info!("✅ 收到客户端初始化通知");
                Ok(None)
            }
            MCPMessage::ToolsList { id, params } => {
                self.handle_tools_list(id, params).await
            }
            MCPMessage::ToolsCall { id, params } => {
                self.handle_tools_call(id, params).await
            }
            MCPMessage::ResourcesList { id, params } => {
                self.handle_resources_list(id, params).await
            }
            MCPMessage::ResourcesRead { id, params } => {
                self.handle_resources_read(id, params).await
            }
            MCPMessage::NotificationsProgress { params } => {
                self.handle_progress_notification(params).await
            }
            MCPMessage::NotificationsCancelled { id: _, jsonrpc: _, params } => {
                self.handle_cancelled_notification(params).await
            }
            MCPMessage::LoggingMessage { params } => {
                self.handle_logging_message(params).await
            }
        }
    }

    /// 处理初始化消息
    async fn handle_initialize(&mut self, id: Value, params: InitializeParams) -> Result<Option<serde_json::Value>> {
        tracing::info!("🔧 MCP初始化请求: 协议版本 {}", params.protocol_version);
        
        // 验证协议版本
        if params.protocol_version != MCP_PROTOCOL_VERSION {
            tracing::warn!("⚠️ 协议版本不匹配: 期望 {}, 收到 {}", MCP_PROTOCOL_VERSION, params.protocol_version);
        }

        // 初始化工具列表
        self.initialize_tools();
        
        // 初始化资源列表
        self.initialize_resources();
        
        // 注意：根据MCP协议标准，initialized状态应该在收到initialized通知后才设置为true
        tracing::info!("🎯 MCP工具和资源初始化完成，工具数量: {}, 资源数量: {}", 
                      self.tools.len(), self.resources.len());

        // 构造标准的JSON-RPC 2.0响应格式
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {
                        "listChanged": true
                    },
                    "resources": {
                        "subscribe": false,
                        "listChanged": true
                    },
                    "logging": {}
                },
                "serverInfo": {
                    "name": "xiaozhi-client",
                    "version": "1.0.0"
                }
            }
        });

        tracing::info!("📤 发送MCP初始化响应，等待客户端发送initialized通知");
        Ok(Some(response))
    }

    /// 处理初始化完成消息
    async fn handle_initialized(&mut self, id: String) -> Result<Option<serde_json::Value>> {
        self.initialized = true;
        tracing::info!("✅ MCP协议握手完成！客户端已确认初始化");
        tracing::info!("📋 服务器已准备就绪，注册工具数量: {}", self.tools.len());
        tracing::info!("📋 服务器已准备就绪，注册资源数量: {}", self.resources.len());
        tracing::info!("🎯 现在客户端可以发送tools/list和其他请求了");

        // 使用标准JSON-RPC 2.0格式
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        });

        Ok(Some(response))
    }

    /// 处理工具列表请求
    async fn handle_tools_list(&self, id: String, _params: Option<ToolsListParams>) -> Result<Option<serde_json::Value>> {
        tracing::info!("📋 收到工具列表请求, id: {}", id);
        tracing::debug!("📋 当前可用工具: {:?}", self.tools);
        
        let result = ToolsListResult {
            tools: self.tools.clone(),
            next_cursor: None,
        };

        // 使用标准JSON-RPC 2.0格式
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        tracing::info!("📤 发送工具列表响应，工具数量: {}", self.tools.len());
        Ok(Some(response))
    }

    /// 处理工具调用请求
    async fn handle_tools_call(&self, id: String, params: ToolsCallParams) -> Result<Option<serde_json::Value>> {
        tracing::info!("🔧 MCP工具调用: {}", params.name);

        let result = match params.name.as_str() {
            "hello_world" => self.handle_hello_world_tool(params.arguments).await?,
            "send_message" => self.handle_send_message_tool(params.arguments).await?,
            "get_device_state" => self.handle_get_device_state_tool().await?,
            "get_device_info" => self.handle_get_device_info_tool().await?,
            "start_listening" => self.handle_start_listening_tool(params.arguments).await?,
            "stop_listening" => self.handle_stop_listening_tool().await?,
            "interrupt_conversation" => self.handle_interrupt_conversation_tool().await?,
            "set_led" => self.handle_set_led_tool(params.arguments).await?,
            "read_sensor" => self.handle_read_sensor_tool(params.arguments).await?,
            "get_wifi_status" => self.handle_get_wifi_status_tool().await?,
            "system_info" => self.handle_system_info_tool().await?,
            _ => {
                // 使用标准JSON-RPC 2.0错误格式
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("未知工具: {}", params.name)
                    }
                });
                return Ok(Some(error_response));
            }
        };

        // 使用标准JSON-RPC 2.0格式
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        Ok(Some(response))
    }

    /// 处理资源列表请求
    async fn handle_resources_list(&self, id: String, _params: Option<ResourcesListParams>) -> Result<Option<serde_json::Value>> {
        let result = ResourcesListResult {
            resources: self.resources.clone(),
            next_cursor: None,
        };

        // 使用标准JSON-RPC 2.0格式
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        Ok(Some(response))
    }

    /// 处理资源读取请求
    async fn handle_resources_read(&self, id: String, params: ResourcesReadParams) -> Result<Option<serde_json::Value>> {
        tracing::info!("📄 MCP资源读取: {}", params.uri);

        let contents = match params.uri.as_str() {
            "esp32://device/status" => vec![Content::Text { 
                text: serde_json::to_string_pretty(&serde_json::json!({
                    "device_type": "ESP32",
                    "status": "online",
                    "last_update": chrono::Utc::now().timestamp()
                }))? 
            }],
            "esp32://audio/config" => vec![Content::Text { 
                text: serde_json::to_string_pretty(&serde_json::json!({
                    "input_sample_rate": 16000,
                    "output_sample_rate": 24000,
                    "channels": 1,
                    "frame_duration": 20
                }))? 
            }],
            "esp32://gpio/config" => vec![Content::Text { 
                text: serde_json::to_string_pretty(&serde_json::json!({
                    "digital_pins": [2, 4, 5, 12, 13, 14, 15, 16, 17, 18, 19, 21, 22, 23, 25, 26, 27, 32, 33],
                    "analog_pins": [32, 33, 34, 35, 36, 39],
                    "pwm_pins": [2, 4, 5, 12, 13, 14, 15, 16, 17, 18, 19, 21, 22, 23, 25, 26, 27]
                }))? 
            }],
            "esp32://wifi/status" => vec![Content::Text { 
                text: serde_json::to_string_pretty(&serde_json::json!({
                    "connected": true,
                    "ssid": "MyWiFi",
                    "signal_strength": -42,
                    "ip_address": "192.168.1.100"
                }))? 
            }],
            _ => {
                // 使用标准JSON-RPC 2.0错误格式
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32602,
                        "message": format!("未知资源: {}", params.uri)
                    }
                });
                return Ok(Some(error_response));
            }
        };

        let result = ResourcesReadResult { contents };

        // 使用标准JSON-RPC 2.0格式
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });

        Ok(Some(response))
    }

    /// 处理进度通知
    async fn handle_progress_notification(&self, params: ProgressNotification) -> Result<Option<serde_json::Value>> {
        tracing::debug!("📊 MCP进度通知: {}%", params.progress);
        Ok(None)
    }

    /// 处理取消通知
    async fn handle_cancelled_notification(&mut self, params: CancelledNotification) -> Result<Option<serde_json::Value>> {
        tracing::info!("❌ 收到取消通知: {}", params.reason);
        Ok(None)
    }

    /// 处理日志消息
    async fn handle_logging_message(&self, params: LoggingMessage) -> Result<Option<serde_json::Value>> {
        match params.level {
            LogLevel::Error | LogLevel::Critical => {
                tracing::error!("🔴 MCP日志: {:?}", params.data);
            }
            LogLevel::Warning => {
                tracing::warn!("🟡 MCP日志: {:?}", params.data);
            }
            LogLevel::Info | LogLevel::Notice => {
                tracing::info!("🔵 MCP日志: {:?}", params.data);
            }
            _ => {
                tracing::debug!("⚪ MCP日志: {:?}", params.data);
            }
        }
        Ok(None)
    }

    /// 初始化工具列表
    fn initialize_tools(&mut self) {
        self.tools = vec![
            // === 基础通信工具 ===
            Tool {
                name: "hello_world".to_string(),
                description: "Hello World示例工具，返回问候消息".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "要问候的名字",
                            "default": "World"
                        }
                    }
                }),
            },
            Tool {
                name: "send_message".to_string(),
                description: "发送文本消息给小智AI".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "要发送的消息内容"
                        }
                    },
                    "required": ["message"]
                }),
            },
            
            // === 设备状态工具 ===
            Tool {
                name: "get_device_state".to_string(),
                description: "获取ESP32设备当前状态".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "get_device_info".to_string(),
                description: "获取ESP32设备信息".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            
            // === 音频控制工具 ===
            Tool {
                name: "start_listening".to_string(),
                description: "开始语音监听".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "mode": {
                            "type": "string",
                            "enum": ["always_on", "auto_stop", "manual"],
                            "description": "监听模式",
                            "default": "always_on"
                        }
                    }
                }),
            },
            Tool {
                name: "stop_listening".to_string(),
                description: "停止语音监听".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "interrupt_conversation".to_string(),
                description: "打断当前对话".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            
            // === ESP32特定工具 ===
            Tool {
                name: "set_led".to_string(),
                description: "控制ESP32设备上的LED灯".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pin": {
                            "type": "integer",
                            "description": "LED连接的GPIO引脚号",
                            "minimum": 0,
                            "maximum": 39
                        },
                        "state": {
                            "type": "boolean",
                            "description": "LED状态：true为开启，false为关闭"
                        }
                    },
                    "required": ["pin", "state"]
                }),
            },
            Tool {
                name: "read_sensor".to_string(),
                description: "读取ESP32设备上的传感器数据".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sensor_type": {
                            "type": "string",
                            "enum": ["temperature", "humidity", "light", "analog"],
                            "description": "传感器类型"
                        },
                        "pin": {
                            "type": "integer",
                            "description": "传感器连接的引脚号",
                            "minimum": 0,
                            "maximum": 39
                        }
                    },
                    "required": ["sensor_type", "pin"]
                }),
            },
            Tool {
                name: "get_wifi_status".to_string(),
                description: "获取ESP32的WiFi连接状态".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "system_info".to_string(),
                description: "获取ESP32系统信息（内存、CPU等）".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];
    }

    /// 初始化资源列表
    fn initialize_resources(&mut self) {
        self.resources = vec![
            Resource {
                uri: "esp32://device/status".to_string(),
                name: "设备状态".to_string(),
                description: Some("获取ESP32设备当前状态信息".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "esp32://audio/config".to_string(),
                name: "音频配置".to_string(),
                description: Some("获取音频配置信息".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "esp32://gpio/config".to_string(),
                name: "GPIO配置".to_string(),
                description: Some("获取GPIO引脚配置信息".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "esp32://wifi/status".to_string(),
                name: "WiFi状态".to_string(),
                description: Some("获取WiFi连接状态信息".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ];
    }

    /// 处理Hello World工具
    async fn handle_hello_world_tool(&self, arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
        let name = arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("World");

        let greeting = format!("Hello, {}! 这是来自ESP32设备的问候！🎉", name);
        tracing::info!("👋 Hello World工具调用: {}", greeting);

        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: greeting,
            }],
            is_error: None,
        })
    }

    /// 处理发送消息工具
    async fn handle_send_message_tool(&self, arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
        let message = arguments
            .as_ref()
            .and_then(|args| args.get("message"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::invalid_state("缺少message参数"))?;

        // 如果有Client实例，则实际发送消息
        if let Some(client_weak) = &self.client_ref {
            if let Some(client) = client_weak.upgrade() {
                match client.send_text_message(message).await {
                    Ok(_) => {
                        tracing::info!("📝 MCP成功发送消息: {}", message);
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: format!("✅ 消息已发送: {}", message),
                            }],
                            is_error: None,
                        });
                    }
                    Err(e) => {
                        tracing::error!("📝 MCP发送消息失败: {}", e);
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: format!("❌ 发送消息失败: {}", e),
                            }],
                            is_error: Some(true),
                        });
                    }
                }
            }
        }

        // 如果没有Client实例，返回模拟结果
        tracing::info!("📝 MCP发送消息（模拟）: {}", message);
        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("📝 消息已发送（模拟）: {}", message),
            }],
            is_error: None,
        })
    }

    /// 处理获取设备状态工具
    async fn handle_get_device_state_tool(&self) -> Result<ToolsCallResult> {
        // 如果有Client实例，则获取真实状态
        if let Some(client_weak) = &self.client_ref {
            if let Some(client) = client_weak.upgrade() {
                let state = client.get_device_state().await;
                let is_recording = client.is_recording();
                let keep_listening = client.is_keep_listening();
                
                let state_info = serde_json::json!({
                    "device_state": state,
                    "is_recording": is_recording,
                    "keep_listening": keep_listening,
                    "timestamp": chrono::Utc::now().timestamp()
                });

                return Ok(ToolsCallResult {
                    content: vec![Content::Text {
                        text: format!("📊 设备状态: {}", serde_json::to_string_pretty(&state_info)?),
                    }],
                    is_error: None,
                });
            }
        }

        // 如果没有Client实例，返回模拟结果
        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: "📊 设备状态: 空闲（模拟）".to_string(),
            }],
            is_error: None,
        })
    }

    /// 处理获取设备信息工具
    async fn handle_get_device_info_tool(&self) -> Result<ToolsCallResult> {
        let device_info = serde_json::json!({
            "device_type": "ESP32",
            "firmware_version": "1.0.0",
            "mcp_version": MCP_PROTOCOL_VERSION,
            "capabilities": ["audio", "gpio", "wifi", "sensors"],
            "uptime": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs(),
            "memory": {
                "total": "4MB",
                "available": "2.5MB"
            }
        });

        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("📱 设备信息:\n{}", serde_json::to_string_pretty(&device_info)?),
            }],
            is_error: None,
        })
    }

    /// 处理LED控制工具
    async fn handle_set_led_tool(&self, arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
        let pin = arguments
            .as_ref()
            .and_then(|args| args.get("pin"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ClientError::invalid_state("缺少pin参数"))? as u8;

        let state = arguments
            .as_ref()
            .and_then(|args| args.get("state"))
            .and_then(|v| v.as_bool())
            .ok_or_else(|| ClientError::invalid_state("缺少state参数"))?;

        // 在实际ESP32实现中，这里会调用GPIO控制函数
        tracing::info!("💡 设置LED: GPIO{} = {}", pin, if state { "ON" } else { "OFF" });

        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("💡 LED控制成功: GPIO{} 设置为 {}", pin, if state { "开启" } else { "关闭" }),
            }],
            is_error: None,
        })
    }

    /// 处理传感器读取工具
    async fn handle_read_sensor_tool(&self, arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
        let sensor_type = arguments
            .as_ref()
            .and_then(|args| args.get("sensor_type"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::invalid_state("缺少sensor_type参数"))?;

        let pin = arguments
            .as_ref()
            .and_then(|args| args.get("pin"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ClientError::invalid_state("缺少pin参数"))? as u8;

        // 在实际ESP32实现中，这里会读取真实的传感器数据
        let sensor_data = match sensor_type {
            "temperature" => serde_json::json!({
                "type": "temperature",
                "value": 25.6,
                "unit": "°C",
                "pin": pin,
                "timestamp": chrono::Utc::now().timestamp()
            }),
            "humidity" => serde_json::json!({
                "type": "humidity",
                "value": 60.2,
                "unit": "%",
                "pin": pin,
                "timestamp": chrono::Utc::now().timestamp()
            }),
            "light" => serde_json::json!({
                "type": "light",
                "value": 850,
                "unit": "lux",
                "pin": pin,
                "timestamp": chrono::Utc::now().timestamp()
            }),
            "analog" => serde_json::json!({
                "type": "analog",
                "value": 2048,
                "unit": "ADC_value",
                "pin": pin,
                "timestamp": chrono::Utc::now().timestamp()
            }),
            _ => {
                return Ok(ToolsCallResult {
                    content: vec![Content::Text {
                        text: format!("❌ 不支持的传感器类型: {}", sensor_type),
                    }],
                    is_error: Some(true),
                });
            }
        };

        tracing::info!("🌡️ 传感器读取: {}", serde_json::to_string(&sensor_data)?);

        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("🌡️ 传感器数据:\n{}", serde_json::to_string_pretty(&sensor_data)?),
            }],
            is_error: None,
        })
    }

    /// 处理WiFi状态工具
    async fn handle_get_wifi_status_tool(&self) -> Result<ToolsCallResult> {
        // 在实际ESP32实现中，这里会获取真实的WiFi状态
        let wifi_status = serde_json::json!({
            "connected": true,
            "ssid": "MyWiFi",
            "signal_strength": -42,
            "ip_address": "192.168.1.100",
            "mac_address": "AA:BB:CC:DD:EE:FF",
            "uptime": 3600
        });

        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("📶 WiFi状态:\n{}", serde_json::to_string_pretty(&wifi_status)?),
            }],
            is_error: None,
        })
    }

    /// 处理系统信息工具
    async fn handle_system_info_tool(&self) -> Result<ToolsCallResult> {
        // 在实际ESP32实现中，这里会获取真实的系统信息
        let uptime_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| ClientError::SystemTimeError(e))?
            .as_millis();

        let system_info = serde_json::json!({
            "chip_model": "ESP32",
            "cpu_frequency": "240MHz",
            "flash_size": "4MB",
            "free_heap": 150000,
            "minimum_free_heap": 120000,
            "chip_revision": 1,
            "sdk_version": "4.4.0",
            "uptime_ms": uptime_ms
        });

        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("💻 系统信息:\n{}", serde_json::to_string_pretty(&system_info)?),
            }],
            is_error: None,
        })
    }

    /// 处理开始监听工具
    async fn handle_start_listening_tool(&self, arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
        let mode_str = arguments
            .as_ref()
            .and_then(|args| args.get("mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("always_on");

        let mode = match mode_str {
            "always_on" => ListeningMode::AlwaysOn,
            "auto_stop" => ListeningMode::AutoStop,
            "manual" => ListeningMode::Manual,
            _ => ListeningMode::AlwaysOn,
        };

        // 如果有Client实例，则实际开始监听
        if let Some(client_weak) = &self.client_ref {
            if let Some(client) = client_weak.upgrade() {
                match client.start_listening(mode).await {
                    Ok(_) => {
                        tracing::info!("🎤 MCP成功开始监听: 模式 {}", mode_str);
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: format!("✅ 开始监听，模式: {}", mode_str),
                            }],
                            is_error: None,
                        });
                    }
                    Err(e) => {
                        tracing::error!("🎤 MCP开始监听失败: {}", e);
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: format!("❌ 开始监听失败: {}", e),
                            }],
                            is_error: Some(true),
                        });
                    }
                }
            }
        }

        tracing::info!("🎤 MCP开始监听（模拟）: 模式 {}", mode_str);
        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("🎤 开始监听（模拟），模式: {}", mode_str),
            }],
            is_error: None,
        })
    }

    /// 处理停止监听工具
    async fn handle_stop_listening_tool(&self) -> Result<ToolsCallResult> {
        // 如果有Client实例，则实际停止监听
        if let Some(client_weak) = &self.client_ref {
            if let Some(client) = client_weak.upgrade() {
                match client.stop_listening().await {
                    Ok(_) => {
                        tracing::info!("🛑 MCP成功停止监听");
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: "✅ 已停止监听".to_string(),
                            }],
                            is_error: None,
                        });
                    }
                    Err(e) => {
                        tracing::error!("🛑 MCP停止监听失败: {}", e);
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: format!("❌ 停止监听失败: {}", e),
                            }],
                            is_error: Some(true),
                        });
                    }
                }
            }
        }

        tracing::info!("🛑 MCP停止监听（模拟）");
        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: "🛑 已停止监听（模拟）".to_string(),
            }],
            is_error: None,
        })
    }

    /// 处理打断对话工具
    async fn handle_interrupt_conversation_tool(&self) -> Result<ToolsCallResult> {
        // 如果有Client实例，则实际打断对话
        if let Some(client_weak) = &self.client_ref {
            if let Some(client) = client_weak.upgrade() {
                match client.interrupt_conversation().await {
                    Ok(_) => {
                        tracing::info!("⚡ MCP成功打断对话");
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: "✅ 对话已被打断".to_string(),
                            }],
                            is_error: None,
                        });
                    }
                    Err(e) => {
                        tracing::error!("⚡ MCP打断对话失败: {}", e);
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text {
                                text: format!("❌ 打断对话失败: {}", e),
                            }],
                            is_error: Some(true),
                        });
                    }
                }
            }
        }

        tracing::info!("⚡ MCP打断对话（模拟）");
        Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: "⚡ 对话已被打断（模拟）".to_string(),
            }],
            is_error: None,
        })
    }

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 获取可用工具列表
    pub fn get_tools(&self) -> &[Tool] {
        &self.tools
    }

    /// 获取可用资源列表
    pub fn get_resources(&self) -> &[Resource] {
        &self.resources
    }
}

impl Default for MCPProtocol {
    fn default() -> Self {
        Self::new()
    }
}

// 默认 ID 生成函数
fn default_string_id() -> String {
    format!("{}", chrono::Utc::now().timestamp())
} 