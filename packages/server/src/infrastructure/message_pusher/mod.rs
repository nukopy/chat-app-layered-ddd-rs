//! メッセージ送信（通知）の実装
//!
//! ## 概要
//!
//! このモジュールは `MessagePusher` trait の具体的な実装を提供します。
//!
//! ## 実装
//!
//! - `websocket`: WebSocket を使った実装
//! - 将来的に: `redis`, `kafka` など

pub mod websocket;

pub use websocket::WebSocketMessagePusher;
