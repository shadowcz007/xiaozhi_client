use std::collections::HashMap;
use std::io::{self, Read};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use nokhwa::{Camera, utils::{CameraFormat, FrameFormat, CameraIndex, RequestedFormat, RequestedFormatType}, NokhwaError};
use nokhwa::pixel_format::RgbFormat;
use serde_json::{json, Value};
use xiaozhi_client::mcp::types::{Content, Tool, ToolsCallResult};
use xiaozhi_client::types::{ClientError, Result};

const API_ENDPOINT: &str = "http://localhost:1234/api/v1/chat";
const MODEL: &str = "google/gemma-4-e4b";

fn get_env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 定义工具
fn get_tool() -> Tool {
    Tool {
        name: "camera_vision".to_string(),
        description: "使用摄像头拍照并进行图像识别分析".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "camera_index": {
                    "type": "integer",
                    "description": "摄像头索引，默认为0（通常是内置摄像头）",
                    "default": 0
                },
                "prompt": {
                    "type": "string",
                    "description": "图像识别描述",
                    "default": "请详细描述这张图片，包括主要内容和场景"
                }
            }
        }),
    }
}

struct CameraError(NokhwaError);

impl From<CameraError> for ClientError {
    fn from(error: CameraError) -> Self {
        ClientError::Internal(error.0.to_string())
    }
}

fn init_camera(camera_index: i32) -> Result<Camera> {
    let index = CameraIndex::Index(camera_index as u32);
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
        CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)
    ));

    Camera::new(index, requested)
        .map_err(|e| CameraError(e).into())
}

async fn capture_image(camera: &mut Camera) -> Result<Vec<u8>> {
    camera.open_stream()
        .map_err(|e| CameraError(e))?;

    let frame = camera.frame()
        .map_err(|e| CameraError(e))?;

    let raw_image = frame.buffer().to_vec();

    camera.stop_stream()
        .map_err(|e| CameraError(e))?;

    Ok(raw_image)
}

async fn analyze_image(base64_image: String, prompt: &str) -> Result<String> {
    let api_token = get_env_or_default("LM_API_TOKEN", "lm-studio");
    let endpoint = get_env_or_default("VISION_API_ENDPOINT", API_ENDPOINT);
    let model = get_env_or_default("VISION_MODEL", MODEL);

    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": model,
            "input": [
                {
                    "type": "text",
                    "content": prompt
                },
                {
                    "type": "image",
                    "data_url": format!("data:image/png;base64,{}", base64_image)
                }
            ],
            "context_length": 2048,
            "temperature": 0
        }))
        .send()
        .await
        .map_err(|e| ClientError::Internal(format!("请求失败: {}", e)))?;

    let response_text = response.text().await?;

    let result: Value = serde_json::from_str(&response_text)
        .map_err(|e| ClientError::Internal(format!("JSON解析失败: {}", e)))?;

    // 尝试多种响应格式
    if let Some(content) = result.get("output")
        .or_else(|| result.get("content"))
        .or_else(|| result.pointer("/response/content"))
        .or_else(|| result.pointer("/choices/0/message/content"))
    {
        if let Some(text) = content.as_str() {
            return Ok(text.to_string());
        }
    }

    // 打印原始响应以便调试
    tracing::debug!("API响应: {}", response_text);

    Ok(format!("识别完成 (原始响应: {})", response_text))
}

async fn capture_and_analyze(camera_index: i32, prompt: &str) -> Result<String> {
    let mut camera = init_camera(camera_index)?;
    let raw_image = capture_image(&mut camera).await?;
    let base64_image = BASE64.encode(&raw_image);
    analyze_image(base64_image, prompt).await
}

async fn handle(arguments: Option<HashMap<String, Value>>) -> Result<ToolsCallResult> {
    let camera_index = arguments
        .as_ref()
        .and_then(|args| args.get("camera_index"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let prompt = arguments
        .as_ref()
        .and_then(|args| args.get("prompt"))
        .and_then(|v| v.as_str())
        .unwrap_or("请详细描述这张图片，包括主要内容、场景、人物和物体");

    match capture_and_analyze(camera_index, prompt).await {
        Ok(description) => Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("图像识别结果：{}", description),
            }],
            is_error: None,
        }),
        Err(e) => Ok(ToolsCallResult {
            content: vec![Content::Text {
                text: format!("错误：{}", e),
            }],
            is_error: Some(true),
        }),
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--get-tool" {
        let tool_def = get_tool();
        let json_output = serde_json::to_string_pretty(&tool_def).unwrap();
        println!("{}", json_output);
        return;
    }

    let mut buffer = Vec::new();
    if let Err(e) = io::stdin().read_to_end(&mut buffer) {
        eprintln!("读取标准输入失败: {}", e);
        std::process::exit(1);
    }

    let input_str = if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&buffer[3..]).into_owned()
    } else {
        String::from_utf8_lossy(&buffer).into_owned()
    };

    let cleaned_input = input_str.trim().replace('\r', "").replace('\n', "");

    let arguments: Option<HashMap<String, Value>> = if cleaned_input.is_empty() {
        None
    } else {
        match serde_json::from_str(&cleaned_input) {
            Ok(args) => Some(args),
            Err(e) => {
                eprintln!("JSON 解析错误: {}", e);
                eprintln!("清理后的输入: '{}'", cleaned_input);
                std::process::exit(1);
            }
        }
    };

    match handle(arguments).await {
        Ok(result) => {
            if let Ok(json_output) = serde_json::to_string(&result) {
                println!("{}", json_output);
            } else {
                eprintln!("结果序列化失败");
                std::process::exit(1);
            }
        }
        Err(e) => {
            let error_result = ToolsCallResult {
                content: vec![Content::Text { text: format!("Plugin Error: {}", e) }],
                is_error: Some(true),
            };
            if let Ok(json_output) = serde_json::to_string(&error_result) {
                println!("{}", json_output);
            }
            std::process::exit(1);
        }
    }
}