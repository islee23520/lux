use std::{
    net::TcpListener,
    process::{Command, Stdio},
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

const TOKEN: &str = "static-smoke-token";
static STATIC_SERVING_LOCK: Mutex<()> = Mutex::new(());

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

fn start_static_test() -> (MutexGuard<'static, ()>, u16) {
    let test_lock = STATIC_SERVING_LOCK
        .lock()
        .expect("static serving test lock poisoned");
    let port = reserve_local_port();
    (test_lock, port)
}

fn start_gateway(port: u16, test_lock: MutexGuard<'static, ()>) -> GatewayProcess {
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
    let deadline = Instant::now() + Duration::from_secs(15);
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
fn ui_route_is_not_served_when_ui_is_removed() {
    let (test_lock, port) = start_static_test();

    let _gateway = start_gateway(port, test_lock);
    wait_for_health(port);

    let status = http_status(port, "/ui/");
    assert_eq!(status, Some(404), "GET /ui/ should not serve UI assets");
}
