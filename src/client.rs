use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[cfg(not(test))]
const BACKOFF_BASE_MS: u64 = 1000;
#[cfg(test)]
const BACKOFF_BASE_MS: u64 = 0;

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug)]
pub struct ExitCode(pub i32);

impl std::fmt::Display for ExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit {}", self.0)
    }
}

impl std::error::Error for ExitCode {}

impl Client {
    fn build_http(timeout: Option<Duration>) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        if let Some(t) = timeout {
            builder = builder.timeout(t);
        }
        builder.build().unwrap_or_default()
    }

    async fn discover_token_endpoint(http: &reqwest::Client, base_url: &str) -> String {
        let discovery_url = format!("{base_url}/.well-known/openid-configuration");
        if let Ok(resp) = http.get(&discovery_url).send().await
            && resp.status().is_success()
                && let Ok(doc) = resp.json::<serde_json::Value>().await
                    && let Some(endpoint) = doc.get("token_endpoint").and_then(|v| v.as_str()) {
                        return endpoint.to_string();
                    }
        format!("{base_url}/auth/token")
    }

    pub async fn authenticate(
        base_url: &str,
        username: &str,
        password: &str,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        let http = Self::build_http(timeout);
        let credentials = STANDARD.encode(format!("{username}:{password}"));
        let token_endpoint = Self::discover_token_endpoint(&http, base_url).await;

        let resp = http
            .post(&token_endpoint)
            .header("Authorization", format!("Basic {credentials}"))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow::anyhow!("request to {token_endpoint} timed out — increase --timeout if needed")
                } else {
                    anyhow::anyhow!("failed to reach token endpoint at {token_endpoint} — verify --base-url is correct and the server is reachable")
                }
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let hint = match status.as_u16() {
                401 => " — credentials rejected, check --username and --password",
                403 => " — credentials valid but access denied, check account permissions",
                _ => "",
            };
            let exit_code = match status.as_u16() { 401 | 403 => 4, _ => 1 };
            return Err(anyhow::Error::from(ExitCode(exit_code))
                .context(format!("authentication failed ({status}){hint}: {body}")));
        }

        let body = resp.text().await.context("failed to read auth response body")?;
        let token_resp: TokenResponse = serde_json::from_str(&body)
            .with_context(|| format!("failed to parse auth response — unexpected format: {body}"))?;

        Ok(Self {
            http,
            base_url: base_url.to_string(),
            token: token_resp.access_token,
        })
    }

    pub fn from_token(base_url: &str, token: String, timeout: Option<Duration>) -> Self {
        Self {
            http: Self::build_http(timeout),
            base_url: base_url.to_string(),
            token,
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    async fn get(&self, path: &str, params: Vec<(String, String)>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        const MAX_RETRIES: u32 = 2;
        let mut attempt = 0u32;

        loop {
            let resp = self
                .http
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .query(&params)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        anyhow::anyhow!("request to {url} timed out — increase --timeout if needed")
                    } else {
                        anyhow::anyhow!("failed to connect to {url} — verify --base-url is correct and the server is reachable")
                    }
                })?;

            let status = resp.status();

            if (status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                && attempt < MAX_RETRIES
            {
                let body = resp.text().await.unwrap_or_default();
                let backoff_ms = BACKOFF_BASE_MS * (1 << attempt);
                let jitter_ms = if BACKOFF_BASE_MS > 0 {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.subsec_millis() as u64 % 500)
                        .unwrap_or(0)
                } else {
                    0
                };
                let delay = Duration::from_millis(backoff_ms + jitter_ms);
                eprintln!(
                    "request failed ({status}), retrying in {}ms (attempt {}/{MAX_RETRIES}): {body}",
                    delay.as_millis(),
                    attempt + 1,
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
                continue;
            }

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                let hint = match status.as_u16() {
                    401 => " — token may be expired, re-authenticate or provide a fresh --token",
                    403 => " — access denied to this resource",
                    404 => " — resource not found, check the ID or filter values",
                    429 => " — rate limited, retry limit reached",
                    500..=599 => " — server error, the API may be temporarily unavailable",
                    _ => "",
                };
                let exit_code = match status.as_u16() { 404 => 3, 401 | 403 => 4, _ => 1 };
                return Err(anyhow::Error::from(ExitCode(exit_code))
                    .context(format!("GET {url} failed ({status}){hint}: {body}")));
            }

            let body = resp.text().await.context("failed to read response body")?;
            return serde_json::from_str(&body)
                .with_context(|| format!("failed to parse response — unexpected format: {body}"));
        }
    }

    fn base_params(limit: Option<u32>, offset: u32) -> Vec<(String, String)> {
        let mut p = Vec::new();
        if let Some(l) = limit {
            p.push(("limit".into(), l.to_string()));
        }
        if offset > 0 {
            p.push(("offset".into(), offset.to_string()));
        }
        p
    }

    async fn get_with_params(
        &self,
        path: &str,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Result<Value> {
        let mut params = Self::base_params(limit, offset);
        for f in filters {
            if let Some((k, v)) = f.split_once('=') {
                params.push((k.into(), v.into()));
            }
        }
        self.get(path, params).await
    }

    fn dry_run_value(&self, path: &str, params: Vec<(String, String)>) -> Value {
        let url = format!("{}{}", self.base_url, path);
        let params_obj: serde_json::Map<String, Value> = params
            .into_iter()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        serde_json::json!({
            "dry_run": true,
            "method": "GET",
            "url": url,
            "params": params_obj,
        })
    }

    pub fn footprints_dry_run(&self, limit: Option<u32>, offset: u32, filter: &[String]) -> Value {
        let mut params = Self::base_params(limit, offset);
        if let Some(f) = filter.first() {
            params.push(("$filter".into(), f.clone()));
        }
        self.dry_run_value("/2/footprints", params)
    }

    pub fn footprint_dry_run(&self, id: &str) -> Value {
        self.dry_run_value(&format!("/2/footprints/{id}"), vec![])
    }

    pub fn list_dry_run(
        &self,
        path: &str,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Value {
        let mut params = Self::base_params(limit, offset);
        for f in filters {
            if let Some((k, v)) = f.split_once('=') {
                params.push((k.into(), v.into()));
            }
        }
        self.dry_run_value(path, params)
    }

    pub async fn footprints(
        &self,
        limit: Option<u32>,
        offset: u32,
        filter: &[String],
    ) -> Result<Value> {
        let mut params = Self::base_params(limit, offset);
        // PACT uses OData $filter; only a single expression is supported
        if let Some(f) = filter.first() {
            params.push(("$filter".into(), f.clone()));
        }
        self.get("/2/footprints", params).await
    }

    pub async fn footprint(&self, id: &str) -> Result<Value> {
        self.get(&format!("/2/footprints/{id}"), vec![]).await
    }

    pub async fn shipments(
        &self,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Result<Value> {
        self.get_with_params("/v1/ileap/shipments", limit, offset, filters).await
    }

    pub async fn tocs(
        &self,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Result<Value> {
        self.get_with_params("/v1/ileap/tocs", limit, offset, filters).await
    }

    pub async fn hocs(
        &self,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Result<Value> {
        self.get_with_params("/v1/ileap/hocs", limit, offset, filters).await
    }

    pub async fn tad(
        &self,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Result<Value> {
        self.get_with_params("/v1/ileap/tad", limit, offset, filters).await
    }

    pub async fn aed(
        &self,
        limit: Option<u32>,
        offset: u32,
        filters: &[String],
    ) -> Result<Value> {
        self.get_with_params("/v1/ileap/aed", limit, offset, filters).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn client(server: &MockServer) -> Client {
        Client::from_token(&server.uri(), "test-token".to_string(), None)
    }

    fn exit_code(err: &anyhow::Error) -> Option<i32> {
        err.chain()
            .find_map(|c| c.downcast_ref::<ExitCode>())
            .map(|ec| ec.0)
    }

    #[tokio::test]
    async fn retry_on_429_then_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/footprints"))
            .respond_with(ResponseTemplate::new(200u16).set_body_json(serde_json::json!({"data": []})))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/2/footprints"))
            .respond_with(ResponseTemplate::new(429u16))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let result = client(&server).await.footprints(None, 0, &[]).await;
        assert!(result.is_ok(), "expected success after 429 retry, got: {result:?}");
    }

    #[tokio::test]
    async fn retry_on_500_then_succeeds() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/footprints"))
            .respond_with(ResponseTemplate::new(200u16).set_body_json(serde_json::json!({"data": []})))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/2/footprints"))
            .respond_with(ResponseTemplate::new(500u16))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let result = client(&server).await.footprints(None, 0, &[]).await;
        assert!(result.is_ok(), "expected success after 500 retry, got: {result:?}");
    }

    #[tokio::test]
    async fn no_retry_on_404_returns_exit_code_3() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/footprints/nope"))
            .respond_with(ResponseTemplate::new(404u16).set_body_json(serde_json::json!("not found")))
            .mount(&server)
            .await;

        let err = client(&server).await.footprint("nope").await.unwrap_err();
        assert_eq!(exit_code(&err), Some(3));

        let received = server.received_requests().await.unwrap();
        assert_eq!(received.len(), 1, "404 must not be retried");
    }

    #[tokio::test]
    async fn auth_401_returns_exit_code_4() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/footprints"))
            .respond_with(ResponseTemplate::new(401u16))
            .mount(&server)
            .await;

        let err = client(&server).await.footprints(None, 0, &[]).await.unwrap_err();
        assert_eq!(exit_code(&err), Some(4));
    }

    #[tokio::test]
    async fn retries_exhausted_returns_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/footprints"))
            .respond_with(ResponseTemplate::new(503u16))
            .mount(&server)
            .await;

        let result = client(&server).await.footprints(None, 0, &[]).await;
        assert!(result.is_err(), "expected error after retries exhausted");

        let received = server.received_requests().await.unwrap();
        assert_eq!(received.len(), 3, "should attempt 1 + 2 retries = 3 total");
    }
}
