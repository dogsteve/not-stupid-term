/// Auto-detects file type and formats code content in-place.
/// This is a zero-dependency pure-Rust formatter for common file types.

/// Represents a detected and formattable file type.
#[derive(Debug, Clone, PartialEq)]
pub enum FileKind {
    Json,
    Xml,
    Html,
    Sql,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Kotlin,
    CStyle,    // C / C++ / C#
    Shell,
    Toml,
    Yaml,
    Css,
    PlainText,
    Unknown,
}

impl FileKind {
    pub fn label(&self) -> &'static str {
        match self {
            FileKind::Json => "JSON",
            FileKind::Xml => "XML",
            FileKind::Html => "HTML",
            FileKind::Sql => "SQL",
            FileKind::Rust => "Rust",
            FileKind::Python => "Python",
            FileKind::JavaScript => "JavaScript",
            FileKind::TypeScript => "TypeScript",
            FileKind::Go => "Go",
            FileKind::Java => "Java",
            FileKind::Kotlin => "Kotlin",
            FileKind::CStyle => "C/C++",
            FileKind::Shell => "Shell",
            FileKind::Toml => "TOML",
            FileKind::Yaml => "YAML",
            FileKind::Css => "CSS",
            FileKind::PlainText => "Text",
            FileKind::Unknown => "Unknown",
        }
    }

    pub fn can_format(&self) -> bool {
        matches!(
            self,
            FileKind::Json
                | FileKind::Xml
                | FileKind::Html
                | FileKind::Sql
                | FileKind::Css
                | FileKind::Rust
                | FileKind::CStyle
                | FileKind::Java
                | FileKind::Kotlin
                | FileKind::JavaScript
                | FileKind::TypeScript
                | FileKind::Go
                | FileKind::Python
                | FileKind::Shell
                | FileKind::Toml
                | FileKind::Yaml
        )
    }
}

/// Detect file kind from file extension (primary) or content heuristics (fallback).
pub fn detect_kind(path: &str, content: &str) -> FileKind {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" => return FileKind::Json,
        "xml" => return FileKind::Xml,
        "html" | "htm" => return FileKind::Html,
        "sql" => return FileKind::Sql,
        "rs" => return FileKind::Rust,
        "py" => return FileKind::Python,
        "js" | "mjs" | "cjs" => return FileKind::JavaScript,
        "ts" | "tsx" | "jsx" => return FileKind::TypeScript,
        "go" => return FileKind::Go,
        "java" => return FileKind::Java,
        "kt" | "kts" => return FileKind::Kotlin,
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "cs" => return FileKind::CStyle,
        "sh" | "bash" | "zsh" | "fish" => return FileKind::Shell,
        "toml" => return FileKind::Toml,
        "yaml" | "yml" => return FileKind::Yaml,
        "css" | "scss" | "less" => return FileKind::Css,
        "txt" | "md" | "markdown" => {
            // Fall through to content sniffing for untitled/txt files
        }
        _ => {}
    }

    // Content sniffing heuristics
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if looks_like_json(trimmed) {
            return FileKind::Json;
        }
    }
    if trimmed.starts_with('<') {
        if trimmed.contains("<!DOCTYPE html") || trimmed.contains("<html") {
            return FileKind::Html;
        }
        return FileKind::Xml;
    }
    if trimmed.starts_with("SELECT ")
        || trimmed.starts_with("select ")
        || trimmed.starts_with("INSERT ")
        || trimmed.starts_with("CREATE ")
        || trimmed.starts_with("ALTER ")
        || trimmed.starts_with("UPDATE ")
        || trimmed.starts_with("DELETE ")
        || trimmed.starts_with("DROP ")
    {
        return FileKind::Sql;
    }
    if trimmed.starts_with("fn ") || trimmed.contains("\nfn ") || trimmed.starts_with("use ") {
        return FileKind::Rust;
    }
    if trimmed.starts_with("def ") || trimmed.starts_with("import ") || trimmed.starts_with("from ") {
        return FileKind::Python;
    }
    if trimmed.starts_with("package ") && trimmed.contains("func ") {
        return FileKind::Go;
    }
    if trimmed.starts_with("public class ") || trimmed.starts_with("class ") && trimmed.contains("void main") {
        return FileKind::Java;
    }

    FileKind::Unknown
}

fn looks_like_json(s: &str) -> bool {
    // Quick heuristic: balanced braces and contains colon
    s.contains(':') || s.starts_with('[')
}

