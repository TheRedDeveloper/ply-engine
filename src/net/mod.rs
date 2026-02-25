mod http;
mod websocket;

pub use http::{HttpConfig, Request, Response};
pub use websocket::{WebSocket, WsConfig, WsMessage};

use http::HttpRequestState;
use websocket::WebSocketState;

use rustc_hash::FxHashMap;
use std::sync::{LazyLock, Mutex};

/// Global manager for all network operations.
#[allow(private_interfaces)]
pub static NET_MANAGER: LazyLock<Mutex<NetManager>> =
    LazyLock::new(|| Mutex::new(NetManager::new()));

/// Generic wrapper that tracks how many frames since last access.
pub(crate) struct Tracked<T> {
    pub(crate) frames_not_accessed: usize,
    pub(crate) state: T,
}

pub(crate) struct NetManager {
    pub(crate) http_requests: FxHashMap<u64, Tracked<HttpRequestState>>,
    pub(crate) websockets: FxHashMap<u64, Tracked<WebSocketState>>,
    /// Number of frames after which an unused closed response is evicted.
    pub max_frames_not_used: usize,
}

impl NetManager {
    fn new() -> Self {
        Self {
            http_requests: FxHashMap::default(),
            websockets: FxHashMap::default(),
            max_frames_not_used: 60,
        }
    }

    /// Frame-based eviction. Called once per frame from `eval()`.
    pub fn clean(&mut self) {
        self.http_requests.retain(|_, entry| {
            // Try to receive if still pending
            if let HttpRequestState::Pending(pending) = &mut entry.state {
                if let Some(result) = pending.try_recv() {
                    match result {
                        Ok(resp) => {
                            entry.state =
                                HttpRequestState::Done(std::sync::Arc::new(resp));
                        }
                        Err(e) => {
                            entry.state = HttpRequestState::Error(e);
                        }
                    }
                }
            }

            match &entry.state {
                HttpRequestState::Pending(_) => true, // never evict
                _ => {
                    entry.frames_not_accessed += 1;
                    entry.frames_not_accessed <= self.max_frames_not_used
                }
            }
        });

        self.websockets.retain(|_, entry| {
            entry.frames_not_accessed += 1;
            let disconnected = entry.state.is_disconnected();
            !(disconnected && entry.frames_not_accessed > self.max_frames_not_used)
        });
    }
}

fn hash_id(id: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    id.hash(&mut hasher);
    hasher.finish()
}

fn fire_http(
    method: &str,
    id: &str,
    url: &str,
    f: impl FnOnce(&mut HttpConfig) -> &mut HttpConfig,
) {
    let key = hash_id(id);
    let mut mgr = NET_MANAGER.lock().unwrap();

    // Idempotent: don't re-fire if a request with this ID already exists
    if mgr.http_requests.contains_key(&key) {
        return;
    }

    let mut config = HttpConfig::new();
    f(&mut config);

    #[cfg(not(target_arch = "wasm32"))]
    {
        use http::PendingHttp;

        let method = method.to_owned();
        let url = url.to_owned();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result: Result<Response, String> = (|| {
                let agent = ureq::Agent::new_with_defaults();

                macro_rules! apply_headers {
                    ($req:expr, $headers:expr) => {{
                        let mut r = $req;
                        for (key, value) in $headers {
                            r = r.header(key.as_str(), value.as_str());
                        }
                        r
                    }};
                }

                let send_result = match method.as_str() {
                    "GET" => {
                        let req = apply_headers!(agent.get(&url), &config.headers);
                        req.call()
                    }
                    "DELETE" => {
                        let req = apply_headers!(agent.delete(&url), &config.headers);
                        req.call()
                    }
                    "POST" => {
                        let req = apply_headers!(agent.post(&url), &config.headers);
                        if let Some(body) = &config.body {
                            req.content_type("application/octet-stream").send(body)
                        } else {
                            req.send_empty()
                        }
                    }
                    "PUT" => {
                        let req = apply_headers!(agent.put(&url), &config.headers);
                        if let Some(body) = &config.body {
                            req.content_type("application/octet-stream").send(body)
                        } else {
                            req.send_empty()
                        }
                    }
                    _ => return Err(format!("Unsupported HTTP method: {method}")),
                };

                match send_result {
                    Ok(resp) => {
                        let status: u16 = resp.status().into();
                        let body = resp
                            .into_body()
                            .read_to_vec()
                            .map_err(|e| e.to_string())?;
                        Ok(Response::new(status, body))
                    }
                    Err(e) => Err(e.to_string()),
                }
            })();

            let _ = tx.send(result);
        });

        mgr.http_requests.insert(
            key,
            Tracked {
                frames_not_accessed: 0,
                state: HttpRequestState::Pending(PendingHttp::new(rx)),
            },
        );
    }

    #[cfg(target_arch = "wasm32")]
    {
        use http::PendingHttp;
        use sapp_jsutils::JsObject;

        let scheme: i32 = match method {
            "GET" => 0,
            "POST" => 1,
            "PUT" => 2,
            "DELETE" => 3,
            _ => return,
        };

        let headers_obj = JsObject::object();
        for (key, value) in &config.headers {
            headers_obj.set_field_string(key, value);
        }

        let body_str = config
            .body
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).to_string())
            .unwrap_or_default();

        let cid = unsafe {
            http::ply_net_http_make_request(
                scheme,
                JsObject::string(url),
                JsObject::string(&body_str),
                headers_obj,
            )
        };

        mgr.http_requests.insert(
            key,
            Tracked {
                frames_not_accessed: 0,
                state: HttpRequestState::Pending(PendingHttp::new(cid)),
            },
        );
    }
}

