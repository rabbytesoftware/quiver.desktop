//! Window screenshots by shelling out to the platform's capture tool.
//!
//! GPUI 0.2.2 cannot read back its own frame buffer, so the bridge asks the OS: ImageMagick `import` on the
//! window under X11 (found by title, falling back to the whole display), `screencapture` on macOS and a
//! PowerShell `CopyFromScreen` on Windows. Where the OS tool is missing the call fails with a clear message.

use std::process::Command;

pub fn capture_window() -> Result<Vec<u8>, String> {
    #[cfg(target_os = "linux")]
    {
        linux()
    }
    #[cfg(target_os = "macos")]
    {
        macos()
    }
    #[cfg(target_os = "windows")]
    {
        windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err("screenshots are not supported on this platform".into())
    }
}

#[cfg(target_os = "linux")]
fn linux() -> Result<Vec<u8>, String> {
    let pid = std::process::id().to_string();
    let target = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--pid", &pid])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().last().map(str::to_string))
        .unwrap_or_else(|| "root".to_string());
    let out = Command::new("import")
        .args(["-window", &target, "png:-"])
        .output()
        .map_err(|e| format!("ImageMagick `import` is required for screenshots: {e}"))?;
    if out.status.success() && !out.stdout.is_empty() {
        Ok(super::screenshot_pages::with_pages(&target, out.stdout))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[cfg(target_os = "macos")]
fn macos() -> Result<Vec<u8>, String> {
    let path = std::env::temp_dir().join(format!("quiver-shot-{}.png", std::process::id()));
    let status = Command::new("screencapture")
        .args(["-x", "-t", "png"])
        .arg(&path)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(
            "screencapture failed (grant Screen Recording permission to the terminal)".into(),
        );
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&path);
    bytes
}

#[cfg(target_os = "windows")]
fn windows() -> Result<Vec<u8>, String> {
    let path = std::env::temp_dir().join(format!("quiver-shot-{}.png", std::process::id()));
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms,System.Drawing; \
         $b=[System.Windows.Forms.Screen]::PrimaryScreen.Bounds; \
         $bmp=New-Object System.Drawing.Bitmap $b.Width,$b.Height; \
         [System.Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); \
         $bmp.Save('{}')",
        path.display()
    );
    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("powershell screenshot failed".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&path);
    bytes
}
