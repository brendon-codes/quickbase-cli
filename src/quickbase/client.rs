use anyhow::Context;
use reqwest::{
    Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::Serialize;
use serde_json::Value;

use crate::{error::Result, quickbase::request::PreparedRequest};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickbaseResponse {
    pub status: u16,
    pub success: bool,
    pub body: Value,
}

#[derive(Clone, Debug, Default)]
pub struct QuickbaseClient {
    inner: reqwest::Client,
}

impl QuickbaseClient {
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
        }
    }

    pub async fn execute(&self, request: &PreparedRequest) -> Result<QuickbaseResponse> {
        let method = Method::from_bytes(request.method.as_bytes())
            .with_context(|| format!("invalid HTTP method {}", request.method))?;
        let mut builder = self
            .inner
            .request(method, &request.url)
            .headers(header_map(request)?);

        if let Some(body) = &request.body {
            builder = builder.json(body);
        }

        let response = builder
            .send()
            .await
            .with_context(|| format!("failed to send request to {}", request.url))?;
        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("failed to read Quickbase response body")?;
        let body = if body_text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&body_text).unwrap_or(Value::String(body_text))
        };

        Ok(QuickbaseResponse {
            status: status.as_u16(),
            success: status.is_success(),
            body,
        })
    }
}

fn header_map(request: &PreparedRequest) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    for (name, value) in &request.headers {
        headers.insert(
            HeaderName::from_bytes(name.as_bytes())
                .with_context(|| format!("invalid header name {name}"))?,
            HeaderValue::from_str(value).with_context(|| format!("invalid header value {name}"))?,
        );
    }

    Ok(headers)
}
