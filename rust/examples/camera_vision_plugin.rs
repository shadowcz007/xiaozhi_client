use std::collections::HashMap;
use std::io::{self, Read};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use nokhwa::{Camera, utils::{CameraFormat, FrameFormat, CameraIndex, RequestedFormat, RequestedFormatType}, NokhwaError};
use nokhwa::pixel_format::{RgbFormat};
use reqwest;
use serde_json::{json, Value};
use xiaozhi_client::mcp::types::{Content, Tool, ToolsCallResult};
use xiaozhi_client::types::{Result, ClientError};

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
        description: "使用摄像头拍照并进行图像识别分析".to_string(),
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

async fn capture_and_analyze(camera_index: i32) -> Result<String> {
    // println!("开始初始化摄像头，索引: {}", camera_index);
    
    // 初始化摄像头
    let index = CameraIndex::Index(camera_index as u32);
    let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(
        CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30)
    ));
    
    let mut camera = Camera::new(index, requested).map_err(|e| {
        println!("摄像头初始化失败: {:?}", e);
        CameraError(e)
    })?;

    // println!("摄像头初始化成功，准备打开视频流");

    // 打开摄像头
    camera.open_stream().map_err(|e| {
        println!("打开视频流失败: {:?}", e);
        CameraError(e)
    })?;

    // println!("视频流打开成功，准备捕获图像");

    // 捕获一帧
    let frame = camera.frame().map_err(|e| {
        println!("捕获图像失败: {:?}", e);
        CameraError(e)
    })?;
    let raw_image = frame.buffer().to_vec();
    // println!("成功捕获图像，大小: {} bytes", raw_image.len());

    // 关闭视频流并释放资源
    camera.stop_stream().map_err(|e| {
        // println!("关闭视频流失败: {:?}", e);
        CameraError(e)
    })?;
    
    // 显式释放摄像头资源
    drop(camera);
    // println!("摄像头资源已完全释放");

    // println!("准备进行base64编码");

    // 将图片转换为base64
    let base64_image = BASE64.encode(&raw_image);
    // println!("base64编码完成，长度: {}", base64_image.len());

    // println!("准备发送API请求");

    // 构建API请求
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.siliconflow.cn/v1/chat/completions")
        .header("Authorization", "Bearer sk-lvhkywnelbagndelvwljzhxqlornzodpmladzhochingimkw")
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

    // println!("API请求已发送，等待响应");

    // 获取API响应
    let response_text = response.text().await?;
    // println!("收到API响应: {}", response_text);

    // 解析 JSON
    let result: Value = match serde_json::from_str(&response_text) {
        Ok(v) => v,
        Err(e) => {
            println!("JSON解析失败: {:?}", e);
            return Err(ClientError::Internal(format!("JSON解析失败: {}", e)));
        }
    };
    
    // println!("开始提取识别结果");
    
    // 提取识别结果 - 更新解析路径
    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            println!("无法提取content内容，使用默认值");
            "无法解析识别结果".to_string()
        });

    // println!("最终识别结果: {}", content);
    Ok(content)
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
    } else {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer).unwrap();
        
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
                let json_output = serde_json::to_string(&result).unwrap();
                println!("{}", json_output);
            }
            Err(e) => {
                let error_result = ToolsCallResult {
                    content: vec![Content::Text { text: format!("Plugin Error: {}", e) }],
                    is_error: Some(true),
                };
                let json_output = serde_json::to_string(&error_result).unwrap();
                println!("{}", json_output);
                std::process::exit(1);
            }
        }
    }
} 