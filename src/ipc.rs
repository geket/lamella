//! IPC (Inter-Process Communication) system
//!
//! Implements an i3-compatible IPC protocol for external control and
//! integration with tools like i3status, polybar, etc.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, error, info, warn};

/// IPC message types (i3-compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MessageType {
    /// Run a command
    RunCommand = 0,
    /// Get workspaces
    GetWorkspaces = 1,
    /// Subscribe to events
    Subscribe = 2,
    /// Get outputs
    GetOutputs = 3,
    /// Get layout tree
    GetTree = 4,
    /// Get marks
    GetMarks = 5,
    /// Get bar config
    GetBarConfig = 6,
    /// Get version
    GetVersion = 7,
    /// Get binding modes
    GetBindingModes = 8,
    /// Get config
    GetConfig = 9,
    /// Tick (heartbeat)
    Tick = 10,
    /// Sync (block until complete)
    Sync = 11,
    /// Get binding state
    GetBindingState = 12,
    /// Get inputs
    GetInputs = 100,
    /// Get seats
    GetSeats = 101,
}

impl MessageType {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::RunCommand),
            1 => Some(Self::GetWorkspaces),
            2 => Some(Self::Subscribe),
            3 => Some(Self::GetOutputs),
            4 => Some(Self::GetTree),
            5 => Some(Self::GetMarks),
            6 => Some(Self::GetBarConfig),
            7 => Some(Self::GetVersion),
            8 => Some(Self::GetBindingModes),
            9 => Some(Self::GetConfig),
            10 => Some(Self::Tick),
            11 => Some(Self::Sync),
            12 => Some(Self::GetBindingState),
            100 => Some(Self::GetInputs),
            101 => Some(Self::GetSeats),
            _ => None,
        }
    }
}

/// Event types for subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Workspace,
    Output,
    Mode,
    Window,
    BarConfigUpdate,
    Binding,
    Shutdown,
    Tick,
    BarStateUpdate,
    Input,
}

/// IPC message header
/// Format: "i3-ipc" (6 bytes) + length (4 bytes) + type (4 bytes) + payload
const IPC_MAGIC: &[u8; 6] = b"i3-ipc";

/// An IPC message
#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub message_type: MessageType,
    pub payload: String,
}

impl IpcMessage {
    pub fn new(message_type: MessageType, payload: impl Into<String>) -> Self {
        Self {
            message_type,
            payload: payload.into(),
        }
    }

    /// Serialize message to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let payload_bytes = self.payload.as_bytes();
        let mut bytes = Vec::with_capacity(14 + payload_bytes.len());
        
        // Magic
        bytes.extend_from_slice(IPC_MAGIC);
        // Length (little-endian)
        bytes.extend_from_slice(&(payload_bytes.len() as u32).to_le_bytes());
        // Type (little-endian)
        bytes.extend_from_slice(&(self.message_type as u32).to_le_bytes());
        // Payload
        bytes.extend_from_slice(payload_bytes);
        
        bytes
    }

    /// Parse message from bytes
    pub fn from_reader(reader: &mut impl Read) -> Result<Self> {
        // Read magic
        let mut magic = [0u8; 6];
        reader.read_exact(&mut magic)
            .context("Failed to read IPC magic")?;
        
        if &magic != IPC_MAGIC {
            anyhow::bail!("Invalid IPC magic: {:?}", magic);
        }

        // Read length
        let mut length_bytes = [0u8; 4];
        reader.read_exact(&mut length_bytes)
            .context("Failed to read message length")?;
        let length = u32::from_le_bytes(length_bytes) as usize;

        // Read type
        let mut type_bytes = [0u8; 4];
        reader.read_exact(&mut type_bytes)
            .context("Failed to read message type")?;
        let msg_type = u32::from_le_bytes(type_bytes);
        
        let message_type = MessageType::from_u32(msg_type)
            .ok_or_else(|| anyhow::anyhow!("Unknown message type: {}", msg_type))?;

        // Read payload
        let mut payload_bytes = vec![0u8; length];
        reader.read_exact(&mut payload_bytes)
            .context("Failed to read payload")?;
        let payload = String::from_utf8(payload_bytes)
            .context("Invalid UTF-8 in payload")?;

        Ok(Self { message_type, payload })
    }
}

/// Command result
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            success: true,
            parse_error: None,
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            parse_error: None,
            error: Some(msg.into()),
        }
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            parse_error: Some(true),
            error: Some(msg.into()),
        }
    }
}

/// Workspace info for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: u64,
    pub num: i32,
    pub name: String,
    pub visible: bool,
    pub focused: bool,
    pub urgent: bool,
    pub output: String,
    pub rect: Rect,
}

/// Output info for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputInfo {
    pub name: String,
    pub active: bool,
    pub primary: bool,
    pub rect: Rect,
    pub current_workspace: Option<String>,
}

