use http::{HeaderMap, HeaderValue, Request};
use http::header::AUTHORIZATION;
use crate::core::error::{CoreError, ProtocolError, Result};

pub struct OpenAIRealtimeConfig {
    pub api_key: String,
    pub model: String,

    pub endpoint: String,

    pub organization: Option<String>,
    pub project: Option<String>,

    pub headers: HeaderMap,
}

impl OpenAIRealtimeConfig {
    pub fn request(&self) -> Result<Request<()>> {
        let mut builder = Request::builder()
            .method("GET")
            .uri(format!(
                "{}?model={}",
                self.endpoint,
                self.model
            ));

        {
            let headers = builder
                .headers_mut()
                .ok_or_else(|| ProtocolError::Other(
                    "Unable to access request headers".into()
                ))?;

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

        builder
            .body(())
            .map_err(|e| CoreError::Protocol(ProtocolError::Http(e)))
    }
}
