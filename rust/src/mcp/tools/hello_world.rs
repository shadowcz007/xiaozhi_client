use std::collections::HashMap;
use serde_json::Value;
use crate::mcp::types::{Content, Tool, ToolsCallResult};
use crate::types::Result;

pub fn get_tool() -> Tool {
    Tool {
        name: "hello_world".to_string(),
        description: "Hello World示例工具，返回问候消息".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "要问候的名字",
                    "default": "World"
                }
            }
        }),
    }
}

pub async fn handle(arguments: Option<HashMap<String, Value>>) -> Result<ToolsCallResult> {
    let name = arguments
        .as_ref()
        .and_then(|args| args.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("World");

    let greeting = format!("Hello, {}! 这是来自设备的问候！🎉", name);
    tracing::info!("👋 Hello World工具调用: {}", greeting);

    Ok(ToolsCallResult {
        content: vec![Content::Text {
            text: greeting,
        }],
        is_error: None,
    })
} 