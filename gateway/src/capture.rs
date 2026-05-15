use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::{mpsc, watch, Mutex, RwLock},
};
use uuid::Uuid;

use crate::protocol::{
    LuxInputEvent, LuxStreamFrame, StartLuxStreamRequest, CMD_LUX_INPUT_EVENT,
    CMD_LUX_STREAM_FRAME, CMD_START_LUX_STREAM, CMD_STOP_LUX_STREAM,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSession {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(skip_serializing)]
    pub frame_rx: watch::Receiver<Option<Vec<u8>>>,
    #[serde(skip_serializing)]
    pub input_tx: mpsc::Sender<InputEvent>,
    pub created_at: DateTime<Utc>,
    pub project_path: PathBuf,
    pub status: CaptureStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CaptureStatus {
    Starting,
    Streaming,
    Stopping,
    Stopped,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub button: Option<u32>,
    pub key: Option<String>,
    pub delta: Option<f64>,
}

#[derive(Clone)]
pub struct CaptureManager {
    sessions: Arc<RwLock<HashMap<String, CaptureSession>>>,
    frame_txs: Arc<RwLock<HashMap<String, watch::Sender<Option<Vec<u8>>>>>>,
    bridge_connection: Arc<Mutex<Option<TcpStream>>>,
    default_project_path: Option<PathBuf>,
}

pub type CaptureSessionManager = CaptureManager;

#[derive(Debug, Deserialize)]
struct UnityBridgeDiscovery {
    host: String,
    port: u16,
    token: String,
}

impl CaptureManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            frame_txs: Arc::new(RwLock::new(HashMap::new())),
            bridge_connection: Arc::new(Mutex::new(None)),
            default_project_path: None,
        }
    }

    pub fn with_project_root(project_path: Option<PathBuf>) -> Self {
        Self {
            default_project_path: project_path,
            ..Self::new()
        }
    }

    pub async fn start_session(
        &self,
        width: u32,
        height: u32,
        fps: u32,
    ) -> anyhow::Result<CaptureSession> {
        let project_path = self.default_project_path.clone().ok_or_else(|| {
            anyhow::anyhow!("project path is required to start a capture session")
        })?;
        self.create_session(project_path, width, height, fps).await
    }

    pub async fn create_session(
        &self,
        project_path: PathBuf,
        width: u32,
        height: u32,
        fps: u32,
    ) -> anyhow::Result<CaptureSession> {
        let session_id = Uuid::new_v4().to_string();
        let (frame_tx, frame_rx) = watch::channel(None);
        let (input_tx, input_rx) = mpsc::channel(256);
        let session = CaptureSession {
            id: session_id.clone(),
            width,
            height,
            fps,
            frame_rx,
            input_tx,
            created_at: Utc::now(),
            project_path: project_path.clone(),
            status: CaptureStatus::Starting,
        };

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session.clone());
        self.frame_txs
            .write()
            .await
            .insert(session_id.clone(), frame_tx);

        let request = StartLuxStreamRequest {
            width,
            height,
            fps,
            session_id: session_id.clone(),
        };

        if let Err(error) = self.open_stream_reader(&project_path, request).await {
            self.sessions.write().await.remove(&session_id);
            self.frame_txs.write().await.remove(&session_id);
            return Err(error);
        }

        self.spawn_input_forwarder(session_id.clone(), project_path, input_rx);

        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .context("capture session disappeared while starting")?;
        session.status = CaptureStatus::Streaming;
        Ok(session.clone())
    }

    pub async fn stop_session(&self, id: &str) -> anyhow::Result<Option<CaptureSession>> {
        let Some(mut session) = self.sessions.read().await.get(id).cloned() else {
            return Ok(None);
        };
        session.status = CaptureStatus::Stopping;
        self.sessions
            .write()
            .await
            .insert(id.to_string(), session.clone());

        self.send_unity_command(
            &session.project_path,
            CMD_STOP_LUX_STREAM,
            json!({ "sessionId": id }),
        )
        .await?;

        session.status = CaptureStatus::Stopped;
        self.sessions.write().await.remove(id);
        self.frame_txs.write().await.remove(id);
        Ok(Some(session))
    }

    pub async fn get_session(&self, id: &str) -> Option<CaptureSession> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn list_sessions(&self) -> Vec<CaptureSession> {
        let mut sessions = self
            .sessions
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        sessions
    }

    pub async fn receive_frame(
        &self,
        session_id: &str,
        frame_data: Vec<u8>,
        _sequence: u64,
    ) -> anyhow::Result<()> {
        let Some(sender) = self.frame_txs.read().await.get(session_id).cloned() else {
            bail!("capture session not found: {session_id}");
        };
        sender
            .send(Some(frame_data))
            .map_err(|_| anyhow::anyhow!("capture frame receiver closed: {session_id}"))
    }

    pub async fn receive_stream_frame(&self, frame: LuxStreamFrame) -> anyhow::Result<()> {
        let frame_data = STANDARD
            .decode(frame.frame.as_bytes())
            .context("lux stream frame was not valid base64")?;
        self.receive_frame(&frame.session_id, frame_data, frame.sequence)
            .await
    }

    pub async fn get_latest_frame(&self, session_id: &str) -> Option<Vec<u8>> {
        self.get_session(session_id)
            .await
            .and_then(|session| session.frame_rx.borrow().clone())
    }

    pub async fn frame_receiver(
        &self,
        session_id: &str,
    ) -> Option<watch::Receiver<Option<Vec<u8>>>> {
        self.get_session(session_id)
            .await
            .map(|session| session.frame_rx.clone())
    }

    pub async fn forward_input(
        &self,
        session_id: &str,
        input_event: InputEvent,
    ) -> anyhow::Result<()> {
        let session = self
            .get_session(session_id)
            .await
            .with_context(|| format!("capture session not found: {session_id}"))?;
        session
            .input_tx
            .send(input_event)
            .await
            .with_context(|| format!("capture input queue closed: {session_id}"))
    }

    pub async fn forward_lux_input(
        &self,
        session_id: &str,
        input_event: LuxInputEvent,
    ) -> anyhow::Result<()> {
        self.forward_input(session_id, InputEvent::from(input_event))
            .await
    }

    pub async fn replace_bridge_connection(&self, stream: TcpStream) {
        *self.bridge_connection.lock().await = Some(stream);
    }

    fn spawn_input_forwarder(
        &self,
        session_id: String,
        project_path: PathBuf,
        mut input_rx: mpsc::Receiver<InputEvent>,
    ) {
        let manager = self.clone();
        tokio::spawn(async move {
            while let Some(input_event) = input_rx.recv().await {
                let payload = json!({
                    "sessionId": session_id,
                    "type": input_event.event_type,
                    "x": input_event.x,
                    "y": input_event.y,
                    "button": input_event.button,
                    "key": input_event.key,
                    "delta": input_event.delta,
                });
                if let Err(error) = manager
                    .send_unity_command(&project_path, CMD_LUX_INPUT_EVENT, payload)
                    .await
                {
                    tracing::warn!(%error, %session_id, "failed to forward Lux capture input event");
                }
            }
        });
    }

    async fn send_unity_command(
        &self,
        project_path: &std::path::Path,
        command: &str,
        params: Value,
    ) -> anyhow::Result<()> {
        let discovery = read_unity_bridge_discovery(project_path).await?;
        let request_line = unity_request_line(command, discovery.token.as_str(), params)?;
        let mut stream = TcpStream::connect((discovery.host.as_str(), discovery.port))
            .await
            .with_context(|| {
                format!(
                    "failed to connect to Unity AI Bridge at {}:{}",
                    discovery.host, discovery.port
                )
            })?;
        stream
            .write_all(request_line.as_bytes())
            .await
            .context("Unity TCP request write failed")?;
        stream
            .flush()
            .await
            .context("Unity TCP request flush failed")?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .context("Unity TCP response read failed")?;
        if line.trim().is_empty() {
            return Ok(());
        }
        let response: Value = serde_json::from_str(line.trim_end())
            .context("Unity TCP response was not valid JSON")?;
        if response.get("command").and_then(Value::as_str) == Some(CMD_LUX_STREAM_FRAME) {
            if let Some(frame) = parse_lux_stream_frame_value(response)? {
                self.receive_stream_frame(frame).await?;
            }
            return Ok(());
        }
        if response.get("ok").and_then(Value::as_bool) == Some(false) {
            bail!("Unity backend rejected {command}: {response}");
        }
        Ok(())
    }

    async fn open_stream_reader(
        &self,
        project_path: &std::path::Path,
        request: StartLuxStreamRequest,
    ) -> anyhow::Result<()> {
        let discovery = read_unity_bridge_discovery(project_path).await?;
        let request_line = unity_request_line(
            CMD_START_LUX_STREAM,
            discovery.token.as_str(),
            serde_json::to_value(request)?,
        )?;
        let stream = TcpStream::connect((discovery.host.as_str(), discovery.port))
            .await
            .with_context(|| {
                format!(
                    "failed to connect to Unity AI Bridge at {}:{}",
                    discovery.host, discovery.port
                )
            })?;
        let (read_half, mut write_half) = stream.into_split();
        write_half
            .write_all(request_line.as_bytes())
            .await
            .context("Unity TCP stream start write failed")?;
        write_half
            .flush()
            .await
            .context("Unity TCP stream start flush failed")?;

        let manager = self.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim_end();
                        if trimmed.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<Value>(trimmed)
                            .context("Unity stream frame was not valid JSON")
                            .and_then(parse_lux_stream_frame_value)
                        {
                            Ok(Some(frame)) => {
                                if let Err(error) = manager.receive_stream_frame(frame).await {
                                    tracing::warn!(%error, "failed to store Lux stream frame");
                                }
                            }
                            Ok(None) => {}
                            Err(error) => {
                                tracing::warn!(%error, "ignored invalid Lux stream frame")
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Unity TCP stream frame read failed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

impl Default for CaptureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl From<LuxInputEvent> for InputEvent {
    fn from(event: LuxInputEvent) -> Self {
        Self {
            event_type: event.event_type,
            x: event.x.map(f64::from),
            y: event.y.map(f64::from),
            button: event.button.and_then(|button| u32::try_from(button).ok()),
            key: event.key_code,
            delta: None,
        }
    }
}

fn unity_request_line(command: &str, token: &str, params: Value) -> anyhow::Result<String> {
    let request = json!({
        "schemaVersion": 1,
        "requestId": Uuid::new_v4().to_string(),
        "command": command,
        "token": token,
        "params": params,
    });
    Ok(format!("{}\n", serde_json::to_string(&request)?))
}

pub fn parse_lux_stream_frame_value(value: Value) -> anyhow::Result<Option<LuxStreamFrame>> {
    if value.get("command").and_then(Value::as_str) != Some(CMD_LUX_STREAM_FRAME) {
        return Ok(None);
    }
    let payload = value
        .get("params")
        .cloned()
        .or_else(|| value.get("payload").cloned())
        .unwrap_or(value);
    serde_json::from_value(payload)
        .map(Some)
        .context("Unity lux_stream_frame payload was invalid")
}

async fn read_unity_bridge_discovery(
    project_path: &std::path::Path,
) -> anyhow::Result<UnityBridgeDiscovery> {
    let discovery_path = project_path.join("Library/UnityAiBridge/server.json");
    let text = std::fs::read_to_string(&discovery_path).with_context(|| {
        format!(
            "Unity AI Bridge discovery file not found at {}",
            discovery_path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse Unity AI Bridge discovery file at {}",
            discovery_path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::Arc};

    use serde_json::Value;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
        sync::Mutex,
    };

    async fn bridge_project() -> (PathBuf, Arc<Mutex<Vec<Value>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured_requests = requests.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let captured_requests = captured_requests.clone();
                tokio::spawn(async move {
                    let (read_half, mut write_half) = stream.into_split();
                    let mut reader = BufReader::new(read_half);
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let request: Value = serde_json::from_str(line.trim_end()).unwrap();
                    let command = request
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    captured_requests.lock().await.push(request.clone());
                    if command == CMD_START_LUX_STREAM {
                        let session_id = request["params"]["sessionId"].as_str().unwrap();
                        let response = json!({
                            "command": CMD_LUX_STREAM_FRAME,
                            "params": {
                                "sessionId": session_id,
                                "frame": STANDARD.encode([1_u8, 2, 3]),
                                "sequence": 1_u64,
                                "timestamp": "2026-05-10T00:00:00Z"
                            }
                        });
                        let _ = write_half
                            .write_all(
                                format!("{}\n", serde_json::to_string(&response).unwrap())
                                    .as_bytes(),
                            )
                            .await;
                    } else {
                        let _ = write_half.write_all(b"{\"ok\":true}\n").await;
                    }
                    let _ = write_half.flush().await;
                });
            }
        });

        let project_path = std::env::temp_dir().join(format!(
            "lux-capture-unit-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        let discovery_dir = project_path.join("Library/UnityAiBridge");
        fs::create_dir_all(&discovery_dir).unwrap();
        fs::write(
            discovery_dir.join("server.json"),
            json!({ "host": "127.0.0.1", "port": port, "token": "unit-token" }).to_string(),
        )
        .unwrap();
        (project_path, requests)
    }

    async fn seeded_manager() -> (CaptureManager, CaptureSession) {
        let manager = CaptureManager::new();
        let (frame_tx, frame_rx) = watch::channel(None);
        let (input_tx, _input_rx) = mpsc::channel(16);
        let session = CaptureSession {
            id: "session-1".to_string(),
            width: 640,
            height: 360,
            fps: 15,
            frame_rx,
            input_tx,
            created_at: Utc::now(),
            project_path: PathBuf::from("/tmp/lux-project"),
            status: CaptureStatus::Streaming,
        };
        manager
            .sessions
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        manager
            .frame_txs
            .write()
            .await
            .insert(session.id.clone(), frame_tx);
        (manager, session)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (project_path, requests) = bridge_project().await;
        let manager = CaptureManager::new();

        let session = manager
            .create_session(project_path.clone(), 800, 600, 24)
            .await
            .unwrap();

        assert_eq!(session.project_path, project_path);
        assert!(matches!(session.status, CaptureStatus::Streaming));
        assert_eq!(session.width, 800);
        assert_eq!(session.height, 600);
        assert_eq!(session.fps, 24);
        assert!(manager.get_session(&session.id).await.is_some());
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(requests.lock().await[0]["command"], CMD_START_LUX_STREAM);

        let _ = fs::remove_dir_all(project_path);
    }

    #[tokio::test]
    async fn test_stop_session() {
        let (project_path, requests) = bridge_project().await;
        let manager = CaptureManager::new();
        let session = manager
            .create_session(project_path.clone(), 320, 240, 10)
            .await
            .unwrap();

        let stopped = manager.stop_session(&session.id).await.unwrap().unwrap();

        assert_eq!(stopped.id, session.id);
        assert!(matches!(stopped.status, CaptureStatus::Stopped));
        assert!(manager.get_session(&session.id).await.is_none());
        assert!(manager.get_latest_frame(&session.id).await.is_none());
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(requests
            .lock()
            .await
            .iter()
            .any(|request| request["command"] == CMD_STOP_LUX_STREAM));

        let _ = fs::remove_dir_all(project_path);
    }

    #[tokio::test]
    async fn test_receive_frame_updates_watch_receiver() {
        let (manager, session) = seeded_manager().await;

        manager
            .receive_frame(&session.id, vec![9, 8, 7], 42)
            .await
            .unwrap();

        assert_eq!(
            manager.get_latest_frame(&session.id).await,
            Some(vec![9, 8, 7])
        );
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let (manager, session) = seeded_manager().await;

        let sessions = manager.list_sessions().await;

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session.id);
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let (project_path, requests) = bridge_project().await;
        let manager = CaptureManager::new();

        let first = manager
            .create_session(project_path.clone(), 640, 360, 15)
            .await
            .unwrap();
        let second = manager
            .create_session(project_path.clone(), 1920, 1080, 60)
            .await
            .unwrap();

        assert_ne!(first.id, second.id);
        assert_eq!(manager.get_session(&first.id).await.unwrap().width, 640);
        assert_eq!(manager.get_session(&second.id).await.unwrap().width, 1920);
        manager.receive_frame(&first.id, vec![1], 1).await.unwrap();
        manager.receive_frame(&second.id, vec![2], 1).await.unwrap();
        assert_eq!(manager.get_latest_frame(&first.id).await, Some(vec![1]));
        assert_eq!(manager.get_latest_frame(&second.id).await, Some(vec![2]));
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(
            requests
                .lock()
                .await
                .iter()
                .filter(|request| request["command"] == CMD_START_LUX_STREAM)
                .count(),
            2
        );

        let _ = fs::remove_dir_all(project_path);
    }
}
