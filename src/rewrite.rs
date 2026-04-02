use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

/// Call Claude CLI to rewrite a vague prompt into a more effective one.
/// Times out after 10 seconds, returning an error.
pub fn rewrite(prompt: &str, cwd: &str) -> Result<String, String> {
    let prompt = prompt.to_string();
    let cwd = cwd.to_string();

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = call_claude(&prompt, &cwd);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(TIMEOUT) {
        Ok(result) => result,
        Err(_) => Err("Rewrite timed out".to_string()),
    }
}

fn call_claude(prompt: &str, cwd: &str) -> Result<String, String> {
    let instruction = format!(
        "You are a prompt refinement tool. The user submitted a vague prompt while coding in: {cwd}\n\n\
         Rewrite their prompt to be specific, actionable, and useful for an AI coding assistant. \
         Infer what they likely mean from context clues.\n\n\
         Rules:\n\
         - Output ONLY the rewritten prompt, nothing else\n\
         - Keep it concise but specific\n\
         - Use [brackets] for details the user needs to fill in\n\
         - Preserve the user's intent and tone (don't add unnecessary formality)\n\
         - If it's about a bug, include placeholders for: file, error, steps to reproduce\n\
         - If it's about a feature, include placeholders for: scope, behavior, location\n\n\
         Original prompt: {prompt}"
    );

    let output = Command::new("claude")
        .arg("-p")
        .arg("--model")
        .arg("haiku")
        .arg("--fallback-model")
        .arg("sonnet")
        .arg(&instruction)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "Claude CLI not found".to_string()
            } else {
                format!("Failed to run claude: {e}")
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude error: {stderr}"));
    }

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if result.is_empty() {
        return Err("Empty rewrite result".to_string());
    }

    Ok(result)
}

/// Generate a static template when the API rewrite isn't available.
pub fn template(prompt: &str) -> String {
    let lower = prompt.to_lowercase();

    if lower.contains("bug")
        || lower.contains("broken")
        || lower.contains("error")
        || lower.contains("fail")
        || lower.contains("wrong")
        || lower.contains("not working")
        || lower.contains("doesn't work")
    {
        "The [specific bug/error] in [file/component] is still occurring. \
         Error message: [paste error]. Steps to reproduce: [steps]. \
         Previous fix attempt: [what was tried]. Expected: [expected behavior]."
            .to_string()
    } else if lower.contains("add")
        || lower.contains("feature")
        || lower.contains("implement")
        || lower.contains("create")
        || lower.contains("build")
    {
        "Add [feature name] to [file/component]. It should [describe behavior]. \
         Requirements: [list requirements]. Similar to [reference if any]."
            .to_string()
    } else {
        "I need help with [specific task] in [file/component]. \
         Context: [what you're working on]. Current state: [what's happening]. \
         Goal: [what you want to achieve]."
            .to_string()
    }
}
