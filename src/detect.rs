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

    // Exempt: context-referencing prompts — specific in conversation
    if references_context(&lower) {
        return Detection { is_vague: false, score: 0 };
    }

    // Exempt: long prompts — effort was made
    if word_count > 25 {
        return Detection { is_vague: false, score: -2 };
    }

    // Count specificity signals (weighted)
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
    if has_urls(trimmed) {
        specificity += 3;
    }
    if has_git_references(trimmed) {
        specificity += 2;
    }
    if has_technical_nouns(&lower) {
        specificity += 2;
    }
    if has_numbers_with_context(&lower) {
        specificity += 1;
    }
    if has_structure(trimmed) {
        specificity += 2;
    }

    // Self-contained commands that don't need more detail
    if is_self_contained_command(&lower) {
        return Detection { is_vague: false, score: -1 };
    }

    // Any specificity signal = not vague, regardless of length
    if specificity > 0 {
        return Detection { is_vague: false, score: -specificity };
    }

    // No specificity + short/medium = vague
    let is_vague = word_count <= 18;
    let score = if is_vague {
        (19 - word_count as i32).max(1)
    } else {
        0
    };

    Detection { is_vague, score }
}

/// Small allowlist of responses that should always pass through.
fn is_conversational(lower: &str, word_count: usize) -> bool {
    if word_count > 6 {
        return false;
    }
    let normalized = lower
        .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ')
        .trim();
    let responses = [
        // English — affirmatives
        "yes", "no", "yeah", "nah", "yep", "nope", "ok", "okay", "sure",
        "y", "n", "yup", "absolutely", "definitely", "of course",
        // English — acknowledgements
        "thanks", "thank you", "looks good", "lgtm", "go ahead", "do it",
        "perfect", "great", "nice", "cool", "got it", "understood",
        "correct", "exactly", "right", "agreed", "approve", "deny",
        "sounds good", "works for me", "that works", "fine by me",
        "go for it", "ship it", "thats fine", "all good",
        // English — flow control
        "please", "sorry", "what", "help", "why", "how",
        "works", "working", "done", "continue", "proceed", "next",
        "skip", "stop", "wait", "hold on", "not yet", "later",
        "nevermind", "never mind", "nvm", "cancel", "abort",
        // Portuguese
        "sim", "nao", "obrigado", "obrigada", "valeu", "beleza", "blz",
        "certo", "correto", "isso", "exato", "pode", "bom", "boa",
        "tranquilo", "perfeito", "pronto", "feito", "por favor",
        "ta bom", "de boa", "falou", "firmeza",
        // Spanish
        "si", "no", "gracias", "vale", "dale", "bueno", "bien",
        "correcto", "exacto", "claro", "listo", "hecho", "por favor",
        "perfecto", "genial", "entendido", "de acuerdo", "sale",
        // French
        "oui", "non", "merci", "parfait", "bien", "bon", "exact",
        "compris", "entendu", "correct", "fait", "daccord",
        // German
        "ja", "nein", "danke", "gut", "richtig", "genau", "fertig",
        "verstanden", "perfekt", "bitte", "alles klar",
        // Italian
        "si", "no", "grazie", "perfetto", "bene", "fatto", "esatto",
        "capito", "corretto", "va bene",
        // Japanese (romanized)
        "hai", "iie", "arigatou", "ok", "ii", "daijoubu",
        // Chinese (pinyin)
        "hao", "dui", "xie xie", "keyi", "hao de", "mei wenti",
    ];
    responses.iter().any(|r| normalized == *r)
}

/// Prompts that refer back to conversation context — specific in context even
/// without standalone specificity signals.
fn references_context(lower: &str) -> bool {
    let phrases = [
        "do the same", "same thing", "same for", "like before",
        "as before", "as discussed", "as we discussed", "as mentioned",
        "what we talked about", "what you said", "what you suggested",
        "like you did", "like last time", "the other one", "the other file",
        "the rest", "the remaining", "keep going", "carry on",
        "do that again", "again but", "one more time",
        "and the other", "now do", "now fix", "now add",
        "also do", "also fix", "also add", "also update",
        "same but", "but this time", "this time",
    ];
    phrases.iter().any(|p| lower.contains(p))
}

