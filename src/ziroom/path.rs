use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use super::BrowserError;

pub(super) fn resolve_browser_path(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return executable_from_path(path).ok_or_else(|| {
            anyhow!(
                "configured browser executable does not exist: {}",
                path.display()
            )
        });
    }
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Helium.app/Contents/MacOS/Helium",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files\Chromium\Application\chrome.exe",
            r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ]
    } else {
        &[
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/usr/bin/brave-browser",
            "/usr/bin/microsoft-edge",
        ]
    };
    candidates
        .iter()
        .map(Path::new)
        .find_map(executable_from_path)
        .ok_or_else(|| BrowserError::BrowserNotFound.into())
}

fn executable_from_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }
    if cfg!(target_os = "macos") && path.extension().is_some_and(|ext| ext == "app") {
        let name = path.file_stem()?;
        let executable = path.join("Contents/MacOS").join(name);
        return executable.is_file().then_some(executable);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_real_explicit_executable() {
        let path = std::env::current_exe().unwrap();
        assert_eq!(resolve_browser_path(Some(&path)).unwrap(), path);
    }

    #[test]
    fn rejects_a_missing_explicit_executable() {
        let path = Path::new("/definitely/not/a/browser");
        assert!(resolve_browser_path(Some(path)).is_err());
    }
}
