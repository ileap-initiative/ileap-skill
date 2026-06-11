use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::time::Duration;

use crate::cli::{AuthCmd, OutputFormat};
use crate::client::Client;
use crate::error::CliError;
use crate::output;
use crate::prompt::{prompt, prompt_password};

fn token_file(base_url: &str, username: &str) -> Result<PathBuf> {
    // Percent-encode each section (ADR-0010). NON_ALPHANUMERIC encodes every
    // byte except [A-Za-z0-9], which makes the mapping injective (no
    // collisions — unlike the earlier lossy `replace([...], "_")`),
    // filesystem-safe (`/` -> %2F keeps it a single path component, no `..`
    // traversal), and keeps the scheme so http/https never collide (ADR-0006).
    // The `__` separator is unambiguous because `_` itself encodes to %5F, so
    // an encoded section can never contain a raw underscore.
    let url = utf8_percent_encode(base_url, NON_ALPHANUMERIC);
    let user = utf8_percent_encode(username, NON_ALPHANUMERIC);
    let config_dir = dirs::config_dir()
        .context("cannot determine config directory; set HOME or XDG_CONFIG_HOME")?;
    Ok(config_dir
        .join("ileap")
        .join(format!("token_{url}__{user}")))
}

pub fn save_token(base_url: &str, username: &str, token: &str) -> Result<()> {
    let path = token_file(base_url, username)?;
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

pub fn jwt_sub(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: Value = serde_json::from_slice(&bytes).ok()?;
    json.get("sub").and_then(|v| v.as_str()).map(str::to_string)
}

pub fn load_saved_token(base_url: &str, username: &str) -> Result<Option<String>> {
    let path = token_file(base_url, username)?;
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
                // The cache file is keyed by username (ADR-0010); for a token
                // passed via flag, fall back to the token's own `sub` claim
                // when no --username/ILEAP_USERNAME is supplied.
                let cache_user = match username {
                    Some(u) => u.to_string(),
                    None => jwt_sub(t).ok_or_else(|| {
                        CliError::Auth(
                            "provide --username (or set ILEAP_USERNAME) to associate the cached token".to_string(),
                        )
                    })?,
                };
                save_token(base_url, &cache_user, t)?;
                output::print_value(
                    &serde_json::json!({"authenticated": true, "token_source": "flag"}),
                    output,
                );
                return Ok(());
            }
            if let Some(u) = username
                && load_saved_token(base_url, u)?.is_some()
            {
                output::print_value(
                    &serde_json::json!({"authenticated": true, "token_source": "cache"}),
                    output,
                );
                return Ok(());
            }
            match (username, password) {
                (Some(u), Some(p)) => {
                    let c = Client::authenticate(base_url, u, p, timeout).await?;
                    save_token(base_url, u, c.token())?;
                    output::print_value(
                        &serde_json::json!({"authenticated": true, "token_source": "credentials"}),
                        output,
                    );
                }
                (u, p) => {
                    if std::io::stdin().is_terminal() {
                        let u = prompt("Username: ")?;
                        // The username is only known now; probe the cache for it
                        // before asking for a password (ADR-0010).
                        if load_saved_token(base_url, &u)?.is_some() {
                            output::print_value(
                                &serde_json::json!({"authenticated": true, "token_source": "cache"}),
                                output,
                            );
                            return Ok(());
                        }
                        let p = prompt_password("Password: ")?;
                        let c = Client::authenticate(base_url, &u, &p, timeout).await?;
                        save_token(base_url, &u, c.token())?;
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
        AuthCmd::Status => match username.and_then(|u| load_saved_token(base_url, u).transpose()) {
            Some(Ok(t)) => {
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
            Some(Err(e)) => return Err(e),
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
        let http = token_file("http://api.example.com", "alice").unwrap();
        let https = token_file("https://api.example.com", "alice").unwrap();
        assert_ne!(http, https);
    }

    #[test]
    fn token_file_distinguishes_usernames() {
        let alice = token_file("https://api.example.com", "alice").unwrap();
        let bob = token_file("https://api.example.com", "bob").unwrap();
        assert_ne!(alice, bob);
    }

    #[test]
    fn token_file_no_collision_on_separator_chars() {
        // Under the old lossy `replace([...], "_")` scheme, both "a/b" and
        // "a_b" mapped to "a_b" and collided. Percent-encoding keeps them
        // distinct ("a%2Fb" vs "a%5Fb"), proving the key is injective.
        let url = "https://api.example.com";
        assert_ne!(
            token_file(url, "a/b").unwrap(),
            token_file(url, "a_b").unwrap()
        );
    }

    #[cfg(unix)]
    #[test]
    fn save_token_sets_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let base_url = "http://test-token-perms.invalid";
        // Remove any leftover file so we exercise the creation path
        let _ = std::fs::remove_file(token_file(base_url, "tester").unwrap());
        save_token(base_url, "tester", "tok").unwrap();
        let mode = std::fs::metadata(token_file(base_url, "tester").unwrap())
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

    // --- jwt_sub ---

    #[test]
    fn jwt_sub_returns_sub_claim() {
        let token = make_jwt(json!({"sub": "carol", "exp": 9999999999u64}));
        assert_eq!(jwt_sub(&token), Some("carol".to_string()));
    }

    #[test]
    fn jwt_sub_no_sub_or_malformed_returns_none() {
        let no_sub = make_jwt(json!({"exp": 9999999999u64}));
        assert_eq!(jwt_sub(&no_sub), None);
        assert_eq!(jwt_sub("not.a.jwt"), None);
        assert_eq!(jwt_sub("onlyone"), None);
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
        save_token(base_url, "tester", &token).unwrap();
        assert!(load_saved_token(base_url, "tester").unwrap().is_none());
    }

    #[test]
    fn load_saved_token_within_60s_buffer_returns_none() {
        // exp = now + 30 is within the 60-second pre-expiry window
        let base_url = "http://test-expiring-soon.invalid";
        let token = make_jwt(json!({"exp": now_secs() + 30u64}));
        save_token(base_url, "tester", &token).unwrap();
        assert!(load_saved_token(base_url, "tester").unwrap().is_none());
    }

    #[test]
    fn load_saved_token_valid_returns_token() {
        let base_url = "http://test-valid-token.invalid";
        let token = make_jwt(json!({"exp": now_secs() + 3600u64}));
        save_token(base_url, "tester", &token).unwrap();
        assert_eq!(load_saved_token(base_url, "tester").unwrap(), Some(token));
    }

    #[test]
    fn load_saved_token_is_scoped_by_username() {
        // A token saved for one user must not be visible to another (ADR-0010).
        let base_url = "http://test-user-scoping.invalid";
        let token = make_jwt(json!({"exp": now_secs() + 3600u64}));
        let _ = std::fs::remove_file(token_file(base_url, "bob").unwrap());
        save_token(base_url, "alice", &token).unwrap();
        assert!(load_saved_token(base_url, "bob").unwrap().is_none());
        assert_eq!(
            load_saved_token(base_url, "alice").unwrap(),
            Some(token)
        );
    }

    #[test]
    fn load_saved_token_no_exp_is_trusted() {
        // Tokens without an exp claim should be returned as-is
        let base_url = "http://test-no-exp.invalid";
        let token = make_jwt(json!({"sub": "test"}));
        save_token(base_url, "tester", &token).unwrap();
        assert!(load_saved_token(base_url, "tester").unwrap().is_some());
    }

    #[test]
    fn load_saved_token_missing_file_returns_none() {
        let base_url = "http://test-no-file-present.invalid";
        let _ = std::fs::remove_file(token_file(base_url, "tester").unwrap());
        assert!(load_saved_token(base_url, "tester").unwrap().is_none());
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
            Some("flaguser"),
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
        assert_eq!(load_saved_token(base_url, "flaguser").unwrap(), Some(token));
    }

    #[tokio::test]
    async fn run_auth_login_token_flag_keys_by_jwt_sub_without_username() {
        // With no --username, the cache key falls back to the JWT `sub` claim.
        let base_url = "http://test-run-auth-flag-sub.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64, "sub": "subuser"}));
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
        assert_eq!(load_saved_token(base_url, "subuser").unwrap(), Some(token));
    }

    #[tokio::test]
    async fn run_auth_login_token_flag_no_username_no_sub_errors() {
        // No --username and no `sub` claim: cannot key the cache → error.
        let base_url = "http://test-run-auth-flag-nosub.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64}));
        let err = run_auth(
            AuthCmd::Login,
            base_url,
            Some(&token),
            None,
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--username"));
    }

    #[tokio::test]
    async fn run_auth_login_cached_token_succeeds_without_credentials() {
        let base_url = "http://test-run-auth-cached.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64}));
        save_token(base_url, "cacheuser", &token).unwrap();
        // No password — must succeed via cache (keyed by username) without
        // hitting any server.
        run_auth(
            AuthCmd::Login,
            base_url,
            None,
            Some("cacheuser"),
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_auth_login_no_credentials_returns_exit_code_4() {
        let base_url = "http://test-run-auth-no-creds.invalid";
        let _ = std::fs::remove_file(token_file(base_url, "tester").unwrap());
        let err = run_auth(
            AuthCmd::Login,
            base_url,
            None,
            None,
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap_err();
        // err is anyhow::Error wrapping a CliError::Auth
        let ce = err
            .downcast_ref::<CliError>()
            .expect("expected CliError in chain");
        assert!(
            matches!(ce, CliError::Auth(_)),
            "expected Auth variant, got: {ce:?}"
        );
        assert_eq!(ce.exit_code(), 4);
    }

    #[tokio::test]
    async fn run_auth_status_with_valid_token_is_ok() {
        let base_url = "http://test-run-auth-status-ok.invalid";
        let token = make_jwt(json!({"exp": 9999999999u64}));
        save_token(base_url, "statususer", &token).unwrap();
        run_auth(
            AuthCmd::Status,
            base_url,
            None,
            Some("statususer"),
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
        let _ = std::fs::remove_file(token_file(base_url, "statususer").unwrap());
        run_auth(
            AuthCmd::Status,
            base_url,
            None,
            Some("statususer"),
            None,
            None,
            &OutputFormat::Compact,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_auth_status_without_username_reports_unauthenticated() {
        // No username → cannot identify a cache entry → authenticated: false, Ok.
        let base_url = "http://test-run-auth-status-no-user.invalid";
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
