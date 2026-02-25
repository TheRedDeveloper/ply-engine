use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use sapp_jsutils::JsObject;

/// Configuration builder passed to the HTTP request closure.
pub struct HttpConfig {
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<Vec<u8>>,
}

impl HttpConfig {
    pub(crate) fn new() -> Self {
        Self {
            headers: Vec::new(),
            body: None,
        }
    }

    /// Add a header to the request.
    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.push((key.to_owned(), value.to_owned()));
        self
    }

    /// Set the request body as a string.
    pub fn body(&mut self, body: &str) -> &mut Self {
        self.body = Some(body.as_bytes().to_vec());
        self
    }

    /// Set the request body as raw bytes.
    pub fn body_bytes(&mut self, body: Vec<u8>) -> &mut Self {
        self.body = Some(body);
        self
    }
}

/// A completed HTTP response.
pub struct Response {
    status: u16,
    body: Vec<u8>,
}

impl Response {
    pub(crate) fn new(status: u16, body: Vec<u8>) -> Self {
        Self { status, body }
    }

    /// HTTP status code.
    pub fn status(&self) -> u16 {
        self.status
    }

    /// Response body as a UTF-8 string (lossy).
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap_or("")
    }

    /// Response body as raw bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.body
    }

    /// Deserialize the response body as JSON.
    #[cfg(feature = "net-json")]
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, String> {
        serde_json::from_slice(&self.body).map_err(|e| e.to_string())
    }
}

/// A thin handle to an HTTP request tracked by the global manager.
///
/// Does not own the data — just a key for lookups.
pub struct Request {
    pub(crate) id: u64,
}

impl Request {
    /// Check for a completed response.
    ///
    /// - `None`: still pending
    /// - `Some(Ok(resp))`: completed successfully (`Arc<Response>`)
    /// - `Some(Err(e))`: request failed
    pub fn response(&self) -> Option<Result<Arc<Response>, String>> {
        let mut mgr = super::NET_MANAGER.lock().unwrap();
        let entry = mgr.http_requests.get_mut(&self.id)?;
        entry.frames_not_accessed = 0;

        // Try to transition Pending → Done/Error
        match &mut entry.state {
            HttpRequestState::Pending(pending) => {
                if let Some(result) = pending.try_recv() {
                    match result {
                        Ok(resp) => {
                            entry.state = HttpRequestState::Done(Arc::new(resp));
                        }
                        Err(e) => {
                            entry.state = HttpRequestState::Error(e);
                        }
                    }
                }
            }
            _ => {}
        }

        match &entry.state {
            HttpRequestState::Pending(_) => None,
            HttpRequestState::Done(resp) => Some(Ok(Arc::clone(resp))),
            HttpRequestState::Error(e) => Some(Err(e.clone())),
        }
    }

    /// Cancel and remove a pending request. Consumes the handle.
    pub fn cancel(self) {
        let mut mgr = super::NET_MANAGER.lock().unwrap();
        mgr.http_requests.remove(&self.id);
    }
}

/// Abstraction over the pending receive mechanism.
pub(crate) struct PendingHttp {
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::mpsc::Receiver<Result<Response, String>>,
    #[cfg(target_arch = "wasm32")]
    cid: i32,
}

impl PendingHttp {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(rx: std::sync::mpsc::Receiver<Result<Response, String>>) -> Self {
        Self { rx }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new(cid: i32) -> Self {
        Self { cid }
    }

    /// Try to receive a completed response. Returns None if still pending.
    pub fn try_recv(&mut self) -> Option<Result<Response, String>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.rx.try_recv().ok()
        }

        #[cfg(target_arch = "wasm32")]
        {
            let js_obj = unsafe { ply_net_http_try_recv(self.cid) };
            if js_obj.is_nil() {
                return None;
            }

            // Check for error field
            if js_obj.have_field("error") {
                let mut error_str = String::new();
                js_obj.field("error").to_string(&mut error_str);
                if !error_str.is_empty() {
                    return Some(Err(error_str));
                }
            }

            let status = js_obj.field_u32("status") as u16;
            let mut body = Vec::new();
            js_obj.field("body").to_byte_buffer(&mut body);
            Some(Ok(Response::new(status, body)))
        }
    }
}

/// Internal state for a tracked HTTP request.
pub(crate) enum HttpRequestState {
    /// In flight — response hasn't arrived yet.
    Pending(PendingHttp),
    /// Response arrived.
    Done(Arc<Response>),
    /// Request failed.
    Error(String),
}

// WASM FFI
#[cfg(target_arch = "wasm32")]
extern "C" {
    pub(crate) fn ply_net_http_make_request(
        scheme: i32,
        url: JsObject,
        body: JsObject,
        headers: JsObject,
    ) -> i32;
    fn ply_net_http_try_recv(cid: i32) -> JsObject;
}
