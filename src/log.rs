//! Logger utilities for veri-easy.

use clap::ValueEnum;
use colored::Colorize;
use std::sync::OnceLock;

/// Logging level.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    /// Brief logging, including components and check results.
    Brief,
    /// Normal logging, including all brief logs and checker states.
    Normal,
    /// Verbose logging, including all normal logs and detailed messages from components.
    Verbose,
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "brief" => LogLevel::Brief,
            "normal" => LogLevel::Normal,
            "verbose" => LogLevel::Verbose,
            _ => LogLevel::Normal,
        }
    }
}

/// Message type.
#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    /// Simple message.
    Simple,
    /// Informational message.
    Info,
    /// Critical information message.
    Critical,
    /// Warning message.
    Warning,
    /// Unsure check result (usually by a formal component).
    Unsure,
    /// Error message.
    Error,
    /// Ok message.
    Ok,
}

/// Logger structure.
#[derive(Debug)]
pub struct Logger {
    /// Logger level.
    level: LogLevel,
}

impl Logger {
    /// Create a new logger.
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }

    /// Get the format string for a message type.
    fn format_msg(&self, msg_type: MessageType, msg: &str) -> String {
        let pref = match msg_type {
            MessageType::Simple => "".to_string(),
            MessageType::Info => "[Info] ".blue().bold().to_string(),
            MessageType::Critical => "[Critical] ".cyan().bold().to_string(),
            MessageType::Warning => "[Warning] ".yellow().bold().to_string(),
            MessageType::Unsure => "[Unsure] ".magenta().bold().to_string(),
            MessageType::Error => "[Error] ".red().bold().to_string(),
            MessageType::Ok => "[Ok] ".green().bold().to_string(),
        };
        format!("{}{}", pref, msg)
    }

    /// Log a message if the level is sufficient.
    pub fn log(&self, level: LogLevel, msg_type: MessageType, msg: &str) {
        if (self.level as u8) >= (level as u8) {
            println!("{}", self.format_msg(msg_type, msg));
        }
    }
}

/// Global logger instance.
static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Initialize the global logger.
pub fn init_logger(level: LogLevel) {
    LOGGER.set(Logger::new(level)).unwrap();
}

/// Get the global logger.
pub fn get_logger() -> &'static Logger {
    LOGGER.get().expect("Logger not initialized")
}

/// Log a message using the global logger.
#[macro_export]
macro_rules! log {
    ($level:ident, $msg_type:ident, $msg:expr) => {
        $crate::log::get_logger().log(
            $crate::log::LogLevel::$level, $crate::log::MessageType::$msg_type, $msg)
    };
    ($level:ident, $msg_type:ident, $fmt:expr, $($arg:tt)*) => {
        $crate::log::get_logger().log(
            $crate::log::LogLevel::$level, $crate::log::MessageType::$msg_type, &format!($fmt, $($arg)*))
    };
    // If no level or type is specified, default to Normal and Simple
    ($msg:expr) => {
        $crate::log::get_logger().log(
            $crate::log::LogLevel::Normal, $crate::log::MessageType::Simple, $msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::log::get_logger().log(
            $crate::log::LogLevel::Normal, $crate::log::MessageType::Simple, &format!($fmt, $($arg)*))
    };
}
