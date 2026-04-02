use crate::detect;
use crate::rewrite;
use std::io::Read;
use std::path::PathBuf;

fn lock_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pt_disabled")
}

pub fn disable() {
    let path = lock_path();
    let _ = std::fs::write(&path, "");
    eprintln!("Prompt Tuner disabled. Run `pt --enable` to re-enable.");
}

pub fn enable() {
    let path = lock_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    eprintln!("Prompt Tuner enabled.");
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Check kill switch
    if lock_path().exists() {
        return Ok(());
    }

    let mut input = String::new();
    std::io::stdin()
        .take(1_000_000)
        .read_to_string(&mut input)?;

    let json: serde_json::Value =
        serde_json::from_str(&input).map_err(|e| format!("Failed to parse hook input: {e}"))?;

    let prompt = json["prompt"].as_str().unwrap_or("");

    if prompt.is_empty() {
        return Ok(());
    }

    let detection = detect::analyze(prompt);

    if !detection.is_vague {
        return Ok(());
    }

    // Prompt is vague — generate a suggestion
    let cwd = json["cwd"].as_str().unwrap_or(".");

    let suggestion = rewrite::rewrite(prompt, cwd).unwrap_or_else(|_| rewrite::template(prompt));

    let reason = format!(
        "Prompt Tuner: Your prompt could be more effective.\n\n\
         Suggested rewrite:\n\
         {suggestion}\n\n\
         Edit your prompt above and resubmit. Run `pt --disable` to turn off."
    );

    let output = serde_json::json!({
        "decision": "block",
        "reason": reason,
    });

    println!("{output}");
    Ok(())
}
