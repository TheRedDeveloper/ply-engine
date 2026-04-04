#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
use macroquad::prelude::next_frame;
#[cfg(target_arch = "wasm32")]
use sapp_jsutils::JsObject;

#[derive(Debug, Clone)]
pub struct Storage {
    #[cfg(not(target_arch = "wasm32"))]
    root_path: PathBuf,
    #[cfg(target_arch = "wasm32")]
    root_id: i32,
}

impl Storage {
    pub async fn new(path: &str) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let candidate = PathBuf::from(path);
            let root_path = if candidate.is_absolute() {
                candidate
            } else {
                let normalized_root = normalize_relative_path(path, "Storage::new path")?;
                let app_data_dir = platform_app_data_dir()?;
                join_normalized_path(&app_data_dir, &normalized_root)
            };

            std::fs::create_dir_all(&root_path).map_err(|e| e.to_string())?;

            Ok(Self { root_path })
        }

        #[cfg(target_arch = "wasm32")]
        {
            let normalized_root = normalize_relative_path(path, "Storage::new path")?;
            let op_id = unsafe { ply_storage_new(JsObject::string(&normalized_root)) };
            let result = wait_for_response(op_id).await?;
            ensure_success(&result)?;

            Ok(Self {
                root_id: result.field_u32("storage_id") as i32,
            })
        }
    }

    pub async fn save_string(&self, path: &str, data: &str) -> Result<(), String> {
        self.save_bytes(path, data.as_bytes()).await
    }

    pub async fn save_bytes(&self, path: &str, data: &[u8]) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let full_path = self.resolve_path(path)?;
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(full_path, data).map_err(|e| e.to_string())?;
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let normalized_path = normalize_relative_path(path, "storage save path")?;
            let op_id = unsafe {
                ply_storage_save_bytes(
                    self.root_id,
                    JsObject::string(&normalized_path),
                    JsObject::buffer(data),
                )
            };
            let result = wait_for_response(op_id).await?;
            ensure_success(&result)
        }
    }

    pub async fn load_string(&self, path: &str) -> Result<Option<String>, String> {
        match self.load_bytes(path).await? {
            Some(bytes) => {
                let content = String::from_utf8(bytes)
                    .map_err(|e| format!("Invalid UTF-8 data: {e}"))?;
                Ok(Some(content))
            }
            None => Ok(None),
        }
    }

    pub async fn load_bytes(&self, path: &str) -> Result<Option<Vec<u8>>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let full_path = self.resolve_path(path)?;
            match std::fs::read(full_path) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.to_string()),
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let normalized_path = normalize_relative_path(path, "storage load path")?;
            let op_id = unsafe {
                ply_storage_load_bytes(self.root_id, JsObject::string(&normalized_path))
            };
            let result = wait_for_response(op_id).await?;
            ensure_success(&result)?;

            if result.field_u32("exists") == 0 {
                return Ok(None);
            }

            let mut bytes = Vec::new();
            result.field("data").to_byte_buffer(&mut bytes);
            Ok(Some(bytes))
        }
    }

    pub async fn remove(&self, path: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let full_path = self.resolve_path(path)?;
            std::fs::remove_file(full_path).map_err(|e| e.to_string())?;
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let normalized_path = normalize_relative_path(path, "storage remove path")?;
            let op_id = unsafe {
                ply_storage_remove(self.root_id, JsObject::string(&normalized_path))
            };
            let result = wait_for_response(op_id).await?;
            ensure_success(&result)
        }
    }

    pub async fn export(&self, path: &str) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let full_path = self.resolve_path(path)?;
            let bytes = std::fs::read(&full_path).map_err(|e| e.to_string())?;

            let normalized_path = normalize_relative_path(path, "storage export path")?;
            let file_name = normalized_path
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Invalid export file name".to_owned())?;

            let target_path = rfd::FileDialog::new()
                .set_file_name(file_name)
                .save_file()
                .ok_or_else(|| "Export canceled".to_owned())?;

            std::fs::write(target_path, bytes).map_err(|e| e.to_string())?;
            Ok(())
        }

        #[cfg(target_arch = "wasm32")]
        {
            let normalized_path = normalize_relative_path(path, "storage export path")?;
            let op_id = unsafe {
                ply_storage_export(self.root_id, JsObject::string(&normalized_path))
            };
            let result = wait_for_response(op_id).await?;
            ensure_success(&result)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_path(&self, relative_path: &str) -> Result<PathBuf, String> {
        let normalized = normalize_relative_path(relative_path, "storage file path")?;
        Ok(join_normalized_path(&self.root_path, &normalized))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn join_normalized_path(root: &Path, normalized: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in normalized.split('/') {
        path.push(part);
    }
    path
}

fn normalize_relative_path(path: &str, what: &str) -> Result<String, String> {
    let trimmed = path.trim();

    if trimmed.is_empty() {
        return Err(format!("{what} cannot be empty"));
    }

    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err(format!("{what} must be a relative path"));
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(format!("{what} must be a relative path"));
    }

    let mut parts: Vec<&str> = Vec::new();
    for part in trimmed.split(|c| c == '/' || c == '\\') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(format!("{what} cannot contain '..'"));
        }
        parts.push(part);
    }

    if parts.is_empty() {
        return Err(format!("{what} is invalid"));
    }

    Ok(parts.join("/"))
}

#[cfg(not(target_arch = "wasm32"))]
fn platform_app_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(appdata));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            return Ok(PathBuf::from(home).join("AppData").join("Roaming"));
        }
        return Err("Could not resolve %APPDATA% on Windows".to_owned());
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support"));
        }
        return Err("Could not resolve HOME on macOS".to_owned());
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(xdg_data_home) = std::env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg_data_home));
        }
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home).join(".local").join("share"));
        }
        return Err("Could not resolve data directory on Linux".to_owned());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home).join(".local").join("share"));
        }
        Err("Could not resolve platform app data directory".to_owned())
    }
}

#[cfg(target_arch = "wasm32")]
fn ensure_success(response: &JsObject) -> Result<(), String> {
    if response.field_u32("status") == 1 {
        return Ok(());
    }

    let mut error_message = String::new();
    if response.have_field("error") {
        response.field("error").to_string(&mut error_message);
    }
    if error_message.is_empty() {
        error_message = "Storage operation failed".to_owned();
    }

    Err(error_message)
}

#[cfg(target_arch = "wasm32")]
async fn wait_for_response(op_id: i32) -> Result<JsObject, String> {
    loop {
        let result = unsafe { ply_storage_try_recv(op_id) };
        if !result.is_nil() {
            return Ok(result);
        }
        next_frame().await;
    }
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn ply_storage_new(path: JsObject) -> i32;
    fn ply_storage_save_bytes(storage_id: i32, path: JsObject, data: JsObject) -> i32;
    fn ply_storage_load_bytes(storage_id: i32, path: JsObject) -> i32;
    fn ply_storage_remove(storage_id: i32, path: JsObject) -> i32;
    fn ply_storage_export(storage_id: i32, path: JsObject) -> i32;
    fn ply_storage_try_recv(op_id: i32) -> JsObject;
}
