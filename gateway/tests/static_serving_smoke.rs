use std::{
    fs,
    net::TcpListener,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const TOKEN: &str = "static-smoke-token";

struct GatewayProcess {
    child: std::process::Child,
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

    GatewayProcess { child }
}

fn wait_for_health(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if http_status(port, "/health") == Some(200) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("lux did not become healthy on port {port}");
}

fn http_status(port: u16, path: &str) -> Option<u16> {
    request_status(port, "GET", path, &[])
}

fn http_body(port: u16, path: &str) -> Option<String> {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");

    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;

    // Use a single BufReader for both headers and body
    let mut reader = BufReader::new(&mut stream);
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = rest.trim().parse().ok();
        }
    }

    // Read body from same BufReader
    if let Some(cl) = content_length {
        let mut body = vec![0_u8; cl];
        let n = reader.read(&mut body).ok()?;
        Some(String::from_utf8_lossy(&body[..n]).to_string())
    } else {
        let mut body_bytes = Vec::new();
        reader.read_to_end(&mut body_bytes).ok()?;
        Some(String::from_utf8_lossy(&body_bytes).to_string())
    }
}

fn request_status(port: u16, method: &str, path: &str, headers: &[(&str, &str)]) -> Option<u16> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

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
    request.push_str("\r\n");

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

#[test]
fn serves_index_html_from_ui_dir() {
    let port = reserve_local_port();

    // Ensure ui/ directory exists with an index.html for this test
    let ui_dir = std::env::current_dir().unwrap().join("ui");
    let _ = fs::create_dir_all(&ui_dir);
    fs::write(
        ui_dir.join("index.html"),
        "<!DOCTYPE html><html><body>LUX UI</body></html>",
    )
    .expect("write test index.html");

    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_status(port, "/ui/");
    assert_eq!(status, Some(200), "GET /ui/ should return 200");

    let body = http_body(port, "/ui/").expect("read /ui/ body");
    assert!(
        body.contains("<html") || body.contains("<!DOCTYPE"),
        "/ui/ should serve HTML content"
    );
}

#[test]
fn serves_nested_asset() {
    let port = reserve_local_port();

    // Ensure ui/ directory exists with a favicon.svg for this test
    let ui_dir = std::env::current_dir().unwrap().join("ui");
    let _ = fs::create_dir_all(&ui_dir);
    fs::write(
        ui_dir.join("favicon.svg"),
        "<svg xmlns='http://www.w3.org/2000/svg'></svg>",
    )
    .expect("write test favicon.svg");

    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_status(port, "/ui/favicon.svg");
    assert_eq!(status, Some(200), "GET /ui/favicon.svg should return 200");

    let body = http_body(port, "/ui/favicon.svg").expect("read favicon body");
    assert!(
        body.contains("<svg") || !body.is_empty(),
        "favicon.svg should contain SVG content or be non-empty"
    );
}

#[test]
fn returns_404_for_missing_file() {
    let port = reserve_local_port();
    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_status(port, "/ui/nonexistent-file-xyz-123.html");
    assert_eq!(
        status,
        Some(404),
        "GET missing file under /ui/ should return 404"
    );
}

#[test]
fn handles_missing_ui_dir_gracefully() {
    let port = reserve_local_port();

    // Rename ui/ if it exists so the server sees it as missing
    let ui_dir_backup = std::env::current_dir()
        .unwrap()
        .join("ui")
        .canonicalize()
        .ok();
    let renamed_path = if let Some(ref ui) = ui_dir_backup {
        let target = ui.with_file_name("ui_smoke_test_renamed");
        let _ = fs::rename(ui, &target);
        Some(target)
    } else {
        None
    };

    let _gateway = start_gateway(port);
    wait_for_health(port);

    let status = http_status(port, "/ui/");
    // ServeDir handles missing dir gracefully — may return 200 (fallback), 404, or 503
    // The important thing is the server did not crash
    assert!(
        status.is_some(),
        "GET /ui/ should return a response (not crash) when ui/ dir is absent"
    );

    // Restore original ui/ directory
    if let (Some(original), Some(renamed)) = (ui_dir_backup, renamed_path) {
        let _ = fs::rename(renamed, original);
    }
}
