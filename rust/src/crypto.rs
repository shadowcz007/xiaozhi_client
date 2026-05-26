use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct LicenseKey {
    pub license: String,
    pub password: String,
}

pub struct LicenseVerifier {}

impl LicenseVerifier {
    pub fn new() -> Self {
        Self {}
    }

    // 解析 base64 编码的密钥
    pub fn decode_license_key(encoded_key: &str) -> Result<LicenseKey, Box<dyn Error>> {
        let decoded = BASE64
            .decode(encoded_key)
            .map_err(|e| format!("Base64 解码失败: {}", e))?;

        let json_str = String::from_utf8(decoded).map_err(|e| format!("字符串转换失败: {}", e))?;

        let license_key: LicenseKey =
            serde_json::from_str(&json_str).map_err(|e| format!("JSON 解析失败: {}", e))?;

        Ok(license_key)
    }

    // 获取默认许可证密钥（可通过环境变量配置）
    pub fn get_default_key() -> &'static str {
        // 通过 XIAOZHI_DEFAULT_LICENSE 环境变量配置
        // 格式: base64({"license":"xxx","password":"xxx"})
        option_env!("XIAOZHI_DEFAULT_LICENSE")
            .unwrap_or("eyJsaWNlbnNlIjoiam95LWNsaWVudC1taXhsYWItYnktc2hhZG93IiwicGFzc3dvcmQiOiJzaGFkb3ctbWl4bGFiLXBhc3N3b3JkMTIzIn0=")
    }

    pub fn verify_license(&self, license_key: &LicenseKey) -> Result<bool, Box<dyn Error>> {
        // 在开发环境中，允许使用测试许可证
        if cfg!(debug_assertions) {
            if license_key.license == "test-license" && license_key.password == "test-password" {
                println!("✅ 开发环境：使用测试许可证");
                return Ok(true);
            }
        }

        // 验证逻辑
        if license_key.license.len() < 5 || license_key.password.len() < 5 {
            return Ok(false);
        }

        let valid_combinations = vec![("joy-client-mixlab-by-shadow", "shadow-mixlab-password123")];

        for (valid_license, valid_password) in valid_combinations {
            if license_key.license == valid_license && license_key.password == valid_password {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

// 用于生成 base64 编码的密钥（用户输入格式）
pub fn generate_encoded_license(license: &str, password: &str) -> Result<String, Box<dyn Error>> {
    let license_key = LicenseKey {
        license: license.to_string(),
        password: password.to_string(),
    };

    let json_str = serde_json::to_string(&license_key)?;
    Ok(BASE64.encode(json_str))
}
