use crate::types::{DeviceStatusResponse, Result, ClientError};
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

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
    /// * `Ok(Some(response))` - 设备已激活，返回配置信息
    /// * `Ok(None)` - 设备需要激活
    /// * `Err(error)` - 检查失败
    pub async fn check_device_status(&self, device_id: &str,name: &str) -> Result<Option<DeviceStatusResponse>> {
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

        let result: Value = response.json().await?;
        tracing::debug!("设备状态检查响应: {:?}", result);

        // 检查是否需要激活
        if let Some(activation) = result.get("activation").and_then(|v| v.as_bool()) {
            if activation {
                tracing::info!("设备需要激活");
                return Ok(None);
            }
        }

        // 解析响应数据
        let device_response: DeviceStatusResponse = serde_json::from_value(result)?;
        tracing::info!("设备已激活，获取到配置信息");
        
        Ok(Some(device_response))
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