/// Fire a GET request. Idempotent: won't re-fire if a request with this ID exists.
pub fn get(id: &str, url: &str, f: impl FnOnce(&mut HttpConfig) -> &mut HttpConfig) {
    fire_http("GET", id, url, f);
}

/// Fire a POST request. Idempotent: won't re-fire if a request with this ID exists.
pub fn post(id: &str, url: &str, f: impl FnOnce(&mut HttpConfig) -> &mut HttpConfig) {
    fire_http("POST", id, url, f);
}

/// Fire a PUT request. Idempotent: won't re-fire if a request with this ID exists.
pub fn put(id: &str, url: &str, f: impl FnOnce(&mut HttpConfig) -> &mut HttpConfig) {
    fire_http("PUT", id, url, f);
}

/// Fire a DELETE request. Idempotent: won't re-fire if a request with this ID exists.
pub fn delete(id: &str, url: &str, f: impl FnOnce(&mut HttpConfig) -> &mut HttpConfig) {
    fire_http("DELETE", id, url, f);
}

/// Get a handle to an existing HTTP request. Returns `None` if no such ID.
pub fn request(id: &str) -> Option<Request> {
    let key = hash_id(id);
    let mut mgr = NET_MANAGER.lock().unwrap();
    let entry = mgr.http_requests.get_mut(&key)?;
    entry.frames_not_accessed = 0;
    Some(Request { id: key })
}

