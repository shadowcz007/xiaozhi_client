use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::mcp::types::{Tool, ToolsCallResult};
use crate::types::{ClientError, Result};

#[derive(Debug, Clone)]
pub struct Plugin {
    pub tool: Tool,
    pub path: PathBuf,
}

/// 扫描并加载插件
pub fn load_plugins() -> Result<Vec<Plugin>> {
    let mut plugins = Vec::new();
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or_else(|| ClientError::Internal("无法获取可执行文件目录".to_string()))?;
    let plugins_dir = exe_dir.join("plugins");

    tracing::info!("🔌 扫描插件目录: {:?}", plugins_dir);

    if !plugins_dir.exists() {
        if let Err(e) = fs::create_dir_all(&plugins_dir) {
            tracing::error!("❌ 创建插件目录失败: {:?}, 错误: {}", plugins_dir, e);
            return Ok(plugins);
        }
    }

    // 扫描插件目录
    for entry in fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // 尝试加载插件
            match load_plugin_tool(&path) {
                Ok(tool) => {
                    tracing::info!("✅ 加载插件成功: {:?}", path);
                    plugins.push(Plugin {
                        tool,
                        path: path.to_owned(),
                    });
                }
                Err(e) => {
                    tracing::error!("❌ 加载插件失败 {:?}: {}", path, e);
                }
            }
        }
    }

    tracing::info!("✨ 共加载了 {} 个插件", plugins.len());
    Ok(plugins)
}

/// 加载插件的工具定义
fn load_plugin_tool(path: &Path) -> Result<Tool> {
    let output = Command::new(path)
        .arg("--get-tool")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ClientError::Internal(format!("插件返回错误: {}", error)));
    }

    let tool: Tool = serde_json::from_slice(&output.stdout)?;
    Ok(tool)
}

/// 执行插件
pub async fn execute_plugin(
    plugin: &Plugin,
    arguments: Option<HashMap<String, Value>>,
) -> Result<ToolsCallResult> {
    let mut child = Command::new(&plugin.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // 写入参数
    if let Some(args) = arguments {
        if let Some(mut stdin) = child.stdin.take() {
            let input = serde_json::to_string(&args)?;
            stdin.write_all(input.as_bytes())?;
        }
    }

    // 获取输出
    let output = child.wait_with_output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ClientError::Internal(format!("插件执行错误: {}", error)));
    }

    let result: ToolsCallResult = serde_json::from_slice(&output.stdout)?;
    Ok(result)
} 