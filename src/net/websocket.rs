#[cfg(target_arch = "wasm32")]
use sapp_jsutils::JsObject;

/// Configuration builder passed to the WebSocket connect closure.
pub struct WsConfig {
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) insecure: bool,
}

impl WsConfig {
    pub(crate) fn new() -> Self {
        Self {
            headers: Vec::new(),
            insecure: false,
        }
    }

    /// Add a header to the connection handshake.
    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.push((key.to_owned(), value.to_owned()));
        self
    }

    /// Disable TLS certificate verification (for dev servers).
    /// Note: has no effect on WASM — browser handles TLS.
    pub fn insecure(&mut self) -> &mut Self {
        self.insecure = true;
        self
    }
}

/// A message received from a WebSocket.
#[derive(Debug, Clone)]
pub enum WsMessage {
    /// Connection established.
    Connected,
    /// Text frame received.
    Text(String),
    /// Binary frame received.
    Binary(Vec<u8>),
    /// An error occurred.
    Error(String),
    /// The connection was closed.
    Closed,
}

/// Outgoing message sent to the background task.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) enum OutgoingWsMessage {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

/// A thin handle to a WebSocket tracked by the global manager.
///
/// Does not own the data — just a key for lookups.
pub struct WebSocket {
    pub(crate) id: u64,
}

impl WebSocket {
    /// Send binary data.
    pub fn send(&self, data: &[u8]) {
        let mgr = super::NET_MANAGER.lock().unwrap();
        if let Some(entry) = mgr.websockets.get(&self.id) {
            entry.state.send_binary(data);
        }
    }

    /// Send a text message.
    pub fn send_text(&self, text: &str) {
        let mgr = super::NET_MANAGER.lock().unwrap();
        if let Some(entry) = mgr.websockets.get(&self.id) {
            entry.state.send_text(text);
        }
    }

    /// Pop the next incoming message.
    ///
    /// Use `while let Some(msg) = ws.recv()` to drain all messages
    /// that arrived since the last frame.
    pub fn recv(&self) -> Option<WsMessage> {
        let mut mgr = super::NET_MANAGER.lock().unwrap();
        let entry = mgr.websockets.get_mut(&self.id)?;
        entry.frames_not_accessed = 0;
        entry.state.try_recv()
    }

    /// Graceful close. Consumes the handle and removes the entry immediately.
    pub fn close(self) {
        let mut mgr = super::NET_MANAGER.lock().unwrap();
        if let Some(entry) = mgr.websockets.get(&self.id) {
            entry.state.close();
        }
        mgr.websockets.remove(&self.id);
    }
}

/// Internal state for a tracked WebSocket (native).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct WebSocketState {
    pub tx: tokio::sync::mpsc::UnboundedSender<OutgoingWsMessage>,
    pub rx: std::sync::mpsc::Receiver<WsMessage>,
    pub _runtime: tokio::runtime::Runtime,
}

#[cfg(not(target_arch = "wasm32"))]
impl WebSocketState {
    pub fn send_binary(&self, data: &[u8]) {
        let _ = self.tx.send(OutgoingWsMessage::Binary(data.to_vec()));
    }

    pub fn send_text(&self, text: &str) {
        let _ = self.tx.send(OutgoingWsMessage::Text(text.to_owned()));
    }

    pub fn try_recv(&self) -> Option<WsMessage> {
        self.rx.try_recv().ok()
    }

    pub fn close(&self) {
        let _ = self.tx.send(OutgoingWsMessage::Close);
    }

    pub fn is_disconnected(&self) -> bool {
        self.tx.is_closed()
    }
}

/// Internal state for a tracked WebSocket (WASM).
#[cfg(target_arch = "wasm32")]
pub(crate) struct WebSocketState {
    /// The integer socket ID used by the JS bridge.
    pub socket_id: i32,
}

#[cfg(target_arch = "wasm32")]
impl WebSocketState {
    pub fn send_binary(&self, data: &[u8]) {
        unsafe {
            ply_net_ws_send_binary(self.socket_id, JsObject::buffer(data));
        }
    }

    pub fn send_text(&self, text: &str) {
        unsafe {
            ply_net_ws_send_text(self.socket_id, JsObject::string(text));
        }
    }

    pub fn try_recv(&self) -> Option<WsMessage> {
        let js_obj = unsafe { ply_net_ws_try_recv(self.socket_id) };
        if js_obj.is_nil() {
            return None;
        }

        let type_id = js_obj.field_u32("type");
        match type_id {
            0 => Some(WsMessage::Connected),   // PLY_WS_CONNECTED
            1 => {                              // PLY_WS_BINARY
                let mut buf = Vec::new();
                js_obj.field("data").to_byte_buffer(&mut buf);
                Some(WsMessage::Binary(buf))
            }
            2 => {                              // PLY_WS_TEXT
                let mut text = String::new();
                js_obj.field("data").to_string(&mut text);
                Some(WsMessage::Text(text))
            }
            3 => {                              // PLY_WS_ERROR
                let mut err = String::new();
                js_obj.field("data").to_string(&mut err);
                Some(WsMessage::Error(err))
            }
            4 => Some(WsMessage::Closed),       // PLY_WS_CLOSED
            _ => None,
        }
    }

    pub fn close(&self) {
        unsafe {
            ply_net_ws_close(self.socket_id);
        }
    }

    /// On WASM, we don't have a channel — we consider it disconnected
    /// if the JS side has removed the socket (close was called).
    pub fn is_disconnected(&self) -> bool {
        // After close() is called, the JS entry is deleted.
        // We rely on the immediate removal in WebSocket::close(self).
        false
    }
}

// WASM FFI
#[cfg(target_arch = "wasm32")]
extern "C" {
    pub(crate) fn ply_net_ws_connect(socket_id: i32, addr: JsObject);
    fn ply_net_ws_send_binary(socket_id: i32, data: JsObject);
    fn ply_net_ws_send_text(socket_id: i32, text: JsObject);
    fn ply_net_ws_close(socket_id: i32);
    fn ply_net_ws_try_recv(socket_id: i32) -> JsObject;
}
