//! Remipedia - IoT Health Platform Backend
//!
//! 一个高效优雅的 Rust 后端服务，用于 IoT 健康数据平台。

pub mod api;
pub mod config;
pub mod core;
pub mod dto;
pub mod errors;
pub mod ingest;
pub mod repository;
pub mod service;
pub mod utils;

pub use config::Settings;
pub use errors::{AppError, AppResult};