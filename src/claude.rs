use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Refine,
    Why,
}

pub enum Message {
    Chunk(String),
    Done,
    Error(String),
}

pub async fn run_claude(input: &str, mode: Mode, tx: mpsc::UnboundedSender<Message>) {
    let prompt = match mode {
        Mode::Refine => format!(
            "You are a prompt engineering expert. Analyze the following prompt, \
             infer its intent and context, identify anything vague or missing, \
             and return ONLY the refined version. No explanations, no preamble, \
             no markdown. Just the improved prompt.\n\n\
             Original prompt:\n{}",
            input
        ),
        Mode::Why => format!(
            "You are a prompt engineering expert. Analyze the following prompt, \
             infer its intent and context, identify anything vague or missing, \
             and return the result in two parts separated by the exact string \
             \"---WHY---\":\n\n\
             1. The refined prompt (no preamble, no markdown)\n\
             2. A brief bullet-point breakdown of what was weak or missing \
             in the original\n\n\
             Original prompt:\n{}",
            input
        ),
    };

    let mut child = match Command::new("claude")
        .arg("-p")
        .arg("--verbose")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--include-partial-messages")
        .arg(&prompt)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let msg = if e.kind() == std::io::ErrorKind::NotFound {
                "Claude CLI not found — install from https://docs.anthropic.com/claude-code"
                    .to_string()
            } else {
                format!("Failed to run claude: {}", e)
            };
            let _ = tx.send(Message::Error(msg));
            return;
        }
    };

    let Some(stdout) = child.stdout.take() else {
        let _ = tx.send(Message::Error("Failed to capture Claude stdout".to_string()));
        return;
    };
    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut got_output = false;

    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };

        match json["type"].as_str() {
            Some("stream_event") => {
                let event = &json["event"];
                if event["type"].as_str() == Some("content_block_delta") {
                    if let Some(text) = event["delta"]["text"].as_str() {
                        got_output = true;
                        if tx.send(Message::Chunk(text.to_string())).is_err() {
                            return;
                        }
                    }
                }
            }
            Some("result") => {
                if json["is_error"].as_bool() == Some(true) {
                    let err = json["result"]
                        .as_str()
                        .unwrap_or("Claude returned an error")
                        .to_string();
                    let _ = tx.send(Message::Error(err));
                    return;
                }
                break;
            }
            _ => {}
        }
    }

    let _ = child.wait().await;

    if got_output {
        let _ = tx.send(Message::Done);
    } else {
        let _ = tx.send(Message::Error("Claude returned empty output".to_string()));
    }
}
