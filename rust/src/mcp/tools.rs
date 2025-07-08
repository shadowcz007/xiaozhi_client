use std::collections::HashMap;
use std::sync::Weak;
use serde_json::Value;
use std::process::Command;

use crate::types::{Result, ClientError, ListeningMode};
use super::types::{
    Content, Tool, ToolsCallParams, ToolsCallResult,
    MCP_PROTOCOL_VERSION,
};

/// 初始化工具列表
pub fn initialize_tools() -> Vec<Tool> {
    vec![
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
            name: "get_wifi_status".to_string(),
            description: "获取ESP32的WiFi连接状态".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }, 
    ]
}

/// 处理工具调用请求
pub async fn handle_tools_call(
    id: Value, 
    params: ToolsCallParams, 
    client_ref: &Option<Weak<crate::client::Client>>
) -> Result<Option<serde_json::Value>> {
    tracing::info!("🔧 MCP工具调用: {}", params.name);

    let result = match params.name.as_str() {
        "hello_world" => handle_hello_world_tool(params.arguments).await?, 
        "get_wifi_status" => handle_get_wifi_status_tool().await?,
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

/// 获取WiFi信息
fn get_wifi_info() -> std::io::Result<serde_json::Value> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .arg("-I")
            .output()?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut info = serde_json::Map::new();
        
        // 解析airport命令输出
        for line in output_str.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                
                match key {
                    "SSID" => info.insert("ssid".to_string(), Value::String(value.to_string())),
                    "agrCtlRSSI" => info.insert("signal_strength".to_string(), Value::String(value.to_string())),
                    "lastTxRate" => info.insert("tx_rate".to_string(), Value::String(value.to_string())),
                    "MCS" => info.insert("mcs".to_string(), Value::String(value.to_string())),
                    _ => None,
                };
            }
        }
        
        Ok(Value::Object(info))
    }
    
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("netsh")
            .args(["wlan", "show", "interfaces"])
            .output()?;
            
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut info = serde_json::Map::new();
        
        // 解析netsh命令输出
        for line in output_str.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                
                match key {
                    "SSID" => info.insert("ssid".to_string(), Value::String(value.to_string())),
                    "Signal" => info.insert("signal_strength".to_string(), Value::String(value.to_string())),
                    "Transmit rate (Mbps)" => info.insert("tx_rate".to_string(), Value::String(value.to_string())),
                    _ => None,
                };
            }
        }
        
        Ok(Value::Object(info))
    }
}

/// 处理Hello World工具
async fn handle_hello_world_tool(arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
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

 

/// 处理WiFi状态工具
async fn handle_get_wifi_status_tool() -> Result<ToolsCallResult> {
    match get_wifi_info() {
        Ok(wifi_info) => {
            tracing::info!("📶 获取WiFi状态成功");
            Ok(ToolsCallResult {
                content: vec![Content::Text {
                    text: format!("📶 WiFi状态:\n{}", serde_json::to_string_pretty(&wifi_info)?),
                }],
                is_error: None,
            })
        }
        Err(e) => {
            tracing::error!("📶 获取WiFi状态失败: {}", e);
            Ok(ToolsCallResult {
                content: vec![Content::Text {
                    text: format!("❌ 获取WiFi状态失败: {}", e),
                }],
                is_error: Some(true),
            })
        }
    }
}

