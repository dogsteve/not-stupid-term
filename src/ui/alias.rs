use std::fs;
use std::path::PathBuf;

pub struct UserAlias {
    pub name: String,
    pub target: String,
}

pub struct SuggestionItem {
    pub display: String,
    pub fill_cmd: String,
    pub is_alias: bool,
    pub detail: String,
}

/// Dynamically parse user's real ~/.zshrc, ~/.zsh_aliases, ~/.bashrc for aliases
pub fn load_user_shell_aliases() -> Vec<UserAlias> {
    let mut aliases = Vec::new();

    let mut config_paths = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home);
        config_paths.push(p.join(".zshrc"));
        config_paths.push(p.join(".zsh_aliases"));
        config_paths.push(p.join(".bashrc"));
        config_paths.push(p.join(".bash_aliases"));
        config_paths.push(p.join(".config/fish/config.fish"));
    }

    for path in config_paths {
        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("alias ") {
                    let raw = &trimmed[6..];
                    if let Some(eq_idx) = raw.find('=') {
                        let name = raw[..eq_idx].trim().to_string();
                        let mut target = raw[eq_idx + 1..].trim().to_string();
                        if (target.starts_with('\'') && target.ends_with('\''))
                            || (target.starts_with('"') && target.ends_with('"'))
                        {
                            if target.len() >= 2 {
                                target = target[1..target.len() - 1].to_string();
                            }
                        }
                        if !name.is_empty() && !target.is_empty() {
                            aliases.push(UserAlias { name, target });
                        }
                    }
                }
            }
        }
    }

    let defaults = vec![
        ("gss", "git status -s"),
        ("gco", "git checkout"),
        ("gp", "git push"),
        ("gl", "git log --oneline -n 10"),
        ("ll", "ls -lah"),
        ("la", "ls -A"),
        ("nr", "npm run"),
        ("ni", "npm install"),
        ("cr", "cargo run"),
        ("cc", "cargo check"),
        ("clr", "clear"),
    ];

    for (name, target) in defaults {
        if !aliases.iter().any(|a| a.name == name) {
            aliases.push(UserAlias {
                name: name.to_string(),
                target: target.to_string(),
            });
        }
    }

    aliases
}

/// Dynamically parse user's real ~/.zsh_history and ~/.bash_history
pub fn load_user_shell_history() -> Vec<String> {
    let mut history = Vec::new();

    let mut hist_paths = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home);
        hist_paths.push(p.join(".zsh_history"));
        hist_paths.push(p.join(".bash_history"));
    }

    for path in hist_paths {
        if let Ok(bytes) = fs::read(&path) {
            let content = String::from_utf8_lossy(&bytes);
            for line in content.lines().rev().take(150) {
                let cmd = if let Some(idx) = line.find(';') {
                    &line[idx + 1..]
                } else {
                    line
                };
                let trimmed = cmd.trim().to_string();
                if !trimmed.is_empty() && !history.contains(&trimmed) {
                    history.push(trimmed);
                }
            }
        }
    }

    history
}

pub fn get_suggestions(input: &str, session_history: &[String]) -> Vec<SuggestionItem> {
    let mut results = Vec::new();
    let q = input.trim().to_lowercase();

    if q.is_empty() {
        return results;
    }

    // 1. Real User Zsh Aliases
    let aliases = load_user_shell_aliases();
    for a in &aliases {
        if a.name.to_lowercase().starts_with(&q) || a.target.to_lowercase().starts_with(&q) {
            results.push(SuggestionItem {
                display: a.name.clone(),
                fill_cmd: a.target.clone(),
                is_alias: true,
                detail: format!("-> {} (Zsh Alias)", a.target),
            });
        }
    }

    // 2. Real User Zsh History
    let zsh_history = load_user_shell_history();
    for cmd in zsh_history.iter().chain(session_history.iter().rev()) {
        if cmd.to_lowercase().starts_with(&q)
            && !results.iter().any(|r| r.fill_cmd == *cmd)
        {
            results.push(SuggestionItem {
                display: cmd.clone(),
                fill_cmd: cmd.clone(),
                is_alias: false,
                detail: "Zsh History".to_string(),
            });
        }
    }

    results.truncate(8);
    results
}