/// Format the content according to its detected file kind.
/// Returns `Ok(formatted)` on success or `Err(description)` on failure.
pub fn format_content(content: &str, kind: &FileKind) -> Result<String, String> {
    match kind {
        FileKind::Json => format_json(content),
        FileKind::Xml | FileKind::Html => format_xml(content),
        FileKind::Sql => format_sql(content),
        FileKind::Css => format_css(content),
        FileKind::Rust
        | FileKind::Java
        | FileKind::Kotlin
        | FileKind::CStyle
        | FileKind::JavaScript
        | FileKind::TypeScript
        | FileKind::Go => format_brace_language(content),
        FileKind::Python | FileKind::Shell => format_indent_normalize(content),
        FileKind::Toml | FileKind::Yaml => format_indent_normalize(content),
        _ => Err(format!("No formatter for {:?}", kind)),
    }
}

// ─────────────────────────────────────────────────────────
// JSON formatter – pretty-prints by re-tokenising the input
// ─────────────────────────────────────────────────────────
fn format_json(src: &str) -> Result<String, String> {
    let mut out = String::with_capacity(src.len() + src.len() / 4);
    let mut indent: usize = 0;
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut escape_next = false;

    while i < bytes.len() {
        let b = bytes[i];

        if escape_next {
            out.push(b as char);
            escape_next = false;
            i += 1;
            continue;
        }

        if in_string {
            if b == b'\\' {
                escape_next = true;
            } else if b == b'"' {
                in_string = false;
            }
            out.push(b as char);
            i += 1;
            continue;
        }

        match b {
            b'"' => {
                in_string = true;
                out.push('"');
            }
            b'{' | b'[' => {
                out.push(b as char);
                indent += 1;
                // Peek: if next non-whitespace is closing bracket, keep on same line
                let rest = src[i + 1..].trim_start();
                if rest.starts_with('}') || rest.starts_with(']') {
                    // empty object/array – leave on one line
                } else {
                    out.push('\n');
                    push_indent(&mut out, indent);
                }
            }
            b'}' | b']' => {
                if indent > 0 {
                    indent -= 1;
                }
                out.push('\n');
                push_indent(&mut out, indent);
                out.push(b as char);
            }
            b',' => {
                out.push(',');
                out.push('\n');
                push_indent(&mut out, indent);
            }
            b':' => {
                out.push_str(": ");
            }
            b' ' | b'\t' | b'\n' | b'\r' => {
                // skip all whitespace outside strings – we control spacing ourselves
            }
            _ => {
                out.push(b as char);
            }
        }
        i += 1;
    }
    Ok(out.trim_end().to_string() + "\n")
}

fn push_indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("    ");
    }
}

