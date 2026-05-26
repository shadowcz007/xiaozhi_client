use crate::ui::color::style;
use std::io::{self, Write};

pub struct InteractiveMenu;

impl InteractiveMenu {
    pub fn new() -> Self {
        Self
    }

    pub fn clear_screen(&self) {
        print!("\x1b[2J\x1b[H");
        io::stdout().flush().ok();
    }

    pub fn header(&self, title: &str) {
        println!();
        println!("{}", style::header(&"═".repeat(50)));
        println!("  {}", style::title(title));
        println!("{}", style::header(&"═".repeat(50)));
        println!();
    }

    pub fn footer(&self) {
        println!();
        println!("{}", style::muted(&"─".repeat(50)));
        println!();
    }

    pub fn menu_item(&self, num: &str, label: &str, description: &str) {
        println!(
            "  {} {}  {}",
            style::highlight(format!("[{}]", num)),
            style::info(label),
            style::muted(description)
        );
    }

    pub fn section(&self, title: &str) {
        println!();
        println!("  {}", style::title(title));
        println!("  {}", style::muted(&"─".repeat(40)));
    }

    pub fn prompt(&self, label: &str) -> String {
        print!("  {} ", style::highlight("▸"));
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        input.trim().to_string()
    }

    pub fn prompt_with_default(&self, label: &str, default: &str) -> String {
        print!("  {} ({}) ", style::highlight("▸"), style::muted(default));
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim();
        if input.is_empty() {
            default.to_string()
        } else {
            input.to_string()
        }
    }

    pub fn confirm(&self, label: &str) -> bool {
        print!("  {} ", style::highlight("▸"));
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes"
    }

    pub fn success(&self, message: &str) {
        println!();
        println!("  {} {}", style::activated("✓"), style::success(message));
        println!();
    }

    pub fn error(&self, message: &str) {
        println!();
        println!("  {} {}", style::deactivated("✗"), style::error(message));
        println!();
    }

    pub fn warning(&self, message: &str) {
        println!();
        println!("  {} {}", style::highlight("!"), style::warning(message));
        println!();
    }

    pub fn info(&self, message: &str) {
        println!("  {} {}", style::info("▸"), message);
    }

    pub fn loading(&self, message: &str) {
        print!("  {} {}", style::cyan("◐"), message);
        io::stdout().flush().ok();
    }

    pub fn loading_end(&self, success: bool, message: &str) {
        if success {
            println!("\r  {} {}", style::activated("✔"), style::success(message));
        } else {
            println!("\r  {} {}", style::deactivated("✗"), style::error(message));
        }
    }

    pub fn device_list_item(
        &self,
        num: usize,
        name: &str,
        device_id: &str,
        activated: bool,
        is_current: bool,
    ) {
        let status = if activated {
            style::activated("已激活")
        } else {
            style::deactivated("未激活")
        };

        let current_marker = if is_current {
            format!(" {} ", style::highlight("[当前]"))
        } else {
            "   ".to_string()
        };

        println!(
            "  {}{}{}  {}{}",
            style::highlight(format!("{}.", num)),
            current_marker,
            style::info(name),
            style::device_id(device_id),
            status
        );
    }

    pub fn device_detail(&self, _name: &str, key: &str, value: &str) {
        println!("  {} {}", style::muted(format!("{}:", key)), style::info(value));
    }

    pub fn activation_code(&self, code: &str, message: &str, url: &str) {
        println!();
        println!("{}", style::header(&"─".repeat(50)));
        println!();
        println!("  {} {}", style::warning("⚠"), style::warning("需要完成激活"));
        println!();
        println!("  {}  {}", style::info("提示:"), style::muted(message));
        println!();
        println!("  {}  {}", style::info("验证码:"), style::highlight(code));
        println!();
        println!("  {}  {}", style::info("网址:"), style::info(url));
        println!();
        println!("{}", style::header(&"─".repeat(50)));
    }
}

impl Default for InteractiveMenu {
    fn default() -> Self {
        Self::new()
    }
}