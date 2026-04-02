use crate::claude::{self, Message, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Modifier, Style};
use tokio::sync::mpsc;
use tui_textarea::TextArea;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Idle,
    Refining,
    Error,
}

pub struct App {
    pub input: TextArea<'static>,
    pub output: String,
    pub why_breakdown: Option<String>,
    pub focus: Focus,
    pub status: Status,
    pub status_msg: String,
    pub last_mode: Option<Mode>,
    pub scroll_offset: u16,
}

impl App {
    pub fn new() -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());
        input.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        Self {
            input,
            output: String::new(),
            why_breakdown: None,
            focus: Focus::Input,
            status: Status::Idle,
            status_msg: String::from("Ready"),
            last_mode: None,
            scroll_offset: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<Message>) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') => return true,
                KeyCode::Char('r') => {
                    self.run_claude(Mode::Refine, tx);
                    return false;
                }
                KeyCode::Char('w') => {
                    self.run_claude(Mode::Why, tx);
                    return false;
                }
                KeyCode::Char('y') => {
                    self.copy_output();
                    return false;
                }
                KeyCode::Char('l') => {
                    self.clear();
                    return false;
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Tab {
            self.toggle_focus();
            return false;
        }

        match self.focus {
            Focus::Input => {
                self.input.input(key);
            }
            Focus::Output => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                }
                _ => {}
            },
        }

        false
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Input => {
                self.input.set_cursor_style(Style::default());
                Focus::Output
            }
            Focus::Output => {
                self.input
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                Focus::Input
            }
        };
    }

    fn run_claude(&mut self, mode: Mode, tx: &mpsc::UnboundedSender<Message>) {
        if self.status == Status::Refining {
            return;
        }

        let input = self.input.lines().join("\n");
        if input.trim().is_empty() {
            self.status = Status::Error;
            self.status_msg = "Empty input".to_string();
            return;
        }

        self.output.clear();
        self.why_breakdown = None;
        self.status = Status::Refining;
        self.status_msg = match mode {
            Mode::Refine => "Refining...".to_string(),
            Mode::Why => "Analyzing...".to_string(),
        };
        self.last_mode = Some(mode);
        self.scroll_offset = 0;

        let tx = tx.clone();
        tokio::spawn(async move {
            claude::run_claude(&input, mode, tx).await;
        });
    }

    pub fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Chunk(text) => {
                self.output.push_str(&text);
            }
            Message::Done => {
                let raw = self.output.trim().to_string();
                if self.last_mode == Some(Mode::Why) {
                    if let Some(idx) = raw.find("---WHY---") {
                        self.output = raw[..idx].trim().to_string();
                        self.why_breakdown = Some(raw[idx + 9..].trim().to_string());
                    } else {
                        self.output = raw;
                    }
                } else {
                    self.output = raw;
                }
                self.status = Status::Idle;
                self.status_msg = "Done".to_string();
            }
            Message::Error(e) => {
                self.status = Status::Error;
                self.status_msg = e;
                self.output.clear();
                self.why_breakdown = None;
            }
        }
    }

    fn copy_output(&mut self) {
        if self.output.is_empty() {
            self.status_msg = "Nothing to copy".to_string();
            return;
        }

        match arboard::Clipboard::new().and_then(|mut c| c.set_text(&self.output)) {
            Ok(_) => {
                self.status_msg = "Copied to clipboard".to_string();
            }
            Err(_) => self.copy_wl_copy(),
        }
    }

    fn copy_wl_copy(&mut self) {
        use std::io::Write;
        use std::process::{Command, Stdio};

        match Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            Ok(mut child) => {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(self.output.as_bytes());
                }
                let _ = child.wait();
                self.status_msg = "Copied (wl-copy)".to_string();
            }
            Err(_) => {
                self.status_msg = "Clipboard unavailable".to_string();
            }
        }
    }

    fn clear(&mut self) {
        self.input = TextArea::default();
        self.input.set_cursor_line_style(Style::default());
        self.input
            .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        self.output.clear();
        self.why_breakdown = None;
        self.status = Status::Idle;
        self.status_msg = "Cleared".to_string();
        self.scroll_offset = 0;
        self.focus = Focus::Input;
    }
}