/// Rectangle for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Tree node for IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub id: u64,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub node_type: String,
    pub border: String,
    pub current_border_width: i32,
    pub layout: String,
    pub orientation: String,
    pub rect: Rect,
    pub window_rect: Rect,
    pub deco_rect: Rect,
    pub geometry: Rect,
    pub urgent: bool,
    pub sticky: bool,
    pub focused: bool,
    pub focus: Vec<u64>,
    pub nodes: Vec<TreeNode>,
    pub floating_nodes: Vec<TreeNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
}

/// Version info
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub human_readable: String,
    pub loaded_config_file_name: String,
}

/// IPC event
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "change")]
pub enum IpcEvent {
    // Workspace events
    #[serde(rename = "init")]
    WorkspaceInit { current: WorkspaceInfo },
    #[serde(rename = "empty")]
    WorkspaceEmpty { current: WorkspaceInfo },
    #[serde(rename = "focus")]
    WorkspaceFocus { 
        current: WorkspaceInfo,
        old: Option<WorkspaceInfo>,
    },
    #[serde(rename = "move")]
    WorkspaceMove { current: WorkspaceInfo },
    #[serde(rename = "rename")]
    WorkspaceRename { current: WorkspaceInfo },
    #[serde(rename = "urgent")]
    WorkspaceUrgent { current: WorkspaceInfo },
    #[serde(rename = "reload")]
    WorkspaceReload,

    // Window events
    #[serde(rename = "new")]
    WindowNew { container: TreeNode },
    #[serde(rename = "close")]
    WindowClose { container: TreeNode },
    #[serde(rename = "focus")]
    WindowFocus { container: TreeNode },
    #[serde(rename = "title")]
    WindowTitle { container: TreeNode },
    #[serde(rename = "fullscreen_mode")]
    WindowFullscreen { container: TreeNode },
    #[serde(rename = "move")]
    WindowMove { container: TreeNode },
    #[serde(rename = "floating")]
    WindowFloating { container: TreeNode },
    #[serde(rename = "urgent")]
    WindowUrgent { container: TreeNode },
    #[serde(rename = "mark")]
    WindowMark { container: TreeNode },

    // Mode event
    #[serde(rename = "default")]
    ModeChange { pango_markup: bool },

    // Shutdown event
    #[serde(rename = "exit")]
    Shutdown,

    // Binding event
    #[serde(rename = "run")]
    Binding { 
        command: String,
        event_state_mask: Vec<String>,
        input_code: u32,
        symbol: Option<String>,
        input_type: String,
    },
}

/// IPC client connection
struct IpcClient {
    stream: UnixStream,
    subscriptions: Vec<EventType>,
}

impl IpcClient {
    fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            subscriptions: Vec::new(),
        }
    }

    fn send(&mut self, message: &IpcMessage) -> Result<()> {
        self.stream.write_all(&message.to_bytes())?;
        Ok(())
    }

    fn send_json(&mut self, msg_type: MessageType, value: &Value) -> Result<()> {
        let message = IpcMessage::new(msg_type, serde_json::to_string(value)?);
        self.send(&message)
    }
}

/// IPC request from client
pub struct IpcRequest {
    pub message: IpcMessage,
    pub response_sender: Sender<IpcMessage>,
}

