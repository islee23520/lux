use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Command, Stdio},
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

const TOKEN: &str = "sessions-smoke-token";
static SESSION_GATEWAY_LOCK: Mutex<()> = Mutex::new(());

struct GatewayProcess {
    child: std::process::Child,
    _test_lock: MutexGuard<'static, ()>,
}

impl Drop for GatewayProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn reserve_local_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("reserve local port")
        .local_addr()
        .expect("read local addr")
        .port()
}

fn start_gateway(port: u16) -> GatewayProcess {
    let test_lock = SESSION_GATEWAY_LOCK
        .lock()
        .expect("session gateway test lock poisoned");
    let child = Command::new(env!("CARGO_BIN_EXE_lux"))
        .args([
            "serve",
            "--token",
            TOKEN,
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start lux test server");

    GatewayProcess {
        child,
        _test_lock: test_lock,
    }
}

fn wait_for_health(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if http_get_status(port, "/health") == Some(200) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("lux did not become healthy on port {port}");
}

fn http_get_status(port: u16, path: &str) -> Option<u16> {
    raw_request(port, "GET", path, None, &[])
}

fn http_post_json_status(port: u16, path: &str, body: &str) -> Option<u16> {
    raw_request(
        port,
        "POST",
        path,
        Some(body),
        &[("x-lux-token", TOKEN), ("Content-Type", "application/json")],
    )
}

fn http_put_json_status(port: u16, path: &str, body: &str) -> Option<u16> {
    raw_request(
        port,
        "PUT",
        path,
        Some(body),
        &[("x-lux-token", TOKEN), ("Content-Type", "application/json")],
    )
}

fn http_delete_status(port: u16, path: &str) -> Option<u16> {
    raw_request(port, "DELETE", path, None, &[("x-lux-token", TOKEN)])
}

fn http_auth_get_status(port: u16, path: &str) -> Option<u16> {
    raw_request(port, "GET", path, None, &[("x-lux-token", TOKEN)])
}

fn raw_request(
    port: u16,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: &[(&str, &str)],
) -> Option<u16> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");

    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    for (name, value) in headers {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    if let Some(b) = body {
        request.push_str(&format!("Content-Length: {}\r\n", b.len()));
    }
    request.push_str("\r\n");
    if let Some(b) = body {
        request.push_str(b);
    }

    stream.write_all(request.as_bytes()).ok()?;

    let mut response = [0_u8; 512];
    let size = stream.read(&mut response).ok()?;
    let response = std::str::from_utf8(&response[..size]).ok()?;
    response
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn http_auth_get_json(port: u16, path: &str) -> Option<serde_json::Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nx-lux-token: {TOKEN}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).ok()?;

    // Use a single BufReader for both headers and body to avoid data loss
    let mut reader = BufReader::new(&mut stream);
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break; // End of headers
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = rest.trim().parse().ok();
        }
    }

    // Read remaining body bytes from the same BufReader
    if let Some(cl) = content_length {
        let mut body_buf = vec![0_u8; cl];
        reader.read_exact(&mut body_buf).ok()?;
        serde_json::from_slice(&body_buf).ok()
    } else {
        // No Content-Length; read all remaining bytes
        let mut body_bytes = Vec::new();
        reader.read_to_end(&mut body_bytes).ok()?;
        if body_bytes.is_empty() {
            return None;
        }
        serde_json::from_slice(&body_bytes).ok()
    }
}

#[test]
fn create_session_returns_201() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_post_json_status(port, "/api/sessions", r#"{"name":"smoke-test-session"}"#);
    assert_eq!(status, Some(201), "POST /api/sessions should return 201");
}

#[test]
fn list_sessions_returns_array() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    http_post_json_status(port, "/api/sessions", r#"{"name":"for-list"}"#);

    let json = http_auth_get_json(port, "/api/sessions").expect("list sessions JSON");
    assert!(
        json.is_array(),
        "GET /api/sessions should return an array, got: {json}"
    );
    assert_eq!(
        json.as_array().unwrap().len(),
        1,
        "expected 1 session after creation"
    );
}

#[test]
fn get_session_by_id() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    http_post_json_status(port, "/api/sessions", r#"{"name":"for-get"}"#);

    let list = http_auth_get_json(port, "/api/sessions").expect("list JSON");
    let session_id = list[0]["id"].as_str().expect("session id");

    let json =
        http_auth_get_json(port, &format!("/api/sessions/{session_id}")).expect("get session JSON");
    assert_eq!(json["id"], session_id);
    assert_eq!(json["name"], "for-get");
    assert_eq!(json["status"], "active");
}

#[test]
fn get_session_404_for_missing() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_auth_get_status(port, "/api/sessions/nonexistent-id-12345");
    assert_eq!(status, Some(404), "GET missing session should return 404");
}

#[test]
fn delete_session_returns_204() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    http_post_json_status(port, "/api/sessions", r#"{"name":"to-delete"}"#);

    let list = http_auth_get_json(port, "/api/sessions").expect("list JSON");
    let session_id = list[0]["id"].as_str().expect("session id");

    let status = http_delete_status(port, &format!("/api/sessions/{session_id}"));
    assert_eq!(status, Some(204), "DELETE should return 204");
}

#[test]
fn delete_session_404_for_missing() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_delete_status(port, "/api/sessions/nonexistent-id-99999");
    assert_eq!(
        status,
        Some(404),
        "DELETE missing session should return 404"
    );
}

#[test]
fn update_session_updates_timestamp() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    http_post_json_status(
        port,
        "/api/sessions",
        r#"{"name":"for-update","metadata":{"key":"v1"}}"#,
    );

    let list = http_auth_get_json(port, "/api/sessions").expect("list JSON");
    let session_id = list[0]["id"].as_str().expect("session id");
    let original_ts = list[0]["updatedAtUtc"]
        .as_str()
        .expect("original ts")
        .to_string();

    thread::sleep(Duration::from_secs(2));

    let put_status = http_put_json_status(
        port,
        &format!("/api/sessions/{session_id}"),
        r#"{"metadata":{"key":"v2","updated":true}}"#,
    );
    assert_eq!(put_status, Some(200), "PUT should return 200");

    let updated = http_auth_get_json(port, &format!("/api/sessions/{session_id}"))
        .expect("updated session JSON");
    let new_ts = updated["updatedAtUtc"].as_str().expect("new ts");
    assert_ne!(
        original_ts, new_ts,
        "updatedAtUtc should change after PUT update"
    );
    assert_eq!(
        updated["metadata"]["key"], "v2",
        "metadata should be updated"
    );
}

#[test]
fn list_empty_when_no_sessions() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    let json = http_auth_get_json(port, "/api/sessions").expect("list JSON");
    assert!(json.is_array(), "should return array");
    assert_eq!(
        json.as_array().unwrap().len(),
        0,
        "no sessions created yet; expected empty array"
    );
}
