pub mod resources;
pub mod tools;
pub mod types;

use serde_json::Value;
use std::sync::Weak;

use self::tools::registry::{handle_tools_call, initialize_tools, ToolSource};
use self::types::*;
use crate::types::Result;

/// MCP协议处理器
pub struct MCPProtocol {
    /// 是否已初始化
    initialized: bool,
    /// 可用工具列表
    tools: Vec<ToolSource>,
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
    pub async fn handle_message(
        &mut self,
        message: MCPMessage,
    ) -> Result<Option<serde_json::Value>> {
        match message {
            MCPMessage::Initialize {
                id,
                jsonrpc: _,
                params,
            } => self.handle_initialize(id, params).await,
            MCPMessage::Initialized { id } => self.handle_initialized(id).await,
            MCPMessage::NotificationsInitialized { jsonrpc: _ } => {
                tracing::info!("✅ 收到客户端初始化通知");
                Ok(None)
            }
            MCPMessage::ToolsList { id, params } => self.handle_tools_list(id, params).await,
            MCPMessage::ToolsCall { id, params } => {
                handle_tools_call(id, params, &self.tools, &self.client_ref).await
            }
            MCPMessage::ResourcesList { id, params } => {
                resources::handle_resources_list(id, params, &self.resources).await
            }
            MCPMessage::ResourcesRead { id, params } => {
                resources::handle_resources_read(id, params).await
            }
            MCPMessage::NotificationsProgress { params } => {
                self.handle_progress_notification(params).await
            }
            MCPMessage::NotificationsCancelled {
                id: _,
                jsonrpc: _,
                params,
            } => self.handle_cancelled_notification(params).await,
            MCPMessage::LoggingMessage { params } => self.handle_logging_message(params).await,
        }
    }

    /// 处理初始化消息
    async fn handle_initialize(
        &mut self,
        id: Value,
        params: InitializeParams,
    ) -> Result<Option<serde_json::Value>> {
        tracing::info!("🔧 MCP初始化请求: 协议版本 {}", params.protocol_version);

        // 验证协议版本
        if params.protocol_version != MCP_PROTOCOL_VERSION {
            tracing::warn!(
                "⚠️ 协议版本不匹配: 期望 {}, 收到 {}",
                MCP_PROTOCOL_VERSION,
                params.protocol_version
            );
        }

        // 初始化工具列表
        self.tools = initialize_tools();

        // 初始化资源列表
        // self.resources = resources::initialize_resources();

        // 注意：根据MCP协议标准，initialized状态应该在收到initialized通知后才设置为true
        tracing::info!(
            "🎯 MCP工具和资源初始化完成，工具数量: {}, 资源数量: {}",
            self.tools.len(),
            self.resources.len()
        );

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
    async fn handle_initialized(&mut self, id: Value) -> Result<Option<serde_json::Value>> {
        self.initialized = true;
        tracing::info!("✅ MCP协议握手完成！客户端已确认初始化");
        tracing::info!("📋 服务器已准备就绪，注册工具数量: {}", self.tools.len());
        tracing::info!(
            "📋 服务器已准备就绪，注册资源数量: {}",
            self.resources.len()
        );
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
    async fn handle_tools_list(
        &self,
        id: Value,
        _params: Option<ToolsListParams>,
    ) -> Result<Option<serde_json::Value>> {
        tracing::info!("📋 收到工具列表请求, id: {}", id);
        tracing::debug!("📋 当前可用工具: {:?}", self.tools);

        let result = ToolsListResult {
            tools: self.tools.iter().map(|ts| ts.get_tool().clone()).collect(),
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

    /// 处理进度通知
    async fn handle_progress_notification(
        &self,
        params: ProgressNotification,
    ) -> Result<Option<serde_json::Value>> {
        tracing::debug!("📊 MCP进度通知: {}%", params.progress);
        Ok(None)
    }

    /// 处理取消通知
    async fn handle_cancelled_notification(
        &mut self,
        params: CancelledNotification,
    ) -> Result<Option<serde_json::Value>> {
        tracing::info!("❌ 收到取消通知: {}", params.reason);
        Ok(None)
    }

    /// 处理日志消息
    async fn handle_logging_message(
        &self,
        params: LoggingMessage,
    ) -> Result<Option<serde_json::Value>> {
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

    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// 获取可用工具列表
    pub fn get_tools(&self) -> Vec<Tool> {
        self.tools.iter().map(|ts| ts.get_tool().clone()).collect()
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
