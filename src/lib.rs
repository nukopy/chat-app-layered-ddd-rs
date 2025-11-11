//! WebSocket chat application library.
//!
//! This library provides server and client implementations for a WebSocket-based
//! chat application with broadcast functionality.

// layers
pub mod domain;
pub mod infrastructure;
pub mod ui;
pub mod usecase;

// shared library
pub mod common;
pub mod utils;
