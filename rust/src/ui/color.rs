// 彩色输出模块
use std::fmt;

pub struct Color {
    pub red: fn(&str) -> String,
    pub green: fn(&str) -> String,
    pub yellow: fn(&str) -> String,
    pub blue: fn(&str) -> String,
    pub cyan: fn(&str) -> String,
    pub magenta: fn(&str) -> String,
    pub bold: fn(&str) -> String,
    pub reset: fn(&str) -> String,
}

impl Color {
    pub fn new() -> Self {
        Self {
            red: |s| format!("\x1b[31m{}\x1b[0m", s),
            green: |s| format!("\x1b[32m{}\x1b[0m", s),
            yellow: |s| format!("\x1b[33m{}\x1b[0m", s),
            blue: |s| format!("\x1b[34m{}\x1b[0m", s),
            cyan: |s| format!("\x1b[36m{}\x1b[0m", s),
            magenta: |s| format!("\x1b[35m{}\x1b[0m", s),
            bold: |s| format!("\x1b[1m{}\x1b[0m", s),
            reset: |s| format!("\x1b[0m{}", s),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ColorCode {
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,
}

pub fn colored<S: AsRef<str>>(text: S, color: ColorCode) -> String {
    format!("\x1b[{}m{}\x1b[0m", color as u8, text.as_ref())
}

pub fn bold<S: AsRef<str>>(text: S) -> String {
    format!("\x1b[1m{}\x1b[0m", text.as_ref())
}

// 常用样式
pub mod style {
    use super::*;

    pub fn success<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Green)
    }

    pub fn error<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Red)
    }

    pub fn warning<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Yellow)
    }

    pub fn info<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Cyan)
    }

    pub fn header<S: AsRef<str>>(text: S) -> String {
        bold(colored(text, ColorCode::Magenta))
    }

    pub fn title<S: AsRef<str>>(text: S) -> String {
        bold(colored(text, ColorCode::Blue))
    }

    pub fn muted<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::White)
    }

    pub fn highlight<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Yellow)
    }

    pub fn device_id<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Cyan)
    }

    pub fn activated<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Green)
    }

    pub fn deactivated<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Red)
    }

    pub fn cyan<S: AsRef<str>>(text: S) -> String {
        colored(text, ColorCode::Cyan)
    }
}