use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::Value;

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

impl Client {
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

    pub async fn authenticate(base_url: &str, username: &str, password: &str) -> Result<Self> {
        let http = reqwest::Client::new();
        let credentials = STANDARD.encode(format!("{username}:{password}"));
        let token_endpoint = Self::discover_token_endpoint(&http, base_url).await;

        let resp = http
            .post(&token_endpoint)
            .header("Authorization", format!("Basic {credentials}"))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .context("failed to reach auth endpoint")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("authentication failed ({status}): {body}");
        }

        let token_resp: TokenResponse = resp
            .json()
            .await
            .context("failed to parse auth response")?;

        Ok(Self {
            http,
            base_url: base_url.to_string(),
            token: token_resp.access_token,
        })
    }

    pub fn from_token(base_url: &str, token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.to_string(),
            token,
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    async fn get(&self, path: &str, params: Vec<(String, String)>) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);

        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .query(&params)
            .send()
            .await
            .with_context(|| format!("GET {url}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GET {url} failed ({status}): {body}");
        }

        resp.json().await.context("failed to parse response")
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