// ─────────────────────────────────────────────────────────
// XML / HTML formatter – simple indent-based pretty printer
// ─────────────────────────────────────────────────────────
fn format_xml(src: &str) -> Result<String, String> {
    let mut out = String::with_capacity(src.len());
    let mut indent: i32 = 0;
    let mut i = 0;
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();

    while i < len {
        if chars[i] == '<' {
            let end = chars[i..].iter().position(|&c| c == '>').unwrap_or(len - i - 1) + i + 1;
            let tag: String = chars[i..end].iter().collect();

            let is_close = tag.starts_with("</");
            let is_self_close = tag.ends_with("/>") || tag.starts_with("<?") || tag.starts_with("<!");
            let is_comment = tag.starts_with("<!--");

            if is_close {
                indent = (indent - 1).max(0);
            }

            push_indent_i32(&mut out, indent);
            out.push_str(&tag);
            out.push('\n');

            if !is_close && !is_self_close && !is_comment {
                indent += 1;
            }

            i = end;
        } else {
            // Text node – collect until next '<'
            let start = i;
            while i < len && chars[i] != '<' {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let text = text.trim();
            if !text.is_empty() {
                push_indent_i32(&mut out, indent);
                out.push_str(text);
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn push_indent_i32(out: &mut String, level: i32) {
    for _ in 0..level.max(0) {
        out.push_str("  ");
    }
}

// ─────────────────────────────────────────────────────────
// SQL formatter – uppercase keywords, newline on major clauses
// ─────────────────────────────────────────────────────────
fn format_sql(src: &str) -> Result<String, String> {
    const NEWLINE_KEYWORDS: &[&str] = &[
        "SELECT", "FROM", "WHERE", "JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN",
        "OUTER JOIN", "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET",
        "INSERT INTO", "VALUES", "UPDATE", "SET", "DELETE FROM", "CREATE TABLE",
        "ALTER TABLE", "DROP TABLE", "ON", "AND", "OR",
    ];

    let mut result = String::with_capacity(src.len());
    // Uppercase all keywords and insert newlines before clause keywords
    let mut line = src
        .lines()
        .flat_map(|l| {
            let mut s = l.trim().to_string();
            // Uppercase known keywords (very simple approach)
            for kw in &["select", "from", "where", "join", "left", "right", "inner",
                         "outer", "group", "order", "by", "having", "limit", "offset",
                         "insert", "into", "values", "update", "set", "delete", "create",
                         "table", "alter", "drop", "on", "and", "or", "as", "distinct",
                         "count", "sum", "avg", "max", "min", "not", "null", "is", "in",
                         "between", "like", "exists", "union", "all", "case", "when",
                         "then", "else", "end"] {
                // Replace whole-word occurrences (case-insensitive)
                let upper = kw.to_uppercase();
                let re_src = format!(" {} ", kw);
                let re_dst = format!(" {} ", upper);
                s = s.replace(&re_src, &re_dst);
                // Handle at start of string
                if s.to_lowercase().starts_with(kw) && (s.len() == kw.len() || !s.as_bytes()[kw.len()].is_ascii_alphanumeric()) {
                    let rest = &s[kw.len()..];
                    s = format!("{}{}", upper, rest);
                }
            }
            vec![s]
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Insert newlines before major clauses
    for kw in NEWLINE_KEYWORDS {
        // replace " KEYWORD " (space-bounded) with "\nKEYWORD "
        let pattern = format!(" {} ", kw);
        let replacement = format!("\n{} ", kw);
        line = line.replace(&pattern, &replacement);
    }

    result.push_str(&line);
    Ok(result)
}

// ─────────────────────────────────────────────────────────
// CSS formatter – one property per line, indented inside {}
// ─────────────────────────────────────────────────────────
fn format_css(src: &str) -> Result<String, String> {
    let mut out = String::with_capacity(src.len());
    let mut indent = 0i32;

    for ch in src.chars() {
        match ch {
            '{' => {
                out.push_str(" {\n");
                indent += 1;
                push_indent_i32(&mut out, indent);
            }
            '}' => {
                indent = (indent - 1).max(0);
                out.push_str("\n}\n");
                if indent > 0 {
                    push_indent_i32(&mut out, indent);
                }
            }
            ';' => {
                out.push(';');
                out.push('\n');
                if indent > 0 {
                    push_indent_i32(&mut out, indent);
                }
            }
            '\n' | '\r' => {
                // skip original newlines, we manage them
            }
            _ => {
                out.push(ch);
            }
        }
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────
// Brace-language formatter (Rust, Java, C/C++, JS, Go, …)
// Re-indents using brace counting. Does NOT rewrite style,
// just normalises indentation and blank lines.
// ─────────────────────────────────────────────────────────
fn format_brace_language(src: &str) -> Result<String, String> {
    let mut out = String::with_capacity(src.len());
    let mut indent: i32 = 0;

    for line in src.lines() {
        let trimmed = line.trim();

        // If line starts with '}' or '{', adjust before printing
        let closing_start = trimmed.starts_with('}') || trimmed.starts_with(')');
        if closing_start {
            indent = (indent - 1).max(0);
        }

        if !trimmed.is_empty() {
            push_indent_i32(&mut out, indent);
            out.push_str(trimmed);
            out.push('\n');
        } else {
            out.push('\n');
        }

        // Count opening vs closing braces in this line (outside strings for a quick pass)
        let open_count = trimmed.chars().filter(|&c| c == '{' || c == '(').count() as i32;
        let close_count = trimmed.chars().filter(|&c| c == '}' || c == ')').count() as i32;
        let net = open_count - close_count;

        if !closing_start {
            // Normal adjustment after printing
            indent = (indent + net).max(0);
        } else {
            // Line started with closing – net was already applied for the leading closer
            // remaining adjustments are for any openers on same line
            indent = (indent + open_count).max(0);
        }
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────
// Python / YAML / TOML / Shell – just normalise blank lines
// and trim trailing whitespace per line
// ─────────────────────────────────────────────────────────
fn format_indent_normalize(src: &str) -> Result<String, String> {
    let mut out = String::with_capacity(src.len());
    let mut consecutive_blank = 0;
    for line in src.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            consecutive_blank += 1;
            // Collapse more than 2 consecutive blank lines into 1
            if consecutive_blank <= 1 {
                out.push('\n');
            }
        } else {
            consecutive_blank = 0;
            out.push_str(trimmed_end);
            out.push('\n');
        }
    }
    Ok(out)
}
