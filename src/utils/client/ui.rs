//! UI utilities for the client.

use std::io::Write;

/// Redisplay the prompt after receiving a message
pub fn redisplay_prompt(client_id: &str) {
    print!("{}> ", client_id);
    std::io::stdout().flush().ok();
}