/// Commands that are complete on their own and don't need a specific target.
/// Uses pattern matching instead of exact phrases for better coverage.
fn is_self_contained_command(lower: &str) -> bool {
    // Exact phrases
    let phrases = [
        // English
        "run the tests", "run tests", "run test", "run the test suite",
        "run all tests", "run my tests", "run the test",
        "commit this", "commit these changes", "commit the changes",
        "push this", "push the changes", "push it",
        "build the project", "build this", "build it",
        "format this", "format this file", "format the file", "format the code",
        "lint this", "lint the code", "lint it",
        "deploy this", "deploy it",
        "revert this", "undo this", "undo that",
        "clean up", "clean this up",
        "show the diff", "show diff", "show the logs", "show logs",
        "check for errors", "check for warnings",
        "save this", "save it", "save the file",
        "close this", "close it",
        "retry", "try again", "retry that",
        // Portuguese
        "roda os testes", "roda o teste", "rodar os testes",
        "comita isso", "commita isso", "faz o commit",
        "compila o projeto", "compila isso",
        "formata isso", "formata o arquivo",
        "limpa isso", "salva isso",
        // Spanish
        "ejecuta los tests", "ejecuta las pruebas",
        "compila el proyecto", "compila esto",
        "formatea esto", "guarda esto",
        // French
        "lance les tests", "compile le projet",
        "formate le fichier",
        // German
        "starte die tests", "baue das projekt",
    ];
    if phrases.iter().any(|p| lower.starts_with(p)) {
        return true;
    }

    // Pattern: "run/build/test" + optional words + "again"/"everything"/"all"
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() <= 4 {
        let first = words.first().copied().unwrap_or("");
        let last = words.last().copied().unwrap_or("");
        let action_verbs = ["run", "build", "test", "lint", "format", "deploy", "check"];
        let completers = ["again", "everything", "all", "it", "this"];
        if action_verbs.contains(&first) && completers.contains(&last) {
            return true;
        }
    }

    false
}

