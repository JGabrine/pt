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

    if prompt.is_empty() || prompt.trim_start().starts_with("pt ") {
        return Ok(());
    }

    let detection = detect::analyze(prompt);

    if !detection.is_vague {
        return Ok(());
    }

    // Prompt is vague — expand it and inject as context
    let cwd = json["cwd"].as_str().unwrap_or(".");
    let transcript_path = json["transcript_path"].as_str();

    let expanded = match rewrite::rewrite(prompt, cwd, transcript_path) {
        Ok(rewrite) if rewrite.trim() != "PUNT" => rewrite,
        _ => {
            // Haiku can't determine intent or failed — let Opus handle it raw
            return Ok(());
        }
    };

    let context = format!(
        "The user's prompt was brief. Here is a more detailed interpretation \
         of what they likely need:\n\n\
         {expanded}\n\n\
         Use this interpretation to provide a helpful response. \
         Respond in the same language the user wrote in. \
         IMPORTANT: Start your response with exactly this line, in italics:\n\
         *[Prompt Tuner: {expanded}]*\n\
         Then continue with your normal response below it."
    );

    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": context
        }
    });

    println!("{output}");
    Ok(())
}
