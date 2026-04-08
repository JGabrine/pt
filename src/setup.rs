use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("settings.json")
}

/// Ensure the binary lives at ~/.local/bin/pt so the hook survives
/// even if the original build directory is deleted.
fn ensure_stable_binary() -> Result<String, String> {
    let stable = bin_path();
    let current = std::env::current_exe()
        .map_err(|e| format!("Could not determine binary path: {e}"))?;

    // Already running from the stable location
    if current == stable {
        return Ok(stable.to_string_lossy().to_string());
    }

    // Copy ourselves to ~/.local/bin/pt
    if let Some(parent) = stable.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    std::fs::copy(&current, &stable)
        .map_err(|e| format!("Could not copy binary to {}: {e}", stable.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&stable, std::fs::Permissions::from_mode(0o755));
    }

    eprintln!("Installed binary to {}", stable.display());
    Ok(stable.to_string_lossy().to_string())
}

fn hook_command() -> Result<String, String> {
    Ok(format!("{} --hook", ensure_stable_binary()?))
}

fn has_pt_hook(entry: &serde_json::Value) -> bool {
    // Check inside the hooks array of a matcher entry
    entry
        .get("hooks")
        .and_then(|h| h.as_array())
        .is_some_and(|hooks| {
            hooks.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c.contains("pt") && c.contains("--hook"))
            })
        })
}

pub fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let path = settings_path();
    let command = hook_command()?;

    // Read existing settings or start fresh
    let mut settings: serde_json::Value = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Remove any existing PT hook entries (may have stale paths)
    if let Some(entries) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("UserPromptSubmit"))
        .and_then(|u| u.as_array_mut())
    {
        entries.retain(|entry| !has_pt_hook(entry));
    }

    // Build the hook entry with matcher + hooks array
    let hook_entry = serde_json::json!({
        "matcher": "",
        "hooks": [
            {
                "type": "command",
                "command": command
            }
        ]
    });

    // Merge into settings
    let hooks = settings
        .as_object_mut()
        .unwrap()
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}));

    let user_prompt = hooks
        .as_object_mut()
        .ok_or("hooks is not an object")?
        .entry("UserPromptSubmit")
        .or_insert_with(|| serde_json::json!([]));

    user_prompt
        .as_array_mut()
        .ok_or("UserPromptSubmit is not an array")?
        .push(hook_entry);

    // Ensure .claude directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write back
    let formatted = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&path, formatted)?;

    eprintln!("Prompt Tuner registered in {}", path.display());
    eprintln!("Hook command: {command}");
    eprintln!("\nRestart Claude Code for the hook to take effect.");
    Ok(())
}

pub fn uninstall() -> Result<(), Box<dyn std::error::Error>> {
    let path = settings_path();

    if !path.exists() {
        eprintln!("No settings file found. Nothing to remove.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&path)?;
    let mut settings: serde_json::Value = serde_json::from_str(&content)?;

    // Find and remove pt hook entries
    let removed = if let Some(entries) = settings
        .get_mut("hooks")
        .and_then(|h| h.get_mut("UserPromptSubmit"))
        .and_then(|u| u.as_array_mut())
    {
        let before = entries.len();
        entries.retain(|entry| !has_pt_hook(entry));
        before - entries.len()
    } else {
        0
    };

    if removed == 0 {
        eprintln!("Prompt Tuner hook not found in settings.");
        return Ok(());
    }

    let formatted = serde_json::to_string_pretty(&settings)?;
    std::fs::write(&path, formatted)?;

    eprintln!("Prompt Tuner removed from {}", path.display());
    Ok(())
}

fn install_dir() -> PathBuf {
    if cfg!(windows) {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pt")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("share")
            .join("pt")
    }
}

fn bin_path() -> PathBuf {
    let bin_name = if cfg!(windows) { "pt.exe" } else { "pt" };
    if cfg!(windows) {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pt")
            .join(bin_name)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("bin")
            .join(bin_name)
    }
}

/// Find the git repository containing PT's source code.
pub fn find_repo_dir() -> Option<PathBuf> {
    let dir = install_dir();
    if dir.join(".git").exists() {
        return Some(dir);
    }

    let exe = std::env::current_exe().ok()?;
    let mut search = exe.parent().map(|p| p.to_path_buf());
    loop {
        match search {
            Some(ref p) if p.join(".git").exists() => return Some(p.clone()),
            Some(ref p) => search = p.parent().map(|p| p.to_path_buf()),
            None => return None,
        }
    }
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join(".cache")
        })
        .join("pt")
}

fn read_cached_remote(path: &Path) -> Option<String> {
    let age = std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .and_then(|t| std::time::SystemTime::now().duration_since(t).ok())?;

    if age > Duration::from_secs(4 * 3600) {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

/// Check if a newer version is available. Non-blocking: reads from cache
/// and spawns a background refresh if stale.
pub fn update_available() -> bool {
    let repo = match find_repo_dir() {
        Some(r) => r,
        None => return false,
    };

    let local = Command::new("git")
        .arg("-C").arg(&repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if local.is_empty() {
        return false;
    }

    let cache = cache_dir();
    let cache_file = cache.join("remote_head");

    // Read cached remote HEAD (returns None if stale or missing)
    let remote = read_cached_remote(&cache_file);

    // Spawn background refresh if cache is stale
    if remote.is_none() {
        let _ = std::fs::create_dir_all(&cache);
        #[cfg(unix)]
        {
            let cmd = format!(
                "git -C '{}' ls-remote origin HEAD 2>/dev/null | cut -f1 > '{}'",
                repo.display(),
                cache_file.display()
            );
            let _ = Command::new("sh")
                .args(["-c", &cmd])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }

    match remote {
        Some(ref r) => !r.is_empty() && *r != local,
        None => false,
    }
}

pub fn update() -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = match find_repo_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("Could not find pt repository. Reinstall with:");
            if cfg!(windows) {
                eprintln!("  irm https://raw.githubusercontent.com/JGabrine/pt/main/install.ps1 | iex");
            } else {
                eprintln!("  curl -fsSL https://raw.githubusercontent.com/JGabrine/pt/main/install.sh | sh");
            }
            return Ok(());
        }
    };

    eprintln!("Updating from {}...", repo_dir.display());

    // Snapshot current HEAD before pulling
    let old_head = Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    // Pull latest
    let pull = Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args(["pull", "--ff-only"])
        .status()?;

    if !pull.success() {
        return Err("git pull failed".into());
    }

    // Show what changed
    let new_head = Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    if old_head == new_head {
        eprintln!("Already up to date.");
        return Ok(());
    }

    // Print changelog
    let range = format!("{old_head}..{new_head}");
    eprintln!("\nChanges:");
    let _ = Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args(["log", "--oneline", &range])
        .status();
    eprintln!();

    // Rebuild
    eprintln!("Building...");
    let build = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--manifest-path")
        .arg(repo_dir.join("Cargo.toml"))
        .status()?;

    if !build.success() {
        return Err("cargo build failed".into());
    }

    // Copy binary to stable location
    let bin_name = if cfg!(windows) { "pt.exe" } else { "pt" };
    let built = repo_dir.join("target").join("release").join(bin_name);
    let dest = bin_path();
    if dest.exists() && dest != built {
        // Remove first to avoid "Text file busy" — the running process keeps its
        // file descriptor, but the path is freed for the new binary.
        let _ = std::fs::remove_file(&dest);
        std::fs::copy(&built, &dest)?;
        eprintln!("Updated binary at {}", dest.display());
    }

    eprintln!("Done.");
    Ok(())
}