/// Connect a WebSocket. Idempotent: won't reconnect if already open.
pub fn ws_connect(
    id: &str,
    url: &str,
    f: impl FnOnce(&mut WsConfig) -> &mut WsConfig,
) {
    let key = hash_id(id);
    let mut mgr = NET_MANAGER.lock().unwrap();

    if mgr.websockets.contains_key(&key) {
        return;
    }

    let mut config = WsConfig::new();
    f(&mut config);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let url = url.to_owned();
        let (incoming_tx, incoming_rx) = std::sync::mpsc::channel();
        let (outgoing_tx, mut outgoing_rx) = tokio::sync::mpsc::unbounded_channel();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for WebSocket");

        let insecure = config.insecure;
        let headers = config.headers;

        runtime.spawn(async move {
            use futures::{SinkExt, StreamExt};
            use tokio_tungstenite::tungstenite;
            use tungstenite::client::IntoClientRequest;

            // Build handshake request: start from URL to get proper WS headers,
            // then add custom headers on top.
            let mut ws_request = match url.into_client_request() {
                Ok(r) => r,
                Err(e) => {
                    let _ = incoming_tx.send(WsMessage::Error(e.to_string()));
                    return;
                }
            };
            for (key, value) in &headers {
                if let (Ok(name), Ok(val)) = (
                    tungstenite::http::header::HeaderName::from_bytes(key.as_bytes()),
                    tungstenite::http::header::HeaderValue::from_str(value),
                ) {
                    ws_request.headers_mut().insert(name, val);
                }
            }

            let socket = if insecure {
                let tls_config = {
                    let config = rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(std::sync::Arc::new(
                            NoCertificateVerification {},
                        ))
                        .with_no_client_auth();
                    std::sync::Arc::new(config)
                };
                let connector = tokio_tungstenite::Connector::Rustls(tls_config);
                tokio_tungstenite::connect_async_tls_with_config(
                    ws_request,
                    None,
                    true,
                    Some(connector),
                )
                .await
            } else {
                tokio_tungstenite::connect_async(ws_request).await
            };

            let (ws_stream, _response) = match socket {
                Ok(s) => s,
                Err(e) => {
                    let _ = incoming_tx.send(WsMessage::Error(e.to_string()));
                    return;
                }
            };

            let _ = incoming_tx.send(WsMessage::Connected);

            let (mut write_half, mut read_half) = ws_stream.split();

            // Read task
            let incoming_tx_read = incoming_tx.clone();
            tokio::spawn(async move {
                while let Some(msg) = read_half.next().await {
                    match msg {
                        Ok(tungstenite::Message::Binary(data)) => {
                            if incoming_tx_read
                                .send(WsMessage::Binary(data.into()))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(tungstenite::Message::Text(text)) => {
                            if incoming_tx_read
                                .send(WsMessage::Text(text.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(tungstenite::Message::Close(_)) => {
                            let _ = incoming_tx_read.send(WsMessage::Closed);
                            break;
                        }
                        Err(e) => {
                            let _ =
                                incoming_tx_read.send(WsMessage::Error(e.to_string()));
                            break;
                        }
                        _ => {}
                    }
                }
            });

            // Write task
            let incoming_tx_write = incoming_tx.clone();
            tokio::spawn(async move {
                use websocket::OutgoingWsMessage;
                while let Some(msg) = outgoing_rx.recv().await {
                    match msg {
                        OutgoingWsMessage::Text(text) => {
                            if let Err(e) = write_half
                                .send(tungstenite::Message::Text(text.into()))
                                .await
                            {
                                let _ = incoming_tx_write
                                    .send(WsMessage::Error(e.to_string()));
                                break;
                            }
                        }
                        OutgoingWsMessage::Binary(data) => {
                            if let Err(e) = write_half
                                .send(tungstenite::Message::Binary(data.into()))
                                .await
                            {
                                let _ = incoming_tx_write
                                    .send(WsMessage::Error(e.to_string()));
                                break;
                            }
                        }
                        OutgoingWsMessage::Close => {
                            let _ = incoming_tx_write.send(WsMessage::Closed);
                            let _ = write_half
                                .send(tungstenite::Message::Close(None))
                                .await;
                            break;
                        }
                    }
                }
            });
        });

        mgr.websockets.insert(
            key,
            Tracked {
                frames_not_accessed: 0,
                state: WebSocketState {
                    tx: outgoing_tx,
                    rx: incoming_rx,
                    _runtime: runtime,
                },
            },
        );
    }

    #[cfg(target_arch = "wasm32")]
    {
        use sapp_jsutils::JsObject;

        // JS bridge uses an integer socket ID; we use the lower 32 bits of the hash
        let socket_id = key as i32;

        unsafe {
            websocket::ply_net_ws_connect(socket_id, JsObject::string(url));
        }

        mgr.websockets.insert(
            key,
            Tracked {
                frames_not_accessed: 0,
                state: WebSocketState { socket_id },
            },
        );
    }
}

/// Get a handle to an existing WebSocket. Returns `None` if no such ID.
pub fn ws(id: &str) -> Option<WebSocket> {
    let key = hash_id(id);
    let mut mgr = NET_MANAGER.lock().unwrap();
    let entry = mgr.websockets.get_mut(&key)?;
    entry.frames_not_accessed = 0;
    Some(WebSocket { id: key })
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct NoCertificateVerification;

#[cfg(not(target_arch = "wasm32"))]
impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}
