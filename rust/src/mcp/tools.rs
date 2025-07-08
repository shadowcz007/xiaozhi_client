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
        "set_led" => handle_set_led_tool(params.arguments).await?,
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


/// 处理LED控制工具
async fn handle_set_led_tool(arguments: Option<HashMap<String, serde_json::Value>>) -> Result<ToolsCallResult> {
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

