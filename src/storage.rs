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
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
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

        #[cfg(target_os = "android")]
        {
            let full_path = self.resolve_path(path)?;
            std::fs::metadata(&full_path).map_err(|e| e.to_string())?;

            let normalized_path = normalize_relative_path(path, "storage export path")?;
            let file_name = normalized_path
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Invalid export file name".to_owned())?;

            let full_path_string = full_path.to_string_lossy().into_owned();
            let source_path_c =
                std::ffi::CString::new(full_path_string).map_err(|_| {
                    "Export path contains unsupported NUL byte".to_owned()
                })?;
            let file_name_c = std::ffi::CString::new(file_name).map_err(|_| {
                "Export file name contains unsupported NUL byte".to_owned()
            })?;
            let mime_type_c = std::ffi::CString::new(guess_mime_type(&normalized_path))
                .map_err(|_| "Export MIME type contains unsupported NUL byte".to_owned())?;

            unsafe {
                let env = macroquad::miniquad::native::android::attach_jni_env();
                let activity = macroquad::miniquad::native::android::ACTIVITY;
                if activity.is_null() {
                    return Err("Android activity is not available".to_owned());
                }

                let get_object_class = (**env).GetObjectClass.unwrap();
                let get_method_id = (**env).GetMethodID.unwrap();
                let call_void_method = (**env).CallVoidMethod.unwrap();
                let new_string_utf = (**env).NewStringUTF.unwrap();
                let delete_local_ref = (**env).DeleteLocalRef.unwrap();
                let exception_check = (**env).ExceptionCheck.unwrap();
                let exception_describe = (**env).ExceptionDescribe.unwrap();
                let exception_clear = (**env).ExceptionClear.unwrap();

                let class = get_object_class(env, activity);
                if class.is_null() {
                    return Err("Failed to access Android activity class".to_owned());
                }

                let method_name = std::ffi::CString::new("exportFile")
                    .map_err(|_| "Invalid Android method name".to_owned())?;
                let method_sig = std::ffi::CString::new(
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                )
                .map_err(|_| "Invalid Android method signature".to_owned())?;
                let method_id = get_method_id(
                    env,
                    class,
                    method_name.as_ptr(),
                    method_sig.as_ptr(),
                );
                if method_id.is_null() {
                    delete_local_ref(env, class as _);
                    return Err(
                        "MainActivity.exportFile(String,String,String) was not found"
                            .to_owned(),
                    );
                }

                let j_source_path = new_string_utf(env, source_path_c.as_ptr());
                let j_file_name = new_string_utf(env, file_name_c.as_ptr());
                let j_mime_type = new_string_utf(env, mime_type_c.as_ptr());
                if j_source_path.is_null() || j_file_name.is_null() || j_mime_type.is_null() {
                    if !j_source_path.is_null() {
                        delete_local_ref(env, j_source_path as _);
                    }
                    if !j_file_name.is_null() {
                        delete_local_ref(env, j_file_name as _);
                    }
                    if !j_mime_type.is_null() {
                        delete_local_ref(env, j_mime_type as _);
                    }
                    delete_local_ref(env, class as _);
                    return Err("Failed to allocate Android export strings".to_owned());
                }

                call_void_method(
                    env,
                    activity,
                    method_id,
                    j_source_path,
                    j_file_name,
                    j_mime_type,
                );

                if exception_check(env) != 0 {
                    exception_describe(env);
                    exception_clear(env);
                    delete_local_ref(env, j_source_path as _);
                    delete_local_ref(env, j_file_name as _);
                    delete_local_ref(env, j_mime_type as _);
                    delete_local_ref(env, class as _);
                    return Err(
                        "Android export failed to open the document picker"
                            .to_owned(),
                    );
                }

                delete_local_ref(env, j_source_path as _);
                delete_local_ref(env, j_file_name as _);
                delete_local_ref(env, j_mime_type as _);
                delete_local_ref(env, class as _);
            }

            Ok(())
        }

        #[cfg(target_os = "ios")]
        {
            use macroquad::miniquad::native::apple::apple_util::str_to_nsstring;
            use macroquad::miniquad::native::apple::frameworks::{
                class, msg_send, nil, NSRect, ObjcId,
            };

            let full_path = self.resolve_path(path)?;
            std::fs::metadata(&full_path).map_err(|e| e.to_string())?;

            let view_ctrl = macroquad::miniquad::window::apple_view_ctrl();
            if view_ctrl.is_null() {
                return Err("iOS view controller is not available".to_owned());
            }

            let full_path_string = full_path.to_string_lossy().into_owned();

            unsafe {
                let ns_path = str_to_nsstring(&full_path_string);
                let file_url: ObjcId = msg_send![class!(NSURL), fileURLWithPath: ns_path];
                if file_url.is_null() {
                    return Err("Failed to create iOS file URL for export".to_owned());
                }

                let items: ObjcId = msg_send![class!(NSMutableArray), arrayWithObject: file_url];
                let activity: ObjcId = msg_send![class!(UIActivityViewController), alloc];
                let activity: ObjcId = msg_send![
                    activity,
                    initWithActivityItems: items
                    applicationActivities: nil
                ];
                if activity.is_null() {
                    return Err("Failed to create iOS share sheet".to_owned());
                }

                // iPad requires an anchor for popovers.
                let popover: ObjcId = msg_send![activity, popoverPresentationController];
                if !popover.is_null() {
                    let view: ObjcId = msg_send![view_ctrl, view];
                    let bounds: NSRect = msg_send![view, bounds];
                    let _: () = msg_send![popover, setSourceView: view];
                    let _: () = msg_send![popover, setSourceRect: bounds];
                }

                let _: () = msg_send![
                    view_ctrl,
                    presentViewController: activity
                    animated: true
                    completion: nil
                ];
            }

            Ok(())
        }

        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(
                target_os = "linux",
                target_os = "macos",
                target_os = "windows",
                target_os = "android",
                target_os = "ios"
            ))
        ))]
        {
            let _ = path;
            Err("Storage::export is not supported on this platform yet".to_owned())
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

#[cfg(target_os = "android")]
fn guess_mime_type(path: &str) -> &'static str {
    let extension = path
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "js" => "application/javascript",
        "wasm" => "application/wasm",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
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

    #[cfg(target_os = "android")]
    {
        unsafe {
            let env = macroquad::miniquad::native::android::attach_jni_env();
            let activity = macroquad::miniquad::native::android::ACTIVITY;

            if activity.is_null() {
                return Err("Android activity is not available".to_owned());
            }

            let get_object_class = (**env).GetObjectClass.unwrap();
            let get_method_id = (**env).GetMethodID.unwrap();
            let call_object_method = (**env).CallObjectMethod.unwrap();
            let get_string_utf_chars = (**env).GetStringUTFChars.unwrap();
            let release_string_utf_chars = (**env).ReleaseStringUTFChars.unwrap();
            let delete_local_ref = (**env).DeleteLocalRef.unwrap();
            let exception_check = (**env).ExceptionCheck.unwrap();
            let exception_describe = (**env).ExceptionDescribe.unwrap();
            let exception_clear = (**env).ExceptionClear.unwrap();

            let class = get_object_class(env, activity);
            if class.is_null() {
                return Err("Failed to access Android activity class".to_owned());
            }

            let get_files_dir_name = std::ffi::CString::new("getFilesDir")
                .map_err(|_| "Invalid Android method name".to_owned())?;
            let get_files_dir_sig = std::ffi::CString::new("()Ljava/io/File;")
                .map_err(|_| "Invalid Android method signature".to_owned())?;
            let get_files_dir = get_method_id(
                env,
                class,
                get_files_dir_name.as_ptr(),
                get_files_dir_sig.as_ptr(),
            );

            if get_files_dir.is_null() {
                delete_local_ref(env, class as _);
                return Err("Failed to resolve Activity.getFilesDir()".to_owned());
            }

            let file_obj = call_object_method(env, activity, get_files_dir);
            if exception_check(env) != 0 || file_obj.is_null() {
                if exception_check(env) != 0 {
                    exception_describe(env);
                    exception_clear(env);
                }
                delete_local_ref(env, class as _);
                return Err("Failed to call Activity.getFilesDir()".to_owned());
            }

            let file_class = get_object_class(env, file_obj);
            if file_class.is_null() {
                delete_local_ref(env, file_obj as _);
                delete_local_ref(env, class as _);
                return Err("Failed to access java.io.File class".to_owned());
            }

            let get_abs_name = std::ffi::CString::new("getAbsolutePath")
                .map_err(|_| "Invalid Android method name".to_owned())?;
            let get_abs_sig = std::ffi::CString::new("()Ljava/lang/String;")
                .map_err(|_| "Invalid Android method signature".to_owned())?;
            let get_abs = get_method_id(
                env,
                file_class,
                get_abs_name.as_ptr(),
                get_abs_sig.as_ptr(),
            );

            if get_abs.is_null() {
                delete_local_ref(env, file_class as _);
                delete_local_ref(env, file_obj as _);
                delete_local_ref(env, class as _);
                return Err("Failed to resolve File.getAbsolutePath()".to_owned());
            }

            let path_obj = call_object_method(env, file_obj, get_abs);
            if exception_check(env) != 0 || path_obj.is_null() {
                if exception_check(env) != 0 {
                    exception_describe(env);
                    exception_clear(env);
                }
                delete_local_ref(env, file_class as _);
                delete_local_ref(env, file_obj as _);
                delete_local_ref(env, class as _);
                return Err("Failed to call File.getAbsolutePath()".to_owned());
            }

            let path_chars = get_string_utf_chars(env, path_obj as _, std::ptr::null_mut());
            if path_chars.is_null() {
                delete_local_ref(env, path_obj as _);
                delete_local_ref(env, file_class as _);
                delete_local_ref(env, file_obj as _);
                delete_local_ref(env, class as _);
                return Err("Failed to read app files directory string".to_owned());
            }

            let path = std::ffi::CStr::from_ptr(path_chars)
                .to_string_lossy()
                .into_owned();

            release_string_utf_chars(env, path_obj as _, path_chars);
            delete_local_ref(env, path_obj as _);
            delete_local_ref(env, file_class as _);
            delete_local_ref(env, file_obj as _);
            delete_local_ref(env, class as _);

            return Ok(PathBuf::from(path));
        }
    }

    #[cfg(target_os = "ios")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(home).join("Documents"));
        }
        if let Some(tmpdir) = std::env::var_os("TMPDIR") {
            return Ok(PathBuf::from(tmpdir));
        }
        return Err("Could not resolve app data directory on iOS".to_owned());
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
