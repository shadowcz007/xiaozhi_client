use std::collections::HashMap;
use std::sync::Weak;
use serde_json::Value;

use crate::types::{Result, ClientError};
use crate::mcp::types::{Tool, ToolsCallParams, ToolsCallResult};
use super::{wifi, plugin_manager};
use super::plugin_manager::Plugin;

/// 用于区分工具来源
#[derive(Debug, Clone)]
pub enum ToolSource {
    BuiltIn(Tool),
    Plugin(Plugin),
}

impl ToolSource {
    /// 获取工具的定义信息
    pub fn get_tool(&self) -> &Tool {
        match self {
            ToolSource::BuiltIn(tool) => tool,
            ToolSource::Plugin(plugin) => &plugin.tool,
        }
    }
}

/// 初始化工具列表
pub fn initialize_tools() -> Vec<ToolSource> {
    let mut tools = Vec::new();

    // 1. 加载内置工具
    tools.push(ToolSource::BuiltIn(wifi::get_tool()));
    tracing::info!("✅ 加载了 {} 个内置工具", tools.len());

    // 2. 加载插件工具
    match plugin_manager::load_plugins() {
        Ok(plugins) => {
            for plugin in plugins {
                tools.push(ToolSource::Plugin(plugin));
            }
        }
        Err(e) => {
            tracing::error!("❌ 加载插件失败: {}", e);
        }
    }

    tools
}

/// 处理工具调用
pub async fn handle_tools_call(
    id: Value, 
    params: ToolsCallParams, 
    tools: &[ToolSource],
    _client_ref: &Option<Weak<crate::client::Client>>
) -> Result<Option<Value>> {
    // 查找工具
    let tool_source = tools.iter().find(|t| t.get_tool().name == params.name);

    let result = match tool_source {
        Some(ToolSource::BuiltIn(tool)) => {
            // 调用内置工具
            match tool.name.as_str() {
                "get_wifi_status" => wifi::handle().await?,
                _ => return Err(ClientError::Internal(format!("未知的内置工具: {}", tool.name))),
            }
        }
        Some(ToolSource::Plugin(plugin)) => {
            // 调用插件工具
            plugin_manager::execute_plugin(plugin, params.arguments).await?
        }
        None => {
            return Err(ClientError::Internal(format!("未找到工具: {}", params.name)));
        }
    };

    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });

    Ok(Some(response))
} 