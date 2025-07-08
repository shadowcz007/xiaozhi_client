use std::process::Command;
use serde_json::Value;
use crate::mcp::types::{Content, Tool, ToolsCallResult};
use crate::types::Result;

pub fn get_tool() -> Tool {
    Tool {
        name: "get_wifi_status".to_string(),
        description: "获取设备的WiFi连接状态".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn get_wifi_info() -> std::io::Result<Value> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .arg("-I")
            .output()?;
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut info = serde_json::Map::new();
        
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

pub async fn handle() -> Result<ToolsCallResult> {
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