/// IPC server
pub struct IpcServer {
    socket_path: PathBuf,
    request_sender: Sender<IpcRequest>,
    event_sender: Sender<(EventType, Value)>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new(socket_path: impl AsRef<Path>) -> Result<(Self, Receiver<IpcRequest>, Receiver<(EventType, Value)>)> {
        let socket_path = socket_path.as_ref().to_path_buf();
        let (request_sender, request_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();

        Ok((
            Self {
                socket_path,
                request_sender,
                event_sender,
            },
            request_receiver,
            event_receiver,
        ))
    }

    /// Start the IPC server
    pub fn start(self) -> Result<()> {
        // Remove existing socket
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind IPC socket: {:?}", self.socket_path))?;

        info!("IPC server listening on {:?}", self.socket_path);

        // Accept connections
        let request_sender = self.request_sender.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let sender = request_sender.clone();
                        thread::spawn(move || {
                            if let Err(e) = handle_client(stream, sender) {
                                debug!("IPC client disconnected: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept IPC connection: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Send an event to all subscribed clients
    pub fn send_event(&self, event_type: EventType, event: Value) {
        let _ = self.event_sender.send((event_type, event));
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

fn handle_client(mut stream: UnixStream, request_sender: Sender<IpcRequest>) -> Result<()> {
    loop {
        let message = IpcMessage::from_reader(&mut stream)?;
        debug!("Received IPC message: {:?}", message.message_type);

        let (response_tx, response_rx) = mpsc::channel();
        request_sender.send(IpcRequest {
            message,
            response_sender: response_tx,
        })?;

        // Wait for response
        let response = response_rx.recv()?;
        stream.write_all(&response.to_bytes())?;
    }
}

/// Handle IPC requests
pub struct IpcHandler {
    receiver: Receiver<IpcRequest>,
}

impl IpcHandler {
    pub fn new(receiver: Receiver<IpcRequest>) -> Self {
        Self { receiver }
    }

    /// Try to receive a request (non-blocking)
    pub fn try_recv(&self) -> Option<IpcRequest> {
        self.receiver.try_recv().ok()
    }

    /// Create a response for a request
    pub fn respond(request: &IpcRequest, response: IpcMessage) {
        let _ = request.response_sender.send(response);
    }

    /// Create a JSON response
    pub fn respond_json(request: &IpcRequest, msg_type: MessageType, value: &Value) {
        let response = IpcMessage::new(msg_type, serde_json::to_string(value).unwrap_or_default());
        Self::respond(request, response);
    }

    /// Handle a command request
    pub fn handle_command(request: &IpcRequest, result: CommandResult) {
        let value = json!([result]);
        Self::respond_json(request, MessageType::RunCommand, &value);
    }

    /// Handle get_version request
    pub fn handle_version(request: &IpcRequest) {
        let version = VersionInfo {
            major: 0,
            minor: 1,
            patch: 0,
            human_readable: format!("fluxway {}", env!("CARGO_PKG_VERSION")),
            loaded_config_file_name: String::new(),
        };
        Self::respond_json(request, MessageType::GetVersion, &serde_json::to_value(version).unwrap());
    }

    /// Handle get_binding_modes request
    pub fn handle_binding_modes(request: &IpcRequest, modes: &[String]) {
        Self::respond_json(request, MessageType::GetBindingModes, &json!(modes));
    }

    /// Handle get_marks request
    pub fn handle_marks(request: &IpcRequest, marks: &[String]) {
        Self::respond_json(request, MessageType::GetMarks, &json!(marks));
    }

    /// Handle subscribe request
    pub fn handle_subscribe(request: &IpcRequest, events: &[EventType]) -> Vec<EventType> {
        // Parse requested events from payload
        let requested: Vec<String> = serde_json::from_str(&request.message.payload)
            .unwrap_or_default();
        
        let mut subscribed = Vec::new();
        for event_name in requested {
            if let Ok(event_type) = serde_json::from_value::<EventType>(json!(event_name)) {
                subscribed.push(event_type);
            }
        }

        let result = json!({ "success": true });
        Self::respond_json(request, MessageType::Subscribe, &result);
        
        subscribed
    }
}

/// IPC client for sending commands to fluxway
pub struct IpcClient2 {
    socket_path: PathBuf,
}

impl IpcClient2 {
    pub fn connect(socket_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        })
    }

    /// Send a command and get response
    pub fn send_command(&self, command: &str) -> Result<Vec<CommandResult>> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        
        let message = IpcMessage::new(MessageType::RunCommand, command);
        stream.write_all(&message.to_bytes())?;
        
        let response = IpcMessage::from_reader(&mut stream)?;
        let results: Vec<CommandResult> = serde_json::from_str(&response.payload)?;
        
        Ok(results)
    }

    /// Get workspaces
    pub fn get_workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        
        let message = IpcMessage::new(MessageType::GetWorkspaces, "");
        stream.write_all(&message.to_bytes())?;
        
        let response = IpcMessage::from_reader(&mut stream)?;
        let workspaces: Vec<WorkspaceInfo> = serde_json::from_str(&response.payload)?;
        
        Ok(workspaces)
    }

    /// Get tree
    pub fn get_tree(&self) -> Result<TreeNode> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        
        let message = IpcMessage::new(MessageType::GetTree, "");
        stream.write_all(&message.to_bytes())?;
        
        let response = IpcMessage::from_reader(&mut stream)?;
        let tree: TreeNode = serde_json::from_str(&response.payload)?;
        
        Ok(tree)
    }

    /// Get version
    pub fn get_version(&self) -> Result<VersionInfo> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        
        let message = IpcMessage::new(MessageType::GetVersion, "");
        stream.write_all(&message.to_bytes())?;
        
        let response = IpcMessage::from_reader(&mut stream)?;
        let version: VersionInfo = serde_json::from_str(&response.payload)?;
        
        Ok(version)
    }

    /// Subscribe to events
    pub fn subscribe(&self, events: &[EventType]) -> Result<UnixStream> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        
        let events_json = serde_json::to_string(events)?;
        let message = IpcMessage::new(MessageType::Subscribe, events_json);
        stream.write_all(&message.to_bytes())?;
        
        // Read subscription confirmation
        let response = IpcMessage::from_reader(&mut stream)?;
        let result: Value = serde_json::from_str(&response.payload)?;
        
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            anyhow::bail!("Subscription failed");
        }
        
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_serialization() {
        let message = IpcMessage::new(MessageType::RunCommand, "kill");
        let bytes = message.to_bytes();
        
        assert_eq!(&bytes[0..6], IPC_MAGIC);
        assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 4); // "kill" length
        assert_eq!(u32::from_le_bytes(bytes[10..14].try_into().unwrap()), 0); // RunCommand
    }

    #[test]
    fn test_command_result() {
        let success = CommandResult::success();
        assert!(success.success);
        
        let error = CommandResult::error("test error");
        assert!(!error.success);
        assert_eq!(error.error.as_deref(), Some("test error"));
    }
}
