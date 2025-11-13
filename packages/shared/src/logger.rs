//! Logging setup utilities for the WebSocket chat application.

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the tracing subscriber with the specified default log level.
///
/// This function sets up logging for both the application crate and the binary.
/// The log level can be overridden using the `RUST_LOG` environment variable.
///
/// # Arguments
///
/// * `binary_name` - The name of the binary (e.g., "server", "client")
/// * `default_level` - The default log level (e.g., "debug", "info", "warn", "error")
///
/// # Examples
///
/// ```no_run
/// use shared::logger::setup_logger;
///
/// setup_logger("server", "debug");
/// ```
pub fn setup_logger(binary_name: &str, default_log_level: &str) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}={},{}={}",
                    env!("CARGO_PKG_NAME").replace("-", "_"),
                    default_log_level,
                    binary_name,
                    default_log_level
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
