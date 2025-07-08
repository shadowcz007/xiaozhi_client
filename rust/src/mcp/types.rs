use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        id: Value,
    },
    #[serde(rename = "notifications/initialized")]
    NotificationsInitialized {
        jsonrpc: Option<String>,
    },
    #[serde(rename = "tools/list")]
    ToolsList {
        id: Value,
        params: Option<ToolsListParams>,
    },
    #[serde(rename = "tools/call")]
    ToolsCall {
        id: Value,
        params: ToolsCallParams,
    },
    #[serde(rename = "resources/list")]
    ResourcesList {
        id: Value,
        params: Option<ResourcesListParams>,
    },
    #[serde(rename = "resources/read")]
    ResourcesRead {
        id: Value,
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