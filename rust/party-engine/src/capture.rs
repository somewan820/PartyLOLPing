//! macOS screen capture Adapter.
//!
//! `screencapture` is the system-owned ScreenCaptureKit/TCC entry point. The
//! backend receives an encoded JPEG frame; no Swift/AppKit capture layer is
//! required.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn capture_screen_jpeg() -> Result<Vec<u8>, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let path = PathBuf::from(format!("/tmp/partylolping-{stamp}.jpg"));
    let result = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "jpg"])
        .arg(&path)
        .output()
        .map_err(|error| format!("screen capture launch failed: {error}"))?;
    if !result.status.success() {
        let _ = std::fs::remove_file(&path);
        return Err(String::from_utf8_lossy(&result.stderr).trim().to_owned());
    }
    let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(path);
    if bytes.is_empty() {
        return Err("screen capture returned an empty frame".to_owned());
    }
    Ok(bytes)
}
