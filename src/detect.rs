/// Result of analyzing a prompt for vagueness.
pub struct Detection {
    pub is_vague: bool,
    pub score: i32,
}

/// Analyze a prompt for specificity. The logic is inverted: instead of detecting
/// vagueness (infinite patterns), we detect specificity (finite signals). If a
/// prompt has nothing actionable and is short, it's vague by default.
pub fn analyze(prompt: &str) -> Detection {
    let trimmed = prompt.trim();
    let lower = trimmed.to_lowercase();
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    let word_count = words.len();

    // Exempt: conversational responses, always pass
    if is_conversational(&lower, word_count) {
        return Detection { is_vague: false, score: 0 };
    }

    // Exempt: clear commands ("run tests", "commit this", etc.)
    if is_clear_command(&lower, word_count) {
        return Detection { is_vague: false, score: -1 };
    }

    // Exempt: long prompts — effort was made
    if word_count > 25 {
        return Detection { is_vague: false, score: -2 };
    }

    // Count specificity signals
    let mut specificity: i32 = 0;

    if has_file_paths(trimmed) {
        specificity += 3;
    }
    if has_code_references(trimmed) {
        specificity += 3;
    }
    if has_error_content(&lower) {
        specificity += 3;
    }
    if has_technical_nouns(&lower) {
        specificity += 2;
    }

    // Short + no specificity = vague
    let is_vague = word_count <= 15 && specificity == 0;
    let score = if is_vague {
        (16 - word_count as i32).max(1)
    } else {
        -specificity
    };

    Detection { is_vague, score }
}

/// Small allowlist of responses that should always pass through.
fn is_conversational(lower: &str, word_count: usize) -> bool {
    if word_count > 5 {
        return false;
    }
    let normalized = lower
        .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ')
        .trim();
    let responses = [
        "yes", "no", "yeah", "nah", "yep", "nope", "ok", "okay", "sure",
        "thanks", "thank you", "looks good", "lgtm", "go ahead", "do it",
        "perfect", "great", "nice", "cool", "got it", "understood",
        "correct", "exactly", "right", "agreed", "approve", "deny",
        "y", "n", "please", "sorry", "what", "help", "why", "how",
        "works", "working", "done", "nope", "yup",
    ];
    responses.iter().any(|r| normalized == *r)
}

/// Starts with a known action verb and is short enough to be unambiguous.
fn is_clear_command(lower: &str, word_count: usize) -> bool {
    if word_count > 10 {
        return false;
    }
    let command_verbs = [
        "run", "test", "commit", "push", "pull", "format", "lint", "build", "deploy", "install",
        "update", "create", "delete", "remove", "add", "show", "list", "explain", "refactor",
        "rename", "move", "copy", "merge", "rebase", "checkout", "reset", "revert", "log",
        "diff", "search", "find", "replace", "open", "close", "start", "stop", "check",
        "clean", "setup", "init", "generate", "migrate", "rollback", "undo",
    ];
    let first_word = lower.split_whitespace().next().unwrap_or("");
    command_verbs.contains(&first_word)
}

fn has_file_paths(prompt: &str) -> bool {
    let extensions = [
        ".rs", ".ts", ".js", ".py", ".go", ".java", ".tsx", ".jsx", ".vue", ".rb", ".cpp", ".c",
        ".h", ".css", ".html", ".toml", ".yaml", ".yml", ".json", ".xml", ".sql", ".sh", ".md",
        ".lock", ".cfg", ".ini", ".env",
    ];
    if extensions.iter().any(|ext| prompt.contains(ext)) {
        return true;
    }

    if prompt.contains("src/") || prompt.contains("./") || prompt.contains("../") {
        return true;
    }

    // file:line pattern (e.g. foo.rs:42)
    let chars: Vec<char> = prompt.chars().collect();
    for i in 0..chars.len().saturating_sub(1) {
        if chars[i] == ':' && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
    }

    false
}

