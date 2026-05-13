use anyhow::{Context, Result};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{AuthCmd, OutputFormat};
use crate::client::{Client, ExitCode};
use crate::output;

fn token_file(base_url: &str) -> PathBuf {
    let name = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .replace(['/', ':', '.', '-'], "_");
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ileap")
        .join(format!("token_{name}"))
}

pub fn save_token(base_url: &str, token: &str) -> Result<()> {
    let path = token_file(base_url);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create config directory at {}", dir.display()))?;
    }
    std::fs::write(&path, token)
        .with_context(|| format!("failed to save token to {}", path.display()))
}

pub fn jwt_exp(token: &str) -> Option<u64> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: Value = serde_json::from_slice(&bytes).ok()?;
    json.get("exp").and_then(|v| v.as_u64())
}

pub fn load_saved_token(base_url: &str) -> Option<String> {
    let t = std::fs::read_to_string(token_file(base_url))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    if let Some(exp) = jwt_exp(&t) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        if exp <= now + 60 {
            return None;
        }
    }

    Some(t)
}

pub fn credential_error(username: Option<&str>, password: Option<&str>) -> anyhow::Error {
    let msg = match (username, password) {
        (Some(_), None) => "--username provided but --password is missing — provide --password or set ILEAP_PASSWORD",
        (None, Some(_)) => "--password provided but --username is missing — provide --username or set ILEAP_USERNAME",
        (None, None) => "not authenticated — provide --username and --password (or set ILEAP_USERNAME + ILEAP_PASSWORD)",
        (Some(_), Some(_)) => unreachable!("credential_error called with both credentials present"),
    };
    anyhow::Error::from(ExitCode(4)).context(msg)
}

pub async fn run_auth(
    cmd: AuthCmd,
    base_url: &str,
    token: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    timeout: Option<Duration>,
    output: &OutputFormat,
) -> Result<()> {
    match cmd {
        AuthCmd::Login => {
            if let Some(t) = token {
                save_token(base_url, t)?;
                output::print_value(
                    &serde_json::json!({"authenticated": true, "token_source": "flag"}),
                    output,
                );
                return Ok(());
            }
            if load_saved_token(base_url).is_some() {
                output::print_value(
                    &serde_json::json!({"authenticated": true, "token_source": "cache"}),
                    output,
                );
                return Ok(());
            }
            match (username, password) {
                (Some(u), Some(p)) => {
                    let c = Client::authenticate(base_url, u, p, timeout).await?;
                    save_token(base_url, c.token())?;
                    output::print_value(
                        &serde_json::json!({"authenticated": true, "token_source": "credentials"}),
                        output,
                    );
                }
                (u, p) => return Err(credential_error(u, p)),
            }
        }
        AuthCmd::Status => {
            match load_saved_token(base_url) {
                Some(t) => {
                    let expires_in = jwt_exp(&t).map(|exp| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        exp.saturating_sub(now)
                    });
                    output::print_value(
                        &serde_json::json!({"authenticated": true, "expires_in": expires_in}),
                        output,
                    );
                }
                None => {
                    output::print_value(&serde_json::json!({"authenticated": false}), output);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_jwt(claims: serde_json::Value) -> String {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        format!("header.{payload}.sig")
    }

    #[test]
    fn jwt_exp_returns_exp_claim() {
        let token = make_jwt(json!({"exp": 9999999999u64, "sub": "test"}));
        assert_eq!(jwt_exp(&token), Some(9999999999u64));
    }

    #[test]
    fn jwt_exp_no_exp_claim_returns_none() {
        let token = make_jwt(json!({"sub": "test"}));
        assert_eq!(jwt_exp(&token), None);
    }

    #[test]
    fn jwt_exp_malformed_token_returns_none() {
        assert_eq!(jwt_exp("not.a.jwt"), None);
        assert_eq!(jwt_exp("onlyone"), None);
    }
}
