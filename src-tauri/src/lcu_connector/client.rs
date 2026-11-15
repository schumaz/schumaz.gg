use irelia::rest::LcuClient;

pub type DefaultRequestClient = hyper_util::client::legacy::Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    http_body_util::Full<bytes::Bytes>,
>;

pub type ConnectedLcuClient = LcuClient<DefaultRequestClient>;

/// High-level struct managing the REST connection to the League Client.
pub struct LcuConnection {
    client: ConnectedLcuClient,
}

impl LcuConnection {
    /// Attempts to locate and connect to the active League Client process (LeagueClientUx.exe).
    pub fn connect() -> Result<Self, String> {
        // Ensure rustls has a default crypto provider installed to avoid runtime ambiguity
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        LcuClient::connect()
            .map(|client| Self { client })
            .map_err(|err| format!("Failed to connect to LCU process: {:?}", err))
    }

    pub(super) fn raw_client(&self) -> &ConnectedLcuClient {
        &self.client
    }

    pub(super) fn is_eof_error(err_str: &str) -> bool {
        err_str.contains("unexpected end of file")
            || err_str.contains("Unexpected end of file")
            || err_str.contains("Eof")
            || err_str.contains("EOF")
    }

    pub(super) async fn get_json_opt(
        &self,
        endpoint: &str,
        context: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        self.client
            .get(endpoint)
            .await
            .map_err(|err| format!("Error requesting {}: {:?}", context, err))
    }

    pub(super) async fn get_json_required(
        &self,
        endpoint: &str,
        context: &str,
    ) -> Result<serde_json::Value, String> {
        let response = self.get_json_opt(endpoint, context).await?;
        response.ok_or_else(|| format!("LCU returned an empty response for {}", context))
    }

    pub(super) async fn post_json_required(
        &self,
        endpoint: &str,
        payload: &serde_json::Value,
        err_prefix: &str,
        empty_err: &str,
    ) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .post(endpoint, payload)
            .await
            .map_err(|err| format!("{}: {:?}", err_prefix, err))?;

        response.ok_or_else(|| empty_err.to_string())
    }

    pub(super) async fn put_json_required(
        &self,
        endpoint: &str,
        payload: &serde_json::Value,
        err_prefix: &str,
        empty_err: &str,
    ) -> Result<serde_json::Value, String> {
        let response: Option<serde_json::Value> = self
            .client
            .put(endpoint, payload)
            .await
            .map_err(|err| format!("{}: {:?}", err_prefix, err))?;

        response.ok_or_else(|| empty_err.to_string())
    }

    pub(super) async fn put_json_ignoring_eof(
        &self,
        endpoint: &str,
        payload: &serde_json::Value,
        err_prefix: &str,
    ) -> Result<(), String> {
        match self
            .client
            .put::<&serde_json::Value, Option<serde_json::Value>>(endpoint, payload)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if Self::is_eof_error(&err_str) {
                    Ok(())
                } else {
                    Err(format!("{}: {}", err_prefix, err_str))
                }
            }
        }
    }

    pub(super) async fn delete_ignoring_eof(
        &self,
        endpoint: &str,
        err_prefix: &str,
    ) -> Result<(), String> {
        match self
            .client
            .delete::<Option<serde_json::Value>>(endpoint)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                let err_str = format!("{:?}", err);
                if Self::is_eof_error(&err_str) {
                    Ok(())
                } else {
                    Err(format!("{}: {}", err_prefix, err_str))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_lcu_handles_closed_client_gracefully() {
        // Unless LeagueClientUx.exe is actively running on this machine during test execution,
        // LcuConnection::connect should return an Err result without panicking.
        let connection_res = LcuConnection::connect();
        match connection_res {
            Ok(_) => println!("LCU process was detected during test execution."),
            Err(err) => {
                assert!(!err.is_empty());
                println!(
                    "LCU was not detected (expected when client is closed): {}",
                    err
                );
            }
        }
    }
}