fn has_file_paths(prompt: &str) -> bool {
    let extensions = [
        ".rs", ".ts", ".js", ".py", ".go", ".java", ".tsx", ".jsx", ".vue", ".rb",
        ".cpp", ".c", ".h", ".hpp", ".cs", ".swift", ".kt", ".scala", ".zig", ".hs",
        ".css", ".scss", ".sass", ".less",
        ".html", ".htm", ".svelte", ".astro",
        ".toml", ".yaml", ".yml", ".json", ".xml", ".ini", ".cfg", ".conf",
        ".sql", ".sh", ".bash", ".zsh", ".fish", ".ps1", ".bat", ".cmd",
        ".md", ".txt", ".log",
        ".lock", ".env", ".gitignore", ".dockerignore",
        ".proto", ".graphql", ".gql",
    ];
    if extensions.iter().any(|ext| prompt.contains(ext)) {
        return true;
    }

    // Path separators
    if prompt.contains("src/") || prompt.contains("./") || prompt.contains("../") {
        return true;
    }

    // Windows paths
    if prompt.contains("src\\") || prompt.contains(".\\") || prompt.contains("..\\") {
        return true;
    }

    // Absolute paths
    if prompt.chars().nth(1) == Some(':') && prompt.chars().nth(2) == Some('\\') {
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
    // Function call syntax
    if prompt.contains("()") {
        return true;
    }

    // Backtick-wrapped code (markdown inline code)
    if prompt.contains('`') {
        let mut in_backtick = false;
        let mut content_len = 0;
        for ch in prompt.chars() {
            if ch == '`' {
                if in_backtick && content_len > 0 {
                    return true;
                }
                in_backtick = !in_backtick;
                content_len = 0;
            } else if in_backtick {
                content_len += 1;
            }
        }
    }

    let keywords = [
        "fn ", "func ", "def ", "class ", "struct ", "impl ", "trait ", "interface ", "enum ",
        "module ", "import ", "require(", "async ", "await ", "const ", "let ", "var ",
        "pub fn ", "pub struct ", "pub enum ",
        "SELECT ", "INSERT ", "UPDATE ", "DELETE ", "CREATE TABLE", "ALTER TABLE",
    ];
    if keywords.iter().any(|k| prompt.contains(k)) {
        return true;
    }

    // snake_case identifiers (e.g. user_service, handle_auth)
    let has_snake = prompt.split_whitespace().any(|w| {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        clean.len() > 3 && clean.contains('_') && clean.chars().all(|c| c.is_alphanumeric() || c == '_')
    });

    // camelCase identifiers (e.g. handleAuth, getUserById)
    let has_camel = prompt.split_whitespace().any(|w| {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
        clean.len() > 3
            && clean.chars().next().is_some_and(|c| c.is_lowercase())
            && clean.chars().any(|c| c.is_uppercase())
            && clean.chars().all(|c| c.is_alphanumeric())
    });

    // PascalCase type names (e.g. UserService, AuthHandler)
    let has_pascal = prompt.split_whitespace().any(|w| {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
        clean.len() > 3
            && clean.chars().next().is_some_and(|c| c.is_uppercase())
            && clean.chars().skip(1).any(|c| c.is_uppercase())
            && clean.chars().all(|c| c.is_alphanumeric())
    });

    has_snake || has_camel || has_pascal
}

fn has_error_content(lower: &str) -> bool {
    let indicators = [
        // Structured error prefixes
        "error:", "error[", "warning:", "warn:",
        // Error types
        "panic", "exception", "traceback", "stack trace",
        "segfault", "null pointer", "nullptr", "nil pointer",
        "cannot find", "not found", "failed to", "unable to",
        "compilation error", "syntax error", "type error", "runtime error",
        "undefined reference", "permission denied", "timed out", "timeout",
        "overflow", "underflow", "deadlock", "race condition",
        "unhandled", "uncaught", "unexpected", "invalid",
        // Status codes
        "status 4", "status 5", "http 4", "http 5",
        "404", "500", "401", "403", "502", "503",
        // Exit codes
        "exit code", "exit status", "returned 1", "returned -1",
        "exited with",
    ];
    indicators.iter().any(|i| lower.contains(i))
}

/// URLs indicate the user is pointing at something specific.
fn has_urls(prompt: &str) -> bool {
    prompt.contains("http://")
        || prompt.contains("https://")
        || prompt.contains("localhost:")
        || prompt.contains("127.0.0.1")
}

/// Git references: commit hashes, HEAD notation, branch-like refs.
fn has_git_references(prompt: &str) -> bool {
    // HEAD notation
    if prompt.contains("HEAD") || prompt.contains("head~") || prompt.contains("HEAD~") {
        return true;
    }

    // Short or full commit hashes (7-40 hex chars as standalone word)
    prompt.split_whitespace().any(|w| {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
        clean.len() >= 7
            && clean.len() <= 40
            && clean.chars().all(|c| c.is_ascii_hexdigit())
            && clean.chars().any(|c| c.is_ascii_digit())
            && clean.chars().any(|c| c.is_ascii_alphabetic())
    })
}

/// Numbers in context suggest specific references (line numbers, ports, counts).
fn has_numbers_with_context(lower: &str) -> bool {
    let patterns = [
        "line ", "port ", "version ", "v0.", "v1.", "v2.", "v3.",
        "step ", "item ", "column ", "row ", "index ", "offset ",
        "issue #", "pr #", "ticket ", "#",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

/// Structured prompts (numbered lists, bullets) indicate deliberate thought.
fn has_structure(prompt: &str) -> bool {
    let lines: Vec<&str> = prompt.lines().collect();
    if lines.len() < 2 {
        return false;
    }
    let structured_lines = lines.iter().filter(|l| {
        let trimmed = l.trim();
        trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("1.")
            || trimmed.starts_with("2.")
            || trimmed.starts_with("3.")
    }).count();
    structured_lines >= 2
}

/// Domain-specific nouns that indicate the user is talking about something concrete.
fn has_technical_nouns(lower: &str) -> bool {
    let nouns = [
        // Architecture
        "endpoint", "api", "database", "query", "migration", "schema", "table", "column",
        "middleware", "controller", "handler", "router", "route", "request", "response",
        "microservice", "monolith", "gateway", "proxy", "load balancer",
        // Auth
        "authentication", "authorization", "token", "session", "cookie", "header",
        "oauth", "jwt", "saml", "credential",
        // Frontend
        "component", "template", "layout", "stylesheet", "callback", "promise",
        "hook", "state", "reducer", "context", "provider", "consumer",
        // System
        "thread", "process", "socket", "connection", "buffer", "cache", "queue",
        "pipeline", "webhook", "cron", "docker", "container", "deployment",
        "kubernetes", "k8s", "nginx", "redis", "postgres", "mongo",
        // Code structure
        "variable", "parameter", "argument", "return value", "dependency",
        "config", "configuration", "environment", "registry", "package",
        "function", "method", "constructor", "destructor", "iterator",
        "index", "foreign key", "primary key", "constraint",
        // Testing
        "test suite", "test case", "fixture", "mock", "stub", "assertion",
        // Git/CI
        "branch", "pull request", "merge conflict", "rebase", "ci pipeline",
        "workflow", "github action",
    ];
    nouns.iter().any(|n| lower.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === VAGUE PROMPTS — should block ===

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
    fn flags_vague_commands() {
        assert!(analyze("add ui").is_vague);
        assert!(analyze("fix it").is_vague);
        assert!(analyze("create the thing").is_vague);
        assert!(analyze("update the stuff").is_vague);
        assert!(analyze("adiciona ui").is_vague);
        assert!(analyze("change it").is_vague);
        assert!(analyze("make it better").is_vague);
        assert!(analyze("improve this").is_vague);
    }

    #[test]
    fn flags_vague_medium_length() {
        // 16-20 words with no specificity should still be caught
        assert!(analyze("I need you to help me fix that thing we were working on yesterday please").is_vague);
        assert!(analyze("can you take a look at what we did and make it work better somehow").is_vague);
    }

    // === SPECIFIC PROMPTS — should allow ===

    #[test]
    fn allows_specific_prompts() {
        assert!(!analyze("fix the null pointer in src/auth.rs:45").is_vague);
        assert!(!analyze("the build fails with error: cannot find module foo").is_vague);
        assert!(!analyze("refactor the UserService class to use dependency injection").is_vague);
        assert!(!analyze("the handleAuth function is returning undefined instead of the token").is_vague);
        assert!(!analyze("add rate limiting to the /api/users endpoint").is_vague);
    }

    #[test]
    fn allows_backtick_code() {
        assert!(!analyze("fix `handleAuth`").is_vague);
        assert!(!analyze("the `user_service` is broken").is_vague);
        assert!(!analyze("update `config.toml`").is_vague);
        assert!(!analyze("rename `foo` to `bar`").is_vague);
    }

    #[test]
    fn allows_urls() {
        assert!(!analyze("look at https://github.com/issue/123").is_vague);
        assert!(!analyze("the page at localhost:3000 is broken").is_vague);
    }

    #[test]
    fn allows_git_references() {
        assert!(!analyze("revert abc1234").is_vague);
        assert!(!analyze("cherry-pick abc1234def").is_vague);
        assert!(!analyze("what changed in HEAD~2").is_vague);
    }

    #[test]
    fn allows_error_codes() {
        assert!(!analyze("getting a 404 on the page").is_vague);
        assert!(!analyze("returns status 500").is_vague);
        assert!(!analyze("exit code 1 when running").is_vague);
    }

    #[test]
    fn allows_pascal_case() {
        assert!(!analyze("fix the AuthHandler").is_vague);
        assert!(!analyze("update UserService").is_vague);
    }

    // === COMMANDS — should allow ===

    #[test]
    fn allows_commands() {
        assert!(!analyze("run the tests").is_vague);
        assert!(!analyze("commit these changes").is_vague);
        assert!(!analyze("format this file").is_vague);
        assert!(!analyze("explain how the auth middleware works").is_vague);
        assert!(!analyze("build the project").is_vague);
        assert!(!analyze("check src/main.rs").is_vague);
        assert!(!analyze("add rate limiting to the /api/users endpoint").is_vague);
        assert!(!analyze("delete the old migration files in src/db").is_vague);
    }

    #[test]
    fn allows_self_contained_variants() {
        assert!(!analyze("run all tests").is_vague);
        assert!(!analyze("run my tests").is_vague);
        assert!(!analyze("build it again").is_vague);
        assert!(!analyze("test everything").is_vague);
        assert!(!analyze("lint this").is_vague);
        assert!(!analyze("retry").is_vague);
        assert!(!analyze("try again").is_vague);
    }

    // === CONVERSATIONAL — should allow ===

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
    fn allows_expanded_conversational() {
        assert!(!analyze("sounds good").is_vague);
        assert!(!analyze("go for it").is_vague);
        assert!(!analyze("absolutely").is_vague);
        assert!(!analyze("not yet").is_vague);
        assert!(!analyze("continue").is_vague);
        assert!(!analyze("skip").is_vague);
        assert!(!analyze("nevermind").is_vague);
        assert!(!analyze("ship it").is_vague);
        assert!(!analyze("all good").is_vague);
    }

    // === CONTEXT REFERENCES — should allow ===

    #[test]
    fn allows_context_references() {
        assert!(!analyze("do the same for the other file").is_vague);
        assert!(!analyze("like before but with error handling").is_vague);
        assert!(!analyze("same thing for the tests").is_vague);
        assert!(!analyze("now do the login page").is_vague);
        assert!(!analyze("also fix the header").is_vague);
        assert!(!analyze("keep going").is_vague);
    }

    // === STRUCTURED — should allow ===

    #[test]
    fn allows_structured_prompts() {
        assert!(!analyze("1. fix the bug\n2. add tests\n3. update docs").is_vague);
        assert!(!analyze("- fix auth\n- add logging\n- update docs").is_vague);
    }

    // === DETAILED — should allow ===

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