fn has_code_references(prompt: &str) -> bool {
    if prompt.contains("()") {
        return true;
    }

    let keywords = [
        "fn ", "func ", "def ", "class ", "struct ", "impl ", "trait ", "interface ", "enum ",
        "module ", "import ", "require(", "async ", "await ",
    ];
    if keywords.iter().any(|k| prompt.contains(k)) {
        return true;
    }

    // snake_case identifiers (e.g. user_service, handle_auth)
    let has_snake = prompt.split_whitespace().any(|w| {
        w.len() > 3 && w.contains('_') && w.chars().all(|c| c.is_alphanumeric() || c == '_')
    });

    // camelCase identifiers (e.g. handleAuth, getUserById)
    let has_camel = prompt.split_whitespace().any(|w| {
        w.len() > 3
            && w.chars().next().is_some_and(|c| c.is_lowercase())
            && w.chars().any(|c| c.is_uppercase())
            && w.chars().all(|c| c.is_alphanumeric())
    });

    has_snake || has_camel
}

fn has_error_content(lower: &str) -> bool {
    let indicators = [
        "error:", "error[", "panic", "exception", "traceback", "stack trace",
        "segfault", "null pointer", "cannot find", "not found", "failed to",
        "compilation error", "syntax error", "type error", "runtime error",
        "undefined reference", "permission denied", "timed out", "timeout",
        "overflow", "underflow", "deadlock", "race condition",
    ];
    indicators.iter().any(|i| lower.contains(i))
}

/// Domain-specific nouns that indicate the user is talking about something concrete.
fn has_technical_nouns(lower: &str) -> bool {
    let nouns = [
        "endpoint", "api", "database", "query", "migration", "schema", "table", "column",
        "middleware", "controller", "handler", "router", "route", "request", "response",
        "authentication", "authorization", "token", "session", "cookie", "header",
        "component", "template", "layout", "stylesheet", "callback", "promise",
        "thread", "process", "socket", "connection", "buffer", "cache", "queue",
        "pipeline", "webhook", "cron", "docker", "container", "deployment",
        "variable", "parameter", "argument", "return value", "dependency",
        "config", "configuration", "environment", "registry", "package",
    ];
    nouns.iter().any(|n| lower.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_vague_prompts() {
        assert!(analyze("fix the bug").is_vague);
        assert!(analyze("it's still broken").is_vague);
        assert!(analyze("freaking bug still happening").is_vague);
        assert!(analyze("this doesn't work").is_vague);
        assert!(analyze("the error is back").is_vague);
        assert!(analyze("ugh not working").is_vague);
        assert!(analyze("damn thing is still failing").is_vague);
        assert!(analyze("it stopped working").is_vague);
        assert!(analyze("can you fix it").is_vague);
        assert!(analyze("make it work").is_vague);
        assert!(analyze("why is this broken").is_vague);
        assert!(analyze("it broke").is_vague);
        assert!(analyze("do that thing from before").is_vague);
    }

    #[test]
    fn allows_specific_prompts() {
        assert!(!analyze("fix the null pointer in src/auth.rs:45").is_vague);
        assert!(!analyze("the build fails with error: cannot find module foo").is_vague);
        assert!(!analyze("refactor the UserService class to use dependency injection").is_vague);
        assert!(!analyze("the handleAuth function is returning undefined instead of the token").is_vague);
        assert!(!analyze("add rate limiting to the /api/users endpoint").is_vague);
    }

    #[test]
    fn allows_commands() {
        assert!(!analyze("run the tests").is_vague);
        assert!(!analyze("commit these changes").is_vague);
        assert!(!analyze("format this file").is_vague);
        assert!(!analyze("explain how the auth middleware works").is_vague);
        assert!(!analyze("build the project").is_vague);
        assert!(!analyze("check src/main.rs").is_vague);
    }

    #[test]
    fn allows_conversational() {
        assert!(!analyze("yes").is_vague);
        assert!(!analyze("no").is_vague);
        assert!(!analyze("looks good").is_vague);
        assert!(!analyze("go ahead").is_vague);
        assert!(!analyze("thanks").is_vague);
        assert!(!analyze("works").is_vague);
        assert!(!analyze("what").is_vague);
        assert!(!analyze("help").is_vague);
    }

    #[test]
    fn allows_detailed_prompts() {
        let detailed = "The authentication middleware in src/middleware/auth.ts is returning \
                        401 for valid tokens. I think the JWT verification is using the wrong \
                        secret. Can you check the verify function and compare it with the \
                        signing logic in src/auth/jwt.ts?";
        assert!(!analyze(detailed).is_vague);
    }

    #[test]
    fn allows_technical_content() {
        assert!(!analyze("the database migration is failing").is_vague);
        assert!(!analyze("fix the authentication token refresh").is_vague);
        assert!(!analyze("the API endpoint returns wrong data").is_vague);
    }
}
