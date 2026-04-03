use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(15);

/// Call Claude CLI to rewrite a vague prompt into a more effective one.
/// Times out after 10 seconds, returning an error.
pub fn rewrite(prompt: &str, cwd: &str, transcript_path: Option<&str>) -> Result<String, String> {
    let prompt = prompt.to_string();
    let cwd = cwd.to_string();
    let context = transcript_path
        .map(|p| read_recent_context(p))
        .unwrap_or_default();

    let (tx, rx) = mpsc::channel();

    // Track the child PID so we can kill it on timeout
    let pid = std::sync::Arc::new(std::sync::Mutex::new(None::<u32>));
    let pid_clone = pid.clone();

    std::thread::spawn(move || {
        let result = call_claude(&prompt, &cwd, &context, &pid_clone);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(TIMEOUT) {
        Ok(result) => result,
        Err(_) => {
            // Kill the child process if still running
            if let Some(child_pid) = pid.lock().ok().and_then(|g| *g) {
                #[cfg(unix)]
                unsafe {
                    libc::kill(child_pid as i32, libc::SIGKILL);
                }
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/PID", &child_pid.to_string()])
                        .output();
                }
            }
            Err("Rewrite timed out".to_string())
        }
    }
}

/// Read the last few human/assistant exchanges from the transcript for context.
fn read_recent_context(path: &str) -> String {
    // Only read transcripts from Claude Code's data directory
    let path_buf = std::path::Path::new(path);
    if let Some(home) = dirs::home_dir() {
        let claude_dir = home.join(".claude");
        if !path_buf.starts_with(&claude_dir) {
            return String::new();
        }
    }

    // Cap reads at 10 MB to avoid OOM on huge transcripts
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return String::new(),
    };
    if meta.len() > 10 * 1024 * 1024 {
        // Read only the last 1 MB for recent context
        use std::io::{Read, Seek, SeekFrom};
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return String::new(),
        };
        let start = meta.len().saturating_sub(1024 * 1024);
        let _ = file.seek(SeekFrom::Start(start));
        let mut content = String::new();
        let _ = file.read_to_string(&mut content);
        return parse_exchanges(&content);
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    parse_exchanges(&content)
}

fn parse_exchanges(content: &str) -> String {

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

fn call_claude(
    prompt: &str,
    cwd: &str,
    conversation_context: &str,
    pid: &std::sync::Mutex<Option<u32>>,
) -> Result<String, String> {
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
         - Keep it to 1-2 sentences max. Be dense, not verbose.\n\
         - ONLY if all of these are true: there is no conversation context, the prompt \
         has zero clues about what the user wants, and you cannot make even a rough guess \
         — then output exactly the word PUNT and nothing else. Otherwise, always expand.\n\n\
         Original prompt: {prompt}"
    );

    let child = Command::new("claude")
        .arg("-p")
        .arg("--model")
        .arg("haiku")
        .arg("--fallback-model")
        .arg("sonnet")
        .arg(&instruction)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "Claude CLI not found".to_string()
            } else {
                format!("Failed to run claude: {e}")
            }
        })?;

    // Store PID so parent can kill on timeout
    if let Ok(mut guard) = pid.lock() {
        *guard = Some(child.id());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for claude: {e}"))?;

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

