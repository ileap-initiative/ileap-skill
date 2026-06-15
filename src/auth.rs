use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{AuthCmd, OutputFormat};
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::prompt::{prompt, prompt_password};

fn token_file(base_url: &str) -> Result<PathBuf> {
    // The scheme stays in the name so http:// and https:// tokens for the
    // same host never share a cache file (ADR-0006).
    let name = base_url.replace(['/', ':', '.', '-'], "_");
    let config_dir = dirs::config_dir()
        .context("cannot determine config directory; set HOME or XDG_CONFIG_HOME")?;
    Ok(config_dir.join("ileap").join(format!("token_{name}")))
}

pub fn save_token(base_url: &str, token: &str) -> Result<()> {
    let path = token_file(base_url)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create config directory at {}", dir.display()))?;
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, token.as_bytes()))
        .with_context(|| format!("failed to save token to {}", path.display()))
}

pub fn jwt_exp(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: Value = serde_json::from_slice(&bytes).ok()?;
    json.get("exp").and_then(|v| v.as_u64())
}

pub fn load_saved_token(base_url: &str) -> Result<Option<String>> {
    let path = token_file(base_url)?;
    let t = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(anyhow::Error::from(e).context(format!(
                "failed to read cached token from {}",
                path.display()
            )));
        }
    };
    let t = t.trim().to_string();
    if t.is_empty() {
        return Ok(None);
    }

    if let Some(exp) = jwt_exp(&t) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if exp <= now + 60 {
            return Ok(None);
        }
    }

    Ok(Some(t))
}

pub fn credential_error(username: Option<&str>, password: Option<&str>) -> CliError {
    let msg = match (username, password) {
        (Some(_), None) => {
            "--username provided but --password is missing — provide --password or set ILEAP_PASSWORD"
        }
        (None, Some(_)) => {
            "--password provided but --username is missing — provide --username or set ILEAP_USERNAME"
        }
        (None, None) => {
            "not authenticated — provide --username and --password (or set ILEAP_USERNAME + ILEAP_PASSWORD)"
        }
        (Some(_), Some(_)) => unreachable!("credential_error called with both credentials present"),
    };
    CliError::Auth(msg.to_string())
}

