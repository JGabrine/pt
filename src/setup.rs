use std::path::PathBuf;

fn settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("settings.json")
}

fn binary_path() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Could not determine binary path: {e}"))
}

fn hook_command() -> Result<String, String> {
    Ok(format!("{} --hook", binary_path()?))
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

    // Check if hook is already registered
    if let Some(entries) = settings
        .get("hooks")
        .and_then(|h| h.get("UserPromptSubmit"))
        .and_then(|u| u.as_array())
    {
        if entries.iter().any(has_pt_hook) {
            eprintln!("Prompt Tuner is already registered.");
            return Ok(());
        }
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
