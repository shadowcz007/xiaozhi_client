use serde_json::Value;
use std::process::Command;

use crate::types::Result;
use super::types::{
    Content, Resource, ResourcesListParams, ResourcesReadParams,
    ResourcesListResult, ResourcesReadResult,
};


/// 获取随机笑话
fn get_random_joke() -> serde_json::Value {
    let jokes = vec![
        "为什么程序员总是分不清万圣节和圣诞节？因为 Oct 31 = Dec 25",
        "有一个程序员，他的问题是什么？他的女朋友是个10，但他只懂二进制。",
        "为什么程序员不喜欢大自然？因为那里有太多的bug。",
        "程序员最讨厌什么？写注释。",
        "什么是程序员最喜欢的食物？咖啡。因为它不需要编译就能运行。"
    ];
    
    let random_index = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as usize % jokes.len();
        
    serde_json::json!({
        "joke": jokes[random_index]
    })
}

/// 初始化资源列表
pub fn initialize_resources() -> Vec<Resource> {
    vec![
      
        Resource {
            uri: "device://joke/random".to_string(),
            name: "随机笑话".to_string(),
            description: Some("获取一条随机的程序员笑话".to_string()),
            mime_type: Some("application/json".to_string()),
        },
    ]
}

/// 处理资源列表请求
pub async fn handle_resources_list(
    id: Value, 
    _params: Option<ResourcesListParams>,
    resources: &Vec<Resource>
) -> Result<Option<serde_json::Value>> {
    let result = ResourcesListResult {
        resources: resources.clone(),
        next_cursor: None,
    };

    // 使用标准JSON-RPC 2.0格式
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });

    Ok(Some(response))
}

/// 处理资源读取请求
pub async fn handle_resources_read(id: Value, params: ResourcesReadParams) -> Result<Option<serde_json::Value>> {
    tracing::info!("📄 MCP资源读取: {}", params.uri);

    let contents = match params.uri.as_str() {
        "device://joke/random" => {
            let joke = get_random_joke();
            vec![Content::Text { 
                text: serde_json::to_string_pretty(&joke)?
            }]
        },
        _ => {
            // 使用标准JSON-RPC 2.0错误格式
            let error_response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32602,
                    "message": format!("未知资源: {}", params.uri)
                }
            });
            return Ok(Some(error_response));
        }
    };

    let result = ResourcesReadResult { contents };

    // 使用标准JSON-RPC 2.0格式
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });

    Ok(Some(response))
} 