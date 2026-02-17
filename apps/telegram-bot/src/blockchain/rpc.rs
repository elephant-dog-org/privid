use reqwest::Client;
use serde_json::{json, Value};
use thiserror::Error;
use log::{debug, warn};
use tokio::time::{sleep, Duration};

/// Errors that can occur during JSON-RPC communication.
#[derive(Debug, Error)]
pub enum RpcError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON-RPC error (code {code}): {message}")]
    JsonRpc { code: i64, message: String },

    #[error("Missing 'result' field in JSON-RPC response")]
    MissingResult,

    #[error("Invalid hex in response: {0}")]
    HexDecode(#[from] hex::FromHexError),

    #[error("Unexpected response format: {0}")]
    UnexpectedFormat(String),
}

impl RpcError {
    /// Returns true if this error is transient and the request should be retried.
    ///
    /// Network errors, timeouts, and connection resets are transient.
    /// JSON-RPC errors and decoding errors are not.
    pub fn is_transient(&self) -> bool {
        match self {
            RpcError::Http(e) => {
                e.is_timeout() || e.is_connect() || e.is_request()
            }
            _ => false,
        }
    }
}

/// JSON-RPC client for Ethereum `eth_call` requests.
///
/// Wraps a `reqwest::Client` and an RPC endpoint URL to provide
/// typed access to Ethereum contract read methods.
#[derive(Debug, Clone)]
pub struct RpcClient {
    client: Client,
    rpc_url: String,
}

impl RpcClient {
    /// Create a new RPC client for the given endpoint URL.
    pub fn new(rpc_url: String) -> Self {
        Self {
            client: Client::new(),
            rpc_url,
        }
    }

    /// Execute an `eth_call` against the given contract address.
    ///
    /// # Arguments
    /// - `to`: Contract address as a hex string (e.g., "0x2AA8...")
    /// - `data`: ABI-encoded calldata bytes
    ///
    /// # Returns
    /// The hex-decoded response bytes from the contract call.
    pub async fn eth_call(&self, to: &str, data: &[u8]) -> Result<Vec<u8>, RpcError> {
        let data_hex = format!("0x{}", hex::encode(data));

        let request_body = json!({
            "jsonrpc": "2.0",
            "method": "eth_call",
            "params": [
                {
                    "to": to,
                    "data": data_hex
                },
                "latest"
            ],
            "id": 1
        });

        debug!("eth_call to={} data_len={}", to, data.len());

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request_body)
            .send()
            .await?;

        let body: Value = response.json().await?;

        // Check for JSON-RPC error
        if let Some(error) = body.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
                .to_string();
            return Err(RpcError::JsonRpc { code, message });
        }

        // Extract result
        let result_hex = body
            .get("result")
            .and_then(|r| r.as_str())
            .ok_or(RpcError::MissingResult)?;

        // Strip "0x" prefix and decode hex
        let clean = result_hex.strip_prefix("0x").unwrap_or(result_hex);

        // Handle empty response (e.g., "0x")
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let bytes = hex::decode(clean)?;
        Ok(bytes)
    }

    /// Execute an `eth_call` with automatic retry and exponential backoff.
    ///
    /// Only retries on transient errors (network failures, timeouts).
    /// Non-transient errors (JSON-RPC errors, decoding errors) fail immediately.
    ///
    /// # Arguments
    /// - `to`: Contract address
    /// - `data`: ABI-encoded calldata
    /// - `max_retries`: Maximum number of retry attempts (0 means try once with no retries)
    ///
    /// # Backoff
    /// Base delay is 500ms, doubling each retry: 500ms, 1s, 2s, 4s, ...
    pub async fn eth_call_with_retry(
        &self,
        to: &str,
        data: &[u8],
        max_retries: u32,
    ) -> Result<Vec<u8>, RpcError> {
        let mut last_error: Option<RpcError> = None;
        let base_delay_ms: u64 = 500;

        for attempt in 0..=max_retries {
            match self.eth_call(to, data).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_transient() || attempt == max_retries {
                        return Err(e);
                    }

                    let delay = base_delay_ms * 2u64.pow(attempt);
                    warn!(
                        "eth_call attempt {} failed (transient): {}. Retrying in {}ms...",
                        attempt + 1,
                        e,
                        delay
                    );
                    last_error = Some(e);
                    sleep(Duration::from_millis(delay)).await;
                }
            }
        }

        // This should be unreachable due to the loop logic, but just in case
        Err(last_error.unwrap_or_else(|| {
            RpcError::UnexpectedFormat("retry loop exited without result or error".to_string())
        }))
    }

    /// Returns the RPC URL this client is configured to use.
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_error_is_transient() {
        // JSON-RPC errors are not transient
        let rpc_err = RpcError::JsonRpc {
            code: -32000,
            message: "execution reverted".to_string(),
        };
        assert!(!rpc_err.is_transient());

        // Missing result is not transient
        assert!(!RpcError::MissingResult.is_transient());

        // Unexpected format is not transient
        assert!(!RpcError::UnexpectedFormat("test".to_string()).is_transient());
    }

    #[test]
    fn test_rpc_client_new() {
        let client = RpcClient::new("https://example.com/rpc".to_string());
        assert_eq!(client.rpc_url(), "https://example.com/rpc");
    }

    #[test]
    fn test_rpc_error_display() {
        let err = RpcError::JsonRpc {
            code: -32000,
            message: "execution reverted".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("-32000"));
        assert!(display.contains("execution reverted"));
    }
}
