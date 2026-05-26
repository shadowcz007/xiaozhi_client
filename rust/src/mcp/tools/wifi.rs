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
        let mut info = serde_json::Map::new();

        // 方法1: 使用 system_profiler 获取 WiFi 信息
        let output = Command::new("system_profiler")
            .args(["SPNetworkDataType", "-json"])
            .output()?;

        if output.status.success() {
            if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                if let Some(network) = json.get("SPNetworkDataType") {
                    if let Some(items) = network.as_array() {
                        for item in items {
                            if item.get("type").and_then(|t| t.as_str()) == Some("Wi-Fi") {
                                if let Some(interface) = item.get("interface") {
                                    info.insert("interface".to_string(), interface.clone());
                                }
                                if let Some(ssid) = item.get("ssid") {
                                    info.insert("ssid".to_string(), ssid.clone());
                                }
                                if let Some(signal) = item.get("signal_strength") {
                                    info.insert("signal_strength".to_string(), signal.clone());
                                }
                                if let Some(noise) = item.get("noise") {
                                    info.insert("noise".to_string(), noise.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // 方法2: 检查 WiFi 端口是否开启
        let airport_output = Command::new("networksetup")
            .args(["-getairportnetwork", "en0"])
            .output()?;
        let airport_str = String::from_utf8_lossy(&airport_output.stdout);

        if airport_str.contains("You are not associated") {
            // WiFi 可能断开或者用其他方式检测
            let _ = info.insert("status".to_string(), Value::String("unknown".to_string()));
        } else {
            let ssid = airport_str.trim().replace("Current Wi-Fi Network: ", "");
            info.insert("status".to_string(), Value::String("connected".to_string()));
            if !ssid.is_empty() && ssid != "On" {
                info.insert("ssid".to_string(), Value::String(ssid));
            }
        }

        // 方法3: 直接读取 airport 路径（如果存在）
        if info.get("ssid").is_none() {
            if let Ok(output) = Command::new("/usr/local/bin/airport")
                .arg("-I")
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("SSID:") {
                        if let Some(ssid) = line.split(':').nth(1) {
                            let ssid = ssid.trim();
                            if !ssid.is_empty() {
                                info.insert("ssid".to_string(), Value::String(ssid.to_string()));
                                info.insert("status".to_string(), Value::String("connected".to_string()));
                            }
                        }
                    }
                }
            }
        }

        // 如果还没有 ssid但有 IP，说明已连接
        if info.get("ssid").is_none() {
            let ip_output = Command::new("ipconfig")
                .args(["getifaddr", "en0"])
                .output()?;
            if ip_output.status.success() {
                let ip = String::from_utf8_lossy(&ip_output.stdout).trim().to_string();
                if !ip.is_empty() && ip != "0.0.0.0" {
                    info.insert("status".to_string(), Value::String("connected".to_string()));
                    info.insert("ip".to_string(), Value::String(ip));
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
            tracing::info!("📶 获取WiFi状态成功: {:?}", wifi_info);

            // 生成友好提示
            let status = wifi_info.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
            let ssid = wifi_info.get("ssid").and_then(|v| v.as_str());
            tracing::debug!("WiFi status: {}, ssid: {:?}", status, ssid);

            let text = if status == "disconnected" {
                "WiFi 未连接".to_string()
            } else if status == "connected" {
                if let Some(name) = ssid {
                    format!("已连接到 {}", name)
                } else {
                    "WiFi 已连接".to_string()
                }
            } else {
                // unknown 状态可能是：以太网连接、有线连接中、或 WiFi 未激活
                if let Some(ip) = wifi_info.get("ip").and_then(|v| v.as_str()) {
                    if !ip.is_empty() && ip != "0.0.0.0" {
                        return Ok(ToolsCallResult {
                            content: vec![Content::Text { text: "当前使用有线网络".to_string() }],
                            is_error: None,
                        });
                    }
                }
                "WiFi 未开启或未连接".to_string()
            };

            tracing::debug!("返回文本: {}", text);
            Ok(ToolsCallResult {
                content: vec![Content::Text { text }],
                is_error: None,
            })
        }
        Err(e) => {
            tracing::error!("📶 获取WiFi状态失败: {}", e);
            Ok(ToolsCallResult {
                content: vec![Content::Text {
                    text: "获取 WiFi 状态失败".to_string(),
                }],
                is_error: Some(true),
            })
        }
    }
}