use crate::mcp::types::{Content, Tool, ToolsCallResult};
use crate::types::Result;
use serde_json::Value;
use std::process::Command;

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
        // 方法1: 使用 networksetup 获取 WiFi 信息
        let output = Command::new("networksetup")
            .args(["-getairportnetwork", "en0"])
            .output()?;

        let mut info = serde_json::Map::new();
        let output_str = String::from_utf8_lossy(&output.stdout);

        if output_str.contains("Off") || output_str.contains("You are not") {
            info.insert("status".to_string(), Value::String("disconnected".to_string()));
        } else {
            info.insert("status".to_string(), Value::String("connected".to_string()));
            // 移除 "Current Wi-Fi Network: " 前缀
            let ssid = output_str.trim().replace("Current Wi-Fi Network: ", "");
            info.insert("ssid".to_string(), Value::String(ssid));
        }

        // 方法2: 获取 WiFi 接口信息
        if let Ok(output) = Command::new("networksetup")
            .args(["-getinfo", "Wi-Fi"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("Signal strength:") {
                    if let Some(value) = line.split(':').nth(1) {
                        info.insert("signal_strength".to_string(), Value::String(value.trim().to_string()));
                    }
                }
            }
        }

        Ok(Value::Object(info))
    }

    #[cfg(target_os = "linux")]
    {
        // 尝试使用 nmcli 获取 WiFi 信息
        let output = Command::new("nmcli")
            .args(["-t", "-f", "SSID,SIGNAL,ACTIVE", "dev", "wifi"])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut info = serde_json::Map::new();

        for line in output_str.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                info.insert("ssid".to_string(), Value::String(parts[0].to_string()));
                info.insert("signal".to_string(), Value::String(parts[1].to_string()));
                info.insert("active".to_string(), Value::String(parts[2].to_string()));
                break;
            }
        }

        if info.is_empty() {
            // 备选：使用 iwconfig
            if let Ok(output) = Command::new("iwconfig").arg("wlan0").output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("ESSID") {
                        if let Some(essid_start) = line.find("ESSID:\"") {
                            let essid_start = essid_start + 7;
                            if let Some(essid_end) = line[essid_start..].find('"') {
                                info.insert("ssid".to_string(), Value::String(line[essid_start..essid_start + essid_end].to_string()));
                            }
                        }
                    }
                }
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
                    "Signal" => info.insert(
                        "signal_strength".to_string(),
                        Value::String(value.to_string()),
                    ),
                    "Transmit rate (Mbps)" => {
                        info.insert("tx_rate".to_string(), Value::String(value.to_string()))
                    }
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
                    text: format!(
                        "📶 WiFi状态:\n{}",
                        serde_json::to_string_pretty(&wifi_info)?
                    ),
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