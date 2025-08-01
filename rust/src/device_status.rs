use crate::types::{DeviceStatusResponse, Result, ClientError};
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

/// 激活信息结构体
#[derive(Debug, Clone)]
pub struct ActivationInfo {
    pub device_id: String,
    pub device_name: String,
    pub challenge: Option<String>,
    pub activation_code: Option<String>,
    pub activation_message: Option<String>,
    pub server_message: Option<String>,
    pub server_code: Option<i64>,
}

/// 设备状态检查结果
#[derive(Debug)]
pub enum DeviceStatusResult {
    Activated(DeviceStatusResponse),
    NeedsActivation(ActivationInfo),
    NeedsActivationNoInfo,
}

/// 设备状态检查器
pub struct DeviceStatusChecker {
    client: HttpClient,
    ota_url: String,
}

impl DeviceStatusChecker {
    /// 创建新的设备状态检查器
    pub fn new() -> Self {
        Self {
            client: HttpClient::new(),
            ota_url: "https://api.tenclass.net/xiaozhi/ota/".to_string(),
        }
    }

    /// 创建带自定义OTA URL的检查器
    pub fn with_ota_url(ota_url: String) -> Self {
        Self {
            client: HttpClient::new(),
            ota_url,
        }
    }

    /// 检查设备状态
    /// 
    /// # Arguments
    /// * `device_id` - 设备ID
    /// 
    /// # Returns
    /// * `Ok(DeviceStatusResult::Activated(response))` - 设备已激活，返回配置信息
    /// * `Ok(DeviceStatusResult::NeedsActivation(info))` - 设备需要激活，返回激活信息
    /// * `Ok(DeviceStatusResult::NeedsActivationNoInfo)` - 设备需要激活，但无详细信息
    /// * `Err(error)` - 检查失败
    pub async fn check_device_status(&self, device_id: &str, name: &str) -> Result<DeviceStatusResult> {
        let client_id = Uuid::new_v4().to_string();
        
        // 准备请求头
        let mut headers = HashMap::new();
        headers.insert("Activation-Version", "2");
        headers.insert("Device-Id", device_id);
        headers.insert("Client-Id", &client_id);
        headers.insert("Content-Type", "application/json");
        let user_agent = format!("{}/1.0.0", name);
        headers.insert("User-Agent", &user_agent);

        // 准备请求数据
        let payload = json!({});

        tracing::debug!("检查设备状态: device_id={}, client_id={}", device_id, client_id);

        // 构建HTTP请求
        let mut request = self.client
            .post(&self.ota_url)
            .json(&payload);

        // 添加请求头
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // 发送请求
        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(ClientError::NetworkError(reqwest::Error::from(
                response.error_for_status().unwrap_err()
            )));
        }

        // 先获取状态码，因为 response.json() 会移动 response
        let status_code = response.status();
        let result: Value = response.json().await?;
        tracing::debug!("设备状态检查响应: {:?}", result);

        // 检查是否需要激活
        if let Some(activation) = result.get("activation") {
            // 检查 activation 是否为对象（包含激活信息）
            if activation.is_object() {
                tracing::info!("设备需要激活");
                println!("📝 检测到激活信息对象，设备需要激活");
                
                // 尝试获取激活相关的详细信息
                let challenge = activation.get("challenge").and_then(|v| v.as_str());
                let code = activation.get("code").and_then(|v| v.as_str());
                let message = activation.get("message").and_then(|v| v.as_str());
                
                return Ok(DeviceStatusResult::NeedsActivation(ActivationInfo {
                    device_id: device_id.to_string(),
                    device_name: name.to_string(),
                    challenge: challenge.map(|s| s.to_string()),
                    activation_code: code.map(|s| s.to_string()),
                    activation_message: message.map(|s| s.to_string()),
                    server_message: result.get("message").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    server_code: result.get("code").and_then(|v| v.as_i64()),
                }));
            } else if let Some(activation_bool) = activation.as_bool() {
                // 兼容旧的布尔值格式
                if activation_bool {
                    tracing::info!("设备需要激活");
                    println!("📝 检测到 activation: true，设备需要激活");
                    
                    return Ok(DeviceStatusResult::NeedsActivationNoInfo);
                }
            }
        }

        // 检查是否包含完整的配置信息（已激活设备应该包含这些字段）
        let has_websocket_config = result.get("websocket").is_some();
        let has_mqtt_config = result.get("mqtt").is_some();
        let has_firmware_config = result.get("firmware").is_some();

        println!("📋 配置字段检查:");
        println!("   - 包含 websocket 配置: {}", has_websocket_config);
        println!("   - 包含 mqtt 配置: {}", has_mqtt_config);
        println!("   - 包含 firmware 配置: {}", has_firmware_config);
        println!();

        // 只有包含完整配置信息的响应才被认为是已激活
        if has_websocket_config && has_mqtt_config && has_firmware_config {
            println!("✅ 检测到完整配置信息，尝试解析为已激活设备");
            // 尝试解析响应数据
            match serde_json::from_value::<DeviceStatusResponse>(result.clone()) {
                Ok(device_response) => {
                    tracing::info!("设备已激活，获取到配置信息");
                    println!("✅ 解析成功，设备已激活");
                    Ok(DeviceStatusResult::Activated(device_response))
                }
                Err(e) => {
                    tracing::warn!("解析设备配置失败: {}", e);
                    println!("❌ 解析失败: {}", e);
                    // 解析失败，但响应包含配置字段，可能是新设备
                    tracing::info!("设备需要激活（响应包含配置字段但解析失败）");
                    Ok(DeviceStatusResult::NeedsActivationNoInfo)
                }
            }
        } else {
            // 响应不包含完整的配置信息，说明设备需要激活
            tracing::info!("设备需要激活（响应不包含完整配置信息）");
            println!("❌ 响应不包含完整配置信息，设备需要激活");
            
            // 尝试获取服务器返回的错误信息
            let server_message = result.get("message").and_then(|v| v.as_str()).map(|s| s.to_string());
            let server_code = result.get("code").and_then(|v| v.as_i64());
            
            // 检查是否有激活相关的信息
            let activation_info = if let Some(activation_data) = result.get("activation") {
                let challenge = activation_data.get("challenge").and_then(|v| v.as_str());
                let code = activation_data.get("code").and_then(|v| v.as_str());
                let message = activation_data.get("message").and_then(|v| v.as_str());
                
                Some((challenge, code, message))
            } else {
                None
            };
            
            if let Some((challenge, code, message)) = activation_info {
                return Ok(DeviceStatusResult::NeedsActivation(ActivationInfo {
                    device_id: device_id.to_string(),
                    device_name: name.to_string(),
                    challenge: challenge.map(|s| s.to_string()),
                    activation_code: code.map(|s| s.to_string()),
                    activation_message: message.map(|s| s.to_string()),
                    server_message,
                    server_code,
                }));
            }
            
            Ok(DeviceStatusResult::NeedsActivationNoInfo)
        }
    }
}

impl Default for DeviceStatusChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_status_checker_creation() {
        let checker = DeviceStatusChecker::new();
        assert_eq!(checker.ota_url, "https://api.tenclass.net/xiaozhi/ota/");
    }

    #[tokio::test]
    async fn test_device_status_checker_with_custom_url() {
        let custom_url = "https://custom.example.com/ota/";
        let checker = DeviceStatusChecker::with_ota_url(custom_url.to_string());
        assert_eq!(checker.ota_url, custom_url);
    }
} 