//! hello_world 插件示例
//!
//! 编译后，将可执行文件（例如 `hello_world_plugin.exe` 或 `hello_world_plugin`）
//! 放到主程序 `plugins` 目录下即可被加载。

use std::collections::HashMap;
use std::io::{self, Read};
use serde_json::Value;
use xiaozhi_client::mcp::types::{Content, Tool, ToolsCallResult};
use xiaozhi_client::types::Result;

/// 定义工具
fn get_tool() -> Tool {
    Tool {
        name: "hello_world".to_string(),
        description: "这是一个来自插件的 Hello World 示例工具，返回问候消息".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "要问候的名字",
                    "default": "Plugin"
                }
            }
        }),
    }
}

/// 执行工具
async fn handle(arguments: Option<HashMap<String, Value>>) -> Result<ToolsCallResult> {
    let name = arguments
        .as_ref()
        .and_then(|args| args.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Plugin");

    let greeting = format!("Hello, {}! 🎉 这条问候来自一个独立的插件!", name);

    Ok(ToolsCallResult {
        content: vec![Content::Text {
            text: greeting,
        }],
        is_error: None,
    })
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
        
        // 打印原始字节，看看实际收到了什么
        // println!("Debug - Raw bytes: {:?}", buffer);
        
        // 尝试检测和处理 BOM
        let input_str = if buffer.starts_with(&[0xEF, 0xBB, 0xBF]) {
            // 如果有 UTF-8 BOM，跳过它
            String::from_utf8_lossy(&buffer[3..]).into_owned()
        } else {
            String::from_utf8_lossy(&buffer).into_owned()
        };
        
        // 清理输入字符串，移除不可见字符
        let cleaned_input = input_str.trim().replace('\r', "").replace('\n', "");
        
        // println!("Debug - Cleaned input: '{}'", cleaned_input);

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