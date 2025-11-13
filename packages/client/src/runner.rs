//! Client execution logic with reconnection support.

use std::time::Duration;

use super::{error::ClientError, session::run_client_session};

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const RECONNECT_INTERVAL_SECS: u64 = 5;

/// Run the WebSocket client with reconnection logic
pub async fn run(url: String, client_id: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut reconnect_count = 0;

    loop {
        tracing::info!(
            "Attempting to connect to {} as '{}' (attempt {}/{})",
            url,
            client_id,
            reconnect_count + 1,
            MAX_RECONNECT_ATTEMPTS
        );

        match run_client_session(&url, &client_id).await {
            Ok(_) => {
                tracing::info!("Client session ended normally");
                // If connection ended normally (user exit), don't reconnect
                break;
            }
            Err(e) => {
                // Check if it's a duplicate client_id error
                if let Some(client_err) = e.downcast_ref::<ClientError>()
                    && matches!(client_err, ClientError::DuplicateClientId(_))
                {
                    tracing::error!("{}", e);
                    tracing::error!(
                        "Cannot connect with client_id '{}' as it is already in use. Exiting.",
                        client_id
                    );
                    std::process::exit(1);
                }

                tracing::warn!("Connection lost: {}", e);
                reconnect_count += 1;

                if reconnect_count >= MAX_RECONNECT_ATTEMPTS {
                    tracing::error!(
                        "Failed to reconnect after {} attempts. Exiting.",
                        MAX_RECONNECT_ATTEMPTS
                    );
                    std::process::exit(1);
                }

                tracing::info!(
                    "Reconnecting in {} seconds... (attempt {}/{})",
                    RECONNECT_INTERVAL_SECS,
                    reconnect_count + 1,
                    MAX_RECONNECT_ATTEMPTS
                );

                tokio::time::sleep(Duration::from_secs(RECONNECT_INTERVAL_SECS)).await;
            }
        }
    }

    Ok(())
}
