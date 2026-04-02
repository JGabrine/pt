use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(10);

/// Call Claude CLI to rewrite a vague prompt into a more effective one.
/// Times out after 10 seconds, returning an error.
pub fn rewrite(prompt: &str, cwd: &str, transcript_path: Option<&str>) -> Result<String, String> {
    let prompt = prompt.to_string();
    let cwd = cwd.to_string();
    let context = transcript_path
        .map(|p| read_recent_context(p))
        .unwrap_or_default();

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = call_claude(&prompt, &cwd, &context);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(TIMEOUT) {
        Ok(result) => result,
        Err(_) => Err("Rewrite timed out".to_string()),
    }
}

/// Read the last few human/assistant exchanges from the transcript for context.
fn read_recent_context(path: &str) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut exchanges: Vec<String> = Vec::new();

    for line in content.lines().rev() {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        let msg_type = json["type"].as_str().unwrap_or("");
        if msg_type != "human" && msg_type != "assistant" {
            continue;
        }

        let role = if msg_type == "human" { "User" } else { "Assistant" };
        let message = &json["message"];
        let content = &message["content"];

        let text = if let Some(s) = content.as_str() {
            s.to_string()
        } else if let Some(arr) = content.as_array() {
            arr.iter()
                .filter_map(|b| {
                    if b["type"].as_str() == Some("text") {
                        b["text"].as_str().map(|t| t.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            continue;
        };

        if text.trim().is_empty() {
            continue;
        }

        // Truncate long messages to keep Haiku's input lean
        let truncated = if text.len() > 300 {
            format!("{}...", &text[..300])
        } else {
            text
        };

        exchanges.push(format!("{role}: {truncated}"));

        // Last 6 messages is enough context
        if exchanges.len() >= 6 {
            break;
        }
    }

    exchanges.reverse();
    exchanges.join("\n\n")
}

fn call_claude(prompt: &str, cwd: &str, conversation_context: &str) -> Result<String, String> {
    let context_section = if conversation_context.is_empty() {
        String::new()
    } else {
        format!(
            "Recent conversation for context:\n\
             ---\n\
             {conversation_context}\n\
             ---\n\n"
        )
    };

    let instruction = format!(
        "You are a prompt refinement tool. The user submitted a vague prompt while coding in: {cwd}\n\n\
         {context_section}\
         Expand their prompt into a detailed interpretation that an AI coding assistant can act on. \
         Use the conversation context to understand what they're referring to.\n\n\
         Rules:\n\
         - Output ONLY the expanded interpretation, nothing else\n\
         - Respond in the SAME LANGUAGE as the original prompt\n\
         - Use the conversation history to ground your interpretation in what was actually being discussed\n\
         - If it's about a bug, reference the specific bug/file/component from the conversation\n\
         - If it's about a feature, reference the specific feature being worked on\n\
         - Don't use [brackets] or placeholders — make your best guess based on context\n\
         - Keep the user's tone and energy\n\
         - Keep it to 1-2 sentences max. Be dense, not verbose.\n\n\
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
