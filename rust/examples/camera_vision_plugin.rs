use std::collections::HashMap;
use std::io::{self, Read, Cursor, Write};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use nokhwa::{Camera, utils::{CameraFormat, FrameFormat, CameraIndex, RequestedFormat, RequestedFormatType}, NokhwaError};
use nokhwa::pixel_format::RgbFormat;
use serde_json::{json, Value};
use xiaozhi_client::mcp::types::{Content, Tool, ToolsCallResult};
use xiaozhi_client::types::{ClientError, Result};

fn convert_yuyv_to_rgb(width: u32, height: u32, yuyv_data: &[u8]) -> Vec<u8> {
    let pixel_count = (width * height) as usize;
    let mut rgb_data = Vec::with_capacity(pixel_count * 3);

    // YUYV: 4 bytes per 2 pixels (Y0 U Y1 V)
    let max_pixels = yuyv_data.len() / 4;

    for i in 0..max_pixels {
        let y_idx = i * 4;
        if y_idx + 3 >= yuyv_data.len() {
            break;
        }

        let y0 = yuyv_data[y_idx] as f32;
        let y1 = yuyv_data[y_idx + 1] as f32;
        let u = yuyv_data[y_idx + 2] as f32 - 128.0;
        let v = yuyv_data[y_idx + 3] as f32 - 128.0;

        // 像素 1
        let r0 = (y0 + 1.402 * v).clamp(0.0, 255.0) as u8;
        let g0 = (y0 - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
        let b0 = (y0 + 1.772 * u).clamp(0.0, 255.0) as u8;
        rgb_data.push(r0);
        rgb_data.push(g0);
        rgb_data.push(b0);

        // 像素 2
        let r1 = (y1 + 1.402 * v).clamp(0.0, 255.0) as u8;
        let g1 = (y1 - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
        let b1 = (y1 + 1.772 * u).clamp(0.0, 255.0) as u8;
        rgb_data.push(r1);
        rgb_data.push(g1);
        rgb_data.push(b1);
    }
    rgb_data
}

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
    // 尝试多种分辨率和格式
    let formats = vec![
        (FrameFormat::MJPEG, 640, 480, 30),
        (FrameFormat::YUYV, 640, 480, 30),
        (FrameFormat::NV12, 640, 480, 30),
        (FrameFormat::MJPEG, 1280, 720, 30),
    ];

    let mut last_error = None;
    for (format, width, height, fps) in formats {
        let index = CameraIndex::Index(camera_index as u32);
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
            CameraFormat::new_from(width, height, format, fps)
        ));
        match Camera::new(index, requested) {
            Ok(cam) => return Ok(cam),
            Err(e) => {
                last_error = Some(e);
                continue;
            }
        }
    }

    Err(last_error.map(|e| CameraError(e).into()).unwrap_or_else(|| {
        ClientError::Internal("无法打开摄像头".to_string()).into()
    }))
}

async fn capture_image(camera: &mut Camera) -> Result<Vec<u8>> {
    camera.open_stream()
        .map_err(|e| CameraError(e))?;

    let frame = camera.frame()
        .map_err(|e| CameraError(e))?;

    let resolution = frame.resolution();
    let width = resolution.width_x;
    let height = resolution.height_y;
    let frame_format = frame.source_frame_format();
    let buffer = frame.buffer();

    let rgb_data = match frame_format {
        FrameFormat::YUYV => {
            let yuyv_data = buffer.to_vec();
            convert_yuyv_to_rgb(width, height, &yuyv_data)
        }
        FrameFormat::GRAY => {
            buffer.to_vec()
        }
        FrameFormat::RAWRGB | FrameFormat::RAWBGR => {
            buffer.to_vec()
        }
        _ => {
            buffer.to_vec()
        }
    };

    // 转换为 JPEG 格式
    let mut jpeg_data = Vec::new();
    let img = image::RgbImage::from_raw(width, height, rgb_data)
        .ok_or_else(|| ClientError::Internal("无法创建图像".to_string()))?;

    let mut cursor = Cursor::new(&mut jpeg_data);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| ClientError::Internal(format!("JPEG编码失败: {}", e)))?;

    camera.stop_stream()
        .map_err(|e| CameraError(e))?;

    Ok(jpeg_data)
}

async fn analyze_image(jpeg_data: Vec<u8>, prompt: &str) -> Result<String> {
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
                    "data_url": format!("data:image/jpeg;base64,{}", BASE64.encode(&jpeg_data))
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

    if let Some(content) = result.get("output")
        .or_else(|| result.get("content"))
        .or_else(|| result.pointer("/response/content"))
        .or_else(|| result.pointer("/choices/0/message/content"))
    {
        if let Some(text) = content.as_str() {
            return Ok(text.to_string());
        }
    }

    tracing::debug!("API响应: {}", response_text);
    Ok(format!("识别完成 (原始响应: {})", response_text))
}

async fn capture_and_analyze(camera_index: i32, prompt: &str) -> Result<String> {
    let mut camera = init_camera(camera_index)?;
    let jpeg_data = capture_image(&mut camera).await?;
    analyze_image(jpeg_data, prompt).await
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