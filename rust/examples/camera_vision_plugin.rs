use std::collections::HashMap;
use std::io::{self, Read};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use nokhwa::{Camera, utils::{CameraFormat, FrameFormat, CameraIndex, RequestedFormat, RequestedFormatType}, NokhwaError};
use nokhwa::pixel_format::{RgbFormat};
use reqwest;
use serde_json::{json, Value};
use xiaozhi_client::mcp::types::{Content, Tool, ToolsCallResult};
use xiaozhi_client::types::{Result, ClientError};

const API_ENDPOINT: &str = "https://api.siliconflow.cn/v1/chat/completions";
const API_KEY: &str = "Bearer sk-lvhkywnelbagndelvwljzhxqlornzodpmladzhochingimkw";

// 包装 NokhwaError
#[derive(Debug)]
struct CameraError(NokhwaError);

impl From<CameraError> for ClientError {
    fn from(error: CameraError) -> Self {
        ClientError::Internal(error.0.to_string())
    }
}

/// 定义工具
fn get_tool() -> Tool {
    Tool {
        name: "camera_vision".to_string(),
        description: "使用摄像头拍照,自拍,并进行图像识别分析".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "camera_index": {
                    "type": "integer",
                    "description": "摄像头索引，默认为0（通常是内置摄像头）",
                    "default": 0
                }
            }
        }),
    }
}

/// 初始化并获取摄像头实例
fn init_camera(camera_index: i32) -> Result<Camera> {
    let index = CameraIndex::Index(camera_index as u32);
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
        CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)
    ));
    
    Camera::new(index, requested)
        .map_err(|e| {
            eprintln!("摄像头初始化失败: {:?}", e);
            CameraError(e).into()
        })
}

/// 捕获图像
async fn capture_image(camera: &mut Camera) -> Result<Vec<u8>> {
    camera.open_stream()
        .map_err(|e| {
            eprintln!("打开视频流失败: {:?}", e);
            CameraError(e)
        })?;

    let frame = camera.frame()
        .map_err(|e| {
            eprintln!("捕获图像失败: {:?}", e);
            CameraError(e)
        })?;

    let raw_image = frame.buffer().to_vec();

    camera.stop_stream()
        .map_err(|e| {
            eprintln!("关闭视频流失败: {:?}", e);
            CameraError(e)
        })?;

    Ok(raw_image)
}

/// 发送API请求进行图像分析
async fn analyze_image(base64_image: String) -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .post(API_ENDPOINT)
        .header("Authorization", API_KEY)
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "THUDM/GLM-4.1V-9B-Thinking",
            "stream": false,
            "max_tokens": 512,
            "min_p": 0.05,
            "temperature": 0.7,
            "top_p": 0.7,
            "top_k": 50,
            "frequency_penalty": 0.5,
            "n": 1,
            "stop": [],
            "messages": [
                {
                    "role": "system",
                    "content": "请用100字以内详细描述这张图片。包括：1. 主要内容和场景 2. 人物的数量、外貌、表情、动作 3. 场景的氛围和光线 4. 重要的物体和细节 5. 画面的构图和视角。使用简洁清晰的语言。"
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "image_url": {
                                "detail": "auto",
                                "url": format!("data:image/jpeg;base64,{}", base64_image)
                            },
                            "type": "image_url"
                        }
                    ]
                }
            ]
        }))
        .send()
        .await?;

    let response_text = response.text().await?;
    
    let result: Value = serde_json::from_str(&response_text)
        .map_err(|e| ClientError::Internal(format!("JSON解析失败: {}", e)))?;
    
    Ok(result["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "无法解析识别结果".to_string()))
}

async fn capture_and_analyze(camera_index: i32) -> Result<String> {
    let mut camera = init_camera(camera_index)?;
    let raw_image = capture_image(&mut camera).await?;
    let base64_image = BASE64.encode(&raw_image);
    analyze_image(base64_image).await
}

/// 执行工具
async fn handle(arguments: Option<HashMap<String, Value>>) -> Result<ToolsCallResult> {
    let camera_index = arguments
        .as_ref()
        .and_then(|args| args.get("camera_index"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    match capture_and_analyze(camera_index).await {
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