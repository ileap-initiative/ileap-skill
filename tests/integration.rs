use assert_cmd::Command;
use serde_json::Value;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ileap() -> Command {
    Command::cargo_bin("ileap").unwrap()
}

// ---------------------------------------------------------------------------
// version
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// auth errors
// ---------------------------------------------------------------------------

#[test]
fn no_auth_exits_4() {
    ileap()
        .args(["--base-url", "http://no-auth-test.invalid", "shipments", "list"])
        .env_remove("ILEAP_TOKEN")
        .env_remove("ILEAP_USERNAME")
        .env_remove("ILEAP_PASSWORD")
        .assert()
        .failure()
        .code(4);
}

fn assert_auth_error_json(stderr: &str) {
    let v: Value = serde_json::from_str(stderr.trim()).expect("stderr must be valid JSON");
    assert_eq!(v["cli_error"]["type"], "auth_error");
    assert!(v["cli_error"]["message"].is_string());
}

#[test]
fn no_auth_compact_error_is_structured_json() {
    let output = ileap()
        .args([
            "-o", "compact",
            "--base-url", "http://no-auth-compact-test.invalid",
            "shipments", "list",
        ])
        .env_remove("ILEAP_TOKEN")
        .env_remove("ILEAP_USERNAME")
        .env_remove("ILEAP_PASSWORD")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_auth_error_json(&String::from_utf8_lossy(&output.stderr));
}

#[test]
fn no_auth_pretty_error_is_also_structured_json() {
    let output = ileap()
        .args([
            "--base-url", "http://no-auth-pretty-test.invalid",
            "shipments", "list",
        ])
        .env_remove("ILEAP_TOKEN")
        .env_remove("ILEAP_USERNAME")
        .env_remove("ILEAP_PASSWORD")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_auth_error_json(&String::from_utf8_lossy(&output.stderr));
}

#[test]
fn username_without_password_exits_4() {
    ileap()
        .args([
            "--base-url", "http://no-pw-test.invalid",
            "--username", "user",
            "shipments", "list",
        ])
        .env_remove("ILEAP_TOKEN")
        .env_remove("ILEAP_PASSWORD")
        .assert()
        .failure()
        .code(4);
}

// ---------------------------------------------------------------------------
// dry run
// ---------------------------------------------------------------------------

#[test]
fn dry_run_returns_request_info_without_hitting_server() {
    let output = ileap()
        .args([
            "--token", "tok",
            "--base-url", "http://dry-run-test.invalid",
            "shipments", "list",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["method"], "GET");
    assert!(v["url"].as_str().unwrap().contains("/v1/ileap/shipments"));
}

#[test]
fn dry_run_footprint_get_returns_request_info() {
    let output = ileap()
        .args([
            "--token", "tok",
            "--base-url", "http://dry-run-test.invalid",
            "footprints", "get", "abc-123",
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    assert_eq!(v["dry_run"], true);
    assert!(v["url"].as_str().unwrap().contains("abc-123"));
}

// ---------------------------------------------------------------------------
// pagination
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auto_mode_merges_pages() {
    let server = MockServer::start().await;

    // Second page (offset=2): mounted first, checked last in LIFO
    Mock::given(method("GET"))
        .and(path("/v1/ileap/shipments"))
        .and(query_param("offset", "2"))
        .respond_with(
            ResponseTemplate::new(200u16)
                .set_body_json(serde_json::json!({"data": [{"id": "c"}]})),
        )
        .mount(&server)
        .await;

    // First page (no offset): mounted second, checked first in LIFO, limited to 1 match
    Mock::given(method("GET"))
        .and(path("/v1/ileap/shipments"))
        .respond_with(
            ResponseTemplate::new(200u16)
                .set_body_json(serde_json::json!({"data": [{"id": "a"}, {"id": "b"}]})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let output = ileap()
        .args([
            "--token", "tok",
            "--base-url", &server.uri(),
            "-o", "compact",
            "shipments", "list",
            "--limit", "2",
            "--yes",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    let items = v["data"].as_array().expect("expected data array");
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["id"], "a");
    assert_eq!(items[1]["id"], "b");
    assert_eq!(items[2]["id"], "c");
}

#[tokio::test]
async fn max_pages_caps_pagination() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/ileap/shipments"))
        .respond_with(
            ResponseTemplate::new(200u16)
                .set_body_json(serde_json::json!({"data": [{"id": "a"}, {"id": "b"}]})),
        )
        .mount(&server)
        .await;

    let output = ileap()
        .args([
            "--token", "tok",
            "--base-url", &server.uri(),
            "-o", "compact",
            "shipments", "list",
            "--limit", "2",
            "--yes",
            "--max-pages", "1",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    let items = v["data"].as_array().expect("expected data array");
    assert_eq!(items.len(), 2, "max-pages=1 should stop after the first page");

    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1, "only one request should be sent");
}