/// Resolve a `Client` from the credential chain: explicit `--token` wins, then a
/// valid cached token, then username/password authentication. This is the most
/// security-relevant decision in the program, so it lives in `auth.rs` (ADR-0009).
pub async fn resolve_client(
    base_url: &str,
    token: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    timeout: Option<Duration>,
) -> Result<Client> {
    if let Some(t) = token {
        Ok(Client::from_token(base_url, t.to_string(), timeout))
    } else if let Some(t) = load_saved_token(base_url)? {
        Ok(Client::from_token(base_url, t, timeout))
    } else {
        match (username, password) {
            (Some(u), Some(p)) => Ok(Client::authenticate(base_url, u, p, timeout).await?),
            (u, p) => Err(credential_error(u, p).into()),
        }
    }
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
            if load_saved_token(base_url)?.is_some() {
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
                (u, p) => {
                    if std::io::stdin().is_terminal() {
                        // Honor any credential already provided; prompt only for
                        // what is missing (ADR-0009 §3).
                        let u = match u {
                            Some(u) => u.to_string(),
                            None => prompt("Username: ")?,
                        };
                        let p = match p {
                            Some(p) => p.to_string(),
                            None => prompt_password("Password: ")?,
                        };
                        let c = Client::authenticate(base_url, &u, &p, timeout).await?;
                        save_token(base_url, c.token())?;
                        output::print_value(
                            &serde_json::json!({"authenticated": true, "token_source": "credentials"}),
                            output,
                        );
                    } else {
                        return Err(credential_error(u, p).into());
                    }
                }
            }
        }
        AuthCmd::Status => match load_saved_token(base_url)? {
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
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{AuthCmd, OutputFormat};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_jwt(claims: serde_json::Value) -> String {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());
        format!("header.{payload}.sig")
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    // --- token_file ---

    #[test]
    fn token_file_distinguishes_schemes() {
        let http = token_file("http://api.example.com").unwrap();
        let https = token_file("https://api.example.com").unwrap();
        assert_ne!(http, https);
    }

    #[cfg(unix)]
    #[test]
    fn save_token_sets_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let base_url = "http://test-token-perms.invalid";
        // Remove any leftover file so we exercise the creation path
        let _ = std::fs::remove_file(token_file(base_url).unwrap());
        save_token(base_url, "tok").unwrap();
        let mode = std::fs::metadata(token_file(base_url).unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // --- jwt_exp ---

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

    // --- credential_error ---

    #[test]
    fn credential_error_username_only_exit_code_4() {
        let err = credential_error(Some("user"), None);
        assert!(
            matches!(err, CliError::Auth(_)),
            "expected Auth variant, got: {err:?}"
        );
        assert_eq!(err.exit_code(), 4);
        let msg = err.to_string();
        assert!(
            msg.contains("--password"),
            "expected --password hint, got: {msg}"
        );
    }

    #[test]
    fn credential_error_password_only_exit_code_4() {
        let err = credential_error(None, Some("pass"));
        assert!(
            matches!(err, CliError::Auth(_)),
            "expected Auth variant, got: {err:?}"
        );
        assert_eq!(err.exit_code(), 4);
        let msg = err.to_string();
        assert!(
            msg.contains("--username"),
            "expected --username hint, got: {msg}"
        );
    }

    #[test]
    fn credential_error_neither_exit_code_4() {
        let err = credential_error(None, None);
        assert!(
            matches!(err, CliError::Auth(_)),
            "expected Auth variant, got: {err:?}"
        );
        assert_eq!(err.exit_code(), 4);
        let msg = err.to_string();
        assert!(
            msg.contains("not authenticated"),
            "expected 'not authenticated', got: {msg}"
        );
    }

    // --- load_saved_token expiry boundary ---

    #[test]
    fn load_saved_token_expired_returns_none() {
        let base_url = "http://test-expired.invalid";
        let token = make_jwt(json!({"exp": 1u64, "sub": "test"}));
        save_token(base_url, &token).unwrap();
        assert!(load_saved_token(base_url).unwrap().is_none());
    }

    #[test]
    fn load_saved_token_within_60s_buffer_returns_none() {
        // exp = now + 30 is within the 60-second pre-expiry window
        let base_url = "http://test-expiring-soon.invalid";
        let token = make_jwt(json!({"exp": now_secs() + 30u64}));
        save_token(base_url, &token).unwrap();
        assert!(load_saved_token(base_url).unwrap().is_none());
    }

    #[test]
    fn load_saved_token_valid_returns_token() {
        let base_url = "http://test-valid-token.invalid";
        let token = make_jwt(json!({"exp": now_secs() + 3600u64}));
        save_token(base_url, &token).unwrap();
        assert_eq!(load_saved_token(base_url).unwrap(), Some(token));
    }

    #[test]
    fn load_saved_token_no_exp_is_trusted() {
        // Tokens without an exp claim should be returned as-is
        let base_url = "http://test-no-exp.invalid";
        let token = make_jwt(json!({"sub": "test"}));
        save_token(base_url, &token).unwrap();
        assert!(load_saved_token(base_url).unwrap().is_some());
    }

    #[test]
    fn load_saved_token_missing_file_returns_none() {
        let base_url = "http://test-no-file-present.invalid";
        let _ = std::fs::remove_file(token_file(base_url).unwrap());
        assert!(load_saved_token(base_url).unwrap().is_none());
    }

    // --- run_auth ---

    #[tokio::test]
    async fn run_auth_login_token_flag_saves_token() {
        let base_url = "http://test-run-auth-flag.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64}));
        run_auth(
            AuthCmd::Login,
            base_url,
            Some(&token),
            None,
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
        assert_eq!(load_saved_token(base_url).unwrap(), Some(token));
    }

    #[tokio::test]
    async fn run_auth_login_cached_token_succeeds_without_credentials() {
        let base_url = "http://test-run-auth-cached.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64}));
        save_token(base_url, &token).unwrap();
        // No credentials — must succeed via cache without hitting any server
        run_auth(
            AuthCmd::Login,
            base_url,
            None,
            None,
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_auth_status_with_valid_token_is_ok() {
        let base_url = "http://test-run-auth-status-ok.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64}));
        save_token(base_url, &token).unwrap();
        run_auth(
            AuthCmd::Status,
            base_url,
            None,
            None,
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_auth_status_without_token_is_ok() {
        // Status with no cached token should still return Ok (prints authenticated: false)
        let base_url = "http://test-run-auth-status-none.invalid";
        let _ = std::fs::remove_file(token_file(base_url).unwrap());
        run_auth(
            AuthCmd::Status,
            base_url,
            None,
            None,
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
    }
}
