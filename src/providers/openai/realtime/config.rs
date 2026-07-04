use http::{HeaderMap, HeaderValue};
use http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;

use crate::core::error::{ProtocolError, Result};

pub struct OpenAIRealtimeConfig {
    pub api_key: String,
    pub model: String,

    pub endpoint: String,

    pub organization: Option<String>,
    pub project: Option<String>,

    pub headers: HeaderMap,
}

impl OpenAIRealtimeConfig {
    pub fn request(&self) -> Result<Request> {
        let mut request = format!(
            "{}?model={}",
            self.endpoint,
            self.model,
        )
            .into_client_request()
            .map_err(|e| ProtocolError::Other(e.to_string()))?;

        {
            let headers = request
                .headers_mut();

            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(
                    &format!("Bearer {}", self.api_key)
                )
                    .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
            );

            headers.insert(
                "OpenAI-Beta",
                HeaderValue::from_static("realtime=v1"),
            );

            if let Some(org) = &self.organization {
                headers.insert(
                    "OpenAI-Organization",
                    HeaderValue::from_str(org)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }

            if let Some(project) = &self.project {
                headers.insert(
                    "OpenAI-Project",
                    HeaderValue::from_str(project)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }

            headers.extend(self.headers.clone());
        }

        Ok(request)
    }
}
