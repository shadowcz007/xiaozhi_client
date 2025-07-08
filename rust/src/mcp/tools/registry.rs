use std::collections::HashMap;
use std::sync::Weak;
use serde_json::Value;

use crate::types::{Result, ClientError};
use crate::mcp::types::{Tool, ToolsCallParams, ToolsCallResult};
use super::{hello_world, wifi};

/// 初始化工具列表
pub fn initialize_tools() -> Vec<Tool> {
    vec![
        hello_world::get_tool(),
        wifi::get_tool(),
    ]
}

/// 处理工具调用请求
pub async fn handle_tools_call(
    id: Value, 
    params: ToolsCallParams, 
    client_ref: &Option<Weak<crate::client::Client>>
) -> Result<Option<Value>> {
    tracing::info!("🔧 MCP工具调用: {}", params.name);

    let result = match params.name.as_str() {
        "hello_world" => hello_world::handle(params.arguments).await?,
        "get_wifi_status" => wifi::handle().await?,
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