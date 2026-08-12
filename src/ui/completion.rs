/// Smart terminal suggestion engine.
///
/// Ranking sources (highest → lowest priority):
/// 1. **Frecency** – commands used frequently AND recently score highest.
/// 2. **Fuzzy prefix match** – partial prefix anywhere in the command.
/// 3. **Context-aware sub-command suggestions** – knows `git`, `docker`, `cargo`, `npm`, `kubectl`, `ssh`, etc.
/// 4. **File/directory path completion** – detects trailing partial path token and lists real FS entries.
/// 5. **Shell aliases** – parsed live from ~/.zshrc, ~/.bashrc, ~/.zsh_aliases, ~/.bash_aliases.
/// 6. **Shell history** – ~/.zsh_history and ~/.bash_history.
/// 7. **Session history** – commands typed in the current session.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────
// Public data types
// ─────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct UserAlias {
    pub name: String,
    pub target: String,
}

/// A single suggestion item shown in the popup.
#[derive(Clone, Debug)]
pub struct SuggestionItem {
    /// Text shown in the popup row.
    pub display: String,
    /// Command that fills the input when this item is accepted.
    pub fill_cmd: String,
    /// Whether this came from a shell alias.
    pub is_alias: bool,
    /// Short tag shown on the right side: "alias", "history", "git", "path", …
    pub detail: String,
    /// Source category for icon selection.
    pub source: SuggestionSource,
    /// Computed relevance score (higher = shown first).
    pub score: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SuggestionSource {
    Alias,
    Frecency,
    History,
    SubCommand,
    Path,
    Snippet,
}

// ─────────────────────────────────────────────────────────
// Frecency tracker  (stored in-memory per session)
// ─────────────────────────────────────────────────────────

/// A per-session frecency table that scores commands by frequency × recency.
#[derive(Default)]
pub struct FrecencyStore {
    /// command → (count, last_used_unix_secs)
    entries: HashMap<String, (u32, u64)>,
}

impl FrecencyStore {
    pub fn record(&mut self, cmd: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let entry = self.entries.entry(cmd.to_string()).or_insert((0, now));
        entry.0 += 1;
        entry.1 = now;
    }

    /// Score = count × decay where decay = 1.0 for commands used in the last hour,
    /// 0.5 for last day, 0.25 for last week, 0.1 otherwise.
    fn score(&self, cmd: &str) -> f64 {
        if let Some(&(count, last)) = self.entries.get(cmd) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let age_secs = now.saturating_sub(last);
            let decay = if age_secs < 3600 {
                1.0
            } else if age_secs < 86400 {
                0.5
            } else if age_secs < 604800 {
                0.25
            } else {
                0.1
            };
            count as f64 * decay
        } else {
            0.0
        }
    }
}

// ─────────────────────────────────────────────────────────
// Context-aware sub-command knowledge base
// ─────────────────────────────────────────────────────────

/// Returns known sub-commands + description for a given base command.
fn known_subcommands(base: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match base {
        "git" => Some(&[
            ("git add .", "Stage all changes"),
            ("git add -p", "Stage changes interactively"),
            ("git commit -m \"\"", "Commit with message"),
            ("git commit --amend", "Amend last commit"),
            ("git push", "Push to remote"),
            ("git push --force-with-lease", "Safe force push"),
            ("git pull", "Pull from remote"),
            ("git pull --rebase", "Pull with rebase"),
            ("git fetch --all", "Fetch all remotes"),
            ("git status", "Show working tree status"),
            ("git status -s", "Short status"),
            ("git log --oneline -n 20", "Compact log"),
            ("git log --graph --oneline --all", "Graph log"),
            ("git diff", "Show unstaged diff"),
            ("git diff --staged", "Show staged diff"),
            ("git stash", "Stash current changes"),
            ("git stash pop", "Pop stash"),
            ("git stash list", "List stashes"),
            ("git checkout -b ", "Create & switch branch"),
            ("git switch -c ", "Create & switch branch (new)"),
            ("git merge ", "Merge a branch"),
            ("git rebase ", "Rebase onto branch"),
            ("git reset --soft HEAD~1", "Undo last commit (keep changes)"),
            ("git reset --hard HEAD~1", "Undo last commit (discard)"),
            ("git clean -fd", "Remove untracked files"),
            ("git tag ", "Create a tag"),
            ("git remote -v", "List remotes"),
            ("git clone ", "Clone repository"),
            ("git init", "Initialise repo"),
            ("git bisect start", "Start bisect"),
            ("git cherry-pick ", "Cherry-pick commit"),
        ]),
        "docker" => Some(&[
            ("docker ps", "List running containers"),
            ("docker ps -a", "List all containers"),
            ("docker images", "List images"),
            ("docker pull ", "Pull image"),
            ("docker run -it ", "Run interactive container"),
            ("docker run -d ", "Run detached container"),
            ("docker exec -it ", "Exec into container"),
            ("docker stop ", "Stop container"),
            ("docker rm ", "Remove container"),
            ("docker rmi ", "Remove image"),
            ("docker build -t ", "Build image"),
            ("docker logs -f ", "Follow container logs"),
            ("docker compose up -d", "Start compose services"),
            ("docker compose down", "Stop compose services"),
            ("docker compose logs -f", "Follow compose logs"),
            ("docker system prune", "Remove unused data"),
            ("docker volume ls", "List volumes"),
            ("docker network ls", "List networks"),
            ("docker inspect ", "Inspect object"),
        ]),
        "cargo" => Some(&[
            ("cargo run", "Run the project"),
            ("cargo run --release", "Run in release mode"),
            ("cargo build", "Build debug"),
            ("cargo build --release", "Build release"),
            ("cargo check", "Fast type-check"),
            ("cargo test", "Run tests"),
            ("cargo test -- --nocapture", "Run tests with output"),
            ("cargo clippy", "Run linter"),
            ("cargo clippy -- -D warnings", "Clippy strict mode"),
            ("cargo fmt", "Format code"),
            ("cargo fmt -- --check", "Check formatting"),
            ("cargo add ", "Add dependency"),
            ("cargo remove ", "Remove dependency"),
            ("cargo update", "Update dependencies"),
            ("cargo doc --open", "Build & open docs"),
            ("cargo publish", "Publish to crates.io"),
            ("cargo bench", "Run benchmarks"),
            ("cargo clean", "Remove build artifacts"),
        ]),
        "npm" | "npx" => Some(&[
            ("npm run ", "Run a script"),
            ("npm run dev", "Start dev server"),
            ("npm run build", "Build production"),
            ("npm run test", "Run tests"),
            ("npm install", "Install dependencies"),
            ("npm install --save-dev ", "Install dev dependency"),
            ("npm uninstall ", "Uninstall package"),
            ("npm update", "Update packages"),
            ("npm audit", "Security audit"),
            ("npm audit fix", "Auto-fix vulnerabilities"),
            ("npm outdated", "List outdated packages"),
            ("npm list --depth=0", "List top-level packages"),
            ("npm init -y", "Init package.json"),
            ("npx create-react-app ", "Create React app"),
            ("npx create-next-app ", "Create Next.js app"),
        ]),
        "kubectl" | "k" => Some(&[
            ("kubectl get pods", "List pods"),
            ("kubectl get pods -A", "List all namespaced pods"),
            ("kubectl get services", "List services"),
            ("kubectl get deployments", "List deployments"),
            ("kubectl get nodes", "List nodes"),
            ("kubectl describe pod ", "Describe pod"),
            ("kubectl logs -f ", "Follow pod logs"),
            ("kubectl exec -it ", "Exec into pod"),
            ("kubectl apply -f ", "Apply manifest"),
            ("kubectl delete -f ", "Delete from manifest"),
            ("kubectl rollout restart deployment/", "Restart deployment"),
            ("kubectl scale deployment/ --replicas=", "Scale deployment"),
            ("kubectl port-forward ", "Port forward"),
            ("kubectl config get-contexts", "List contexts"),
            ("kubectl config use-context ", "Switch context"),
            ("kubectl top pods", "Pod resource usage"),
            ("kubectl top nodes", "Node resource usage"),
        ]),
        "ssh" => Some(&[
            ("ssh -i ", "SSH with identity file"),
            ("ssh -L ", "Local port forward"),
            ("ssh -R ", "Remote port forward"),
            ("ssh -N -f ", "Background tunnel"),
            ("ssh-keygen -t ed25519 -C \"\"", "Generate ED25519 key"),
            ("ssh-copy-id ", "Copy key to server"),
        ]),
        "python" | "python3" => Some(&[
            ("python3 -m venv venv", "Create virtual env"),
            ("python3 -m pip install ", "Install package"),
            ("python3 -m pip install -r requirements.txt", "Install requirements"),
            ("python3 -m pip freeze > requirements.txt", "Export requirements"),
            ("python3 -c \"\"", "Run inline code"),
            ("python3 -m http.server 8080", "Start HTTP server"),
        ]),
        "pip" | "pip3" => Some(&[
            ("pip install ", "Install package"),
            ("pip install -r requirements.txt", "Install requirements"),
            ("pip freeze > requirements.txt", "Export requirements"),
            ("pip uninstall ", "Uninstall package"),
            ("pip list", "List installed packages"),
            ("pip show ", "Show package info"),
            ("pip upgrade ", "Upgrade package"),
        ]),
        "systemctl" => Some(&[
            ("systemctl status ", "Check service status"),
            ("systemctl start ", "Start service"),
            ("systemctl stop ", "Stop service"),
            ("systemctl restart ", "Restart service"),
            ("systemctl enable ", "Enable service at boot"),
            ("systemctl disable ", "Disable service at boot"),
            ("systemctl list-units --type=service", "List all services"),
            ("systemctl daemon-reload", "Reload daemon config"),
            ("journalctl -u ", "View service logs"),
            ("journalctl -f", "Follow journal"),
        ]),
        "ls" => Some(&[
            ("ls -lah", "Long list with hidden, human sizes"),
            ("ls -la", "Long list with hidden"),
            ("ls -lt", "Sort by modification time"),
            ("ls -lS", "Sort by size"),
            ("ls -R", "Recursive list"),
        ]),
        "find" => Some(&[
            ("find . -name \"\"", "Find by name"),
            ("find . -type f -name \"*.\"", "Find files by extension"),
            ("find . -type d", "Find directories"),
            ("find . -mtime -7", "Modified in last 7 days"),
            ("find . -size +100M", "Files larger than 100MB"),
            ("find . -empty", "Find empty files/dirs"),
        ]),
        "grep" => Some(&[
            ("grep -r \"\" .", "Recursive search"),
            ("grep -rn \"\" .", "Recursive with line numbers"),
            ("grep -ri \"\" .", "Case-insensitive recursive"),
            ("grep -v \"\"", "Invert match (exclude)"),
            ("grep -E \"\"", "Extended regex"),
            ("grep -l \"\" .", "Only filenames"),
        ]),
        "curl" => Some(&[
            ("curl -s ", "Silent request"),
            ("curl -X POST -H 'Content-Type: application/json' -d '{}' ", "POST JSON"),
            ("curl -X PUT -H 'Content-Type: application/json' -d '{}' ", "PUT JSON"),
            ("curl -X DELETE ", "DELETE request"),
            ("curl -H 'Authorization: Bearer ' ", "Auth header"),
            ("curl -o output.file ", "Save to file"),
            ("curl -I ", "Headers only"),
            ("curl -L ", "Follow redirects"),
        ]),
        "tar" => Some(&[
            ("tar -czf archive.tar.gz ", "Create gzip archive"),
            ("tar -xzf ", "Extract gzip archive"),
            ("tar -cjf archive.tar.bz2 ", "Create bzip2 archive"),
            ("tar -xjf ", "Extract bzip2 archive"),
            ("tar -tf ", "List archive contents"),
        ]),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────
// File/directory path completion
// ─────────────────────────────────────────────────────────

/// Detects if the last token of `input` looks like a path prefix and returns completions.
fn path_completions(input: &str) -> Vec<SuggestionItem> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    let last = match tokens.last() {
        Some(t) if t.contains('/') || t.starts_with('.') || t.starts_with('~') => *t,
        _ => return Vec::new(),
    };

    let expanded = if last.starts_with('~') {
        let home = crate::ui::session::get_home_dir().to_string_lossy().to_string();
        last.replacen('~', &home, 1)
    } else {
        last.to_string()
    };

    let (dir_part, prefix) = if expanded.ends_with('/') {
        (expanded.clone(), String::new())
    } else {
        let p = PathBuf::from(&expanded);
        let parent = p.parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        let file_prefix = p.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        (parent, file_prefix)
    };

    let dir_to_read = if dir_part.is_empty() { ".".to_string() } else { dir_part.clone() };

    let mut completions = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir_to_read) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !prefix.starts_with('.') {
                continue; // skip hidden unless user typed dot
            }
            if prefix.is_empty() || name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let full = if dir_part == "." || dir_part.is_empty() {
                    if is_dir { format!("{}/", name) } else { name.clone() }
                } else {
                    if is_dir {
                        format!("{}/{}/", dir_part.trim_end_matches('/'), name)
                    } else {
                        format!("{}/{}", dir_part.trim_end_matches('/'), name)
                    }
                };
                // Reconstruct the full command with this path substituted
                let base_cmd = if tokens.len() > 1 {
                    tokens[..tokens.len() - 1].join(" ") + " " + &full
                } else {
                    full.clone()
                };
                completions.push(SuggestionItem {
                    display: if is_dir { format!("📁 {}", full) } else { format!("📄 {}", full) },
                    fill_cmd: base_cmd,
                    is_alias: false,
                    detail: if is_dir { "dir".into() } else { "file".into() },
                    source: SuggestionSource::Path,
                    score: if is_dir { 5.0 } else { 3.0 },
                });
                if completions.len() >= 8 {
                    break;
                }
            }
        }
    }
    completions.sort_by(|a, b| a.display.cmp(&b.display));
    completions
}

// ─────────────────────────────────────────────────────────
// Shell alias loader
// ─────────────────────────────────────────────────────────

use std::sync::{Arc, Mutex};
use std::time::Duration;

static ALIAS_CACHE: Mutex<Option<(Instant, Arc<Vec<UserAlias>>)>> = Mutex::new(None);
static HISTORY_CACHE: Mutex<Option<(Instant, Arc<Vec<String>>)>> = Mutex::new(None);

pub fn load_user_shell_aliases() -> Arc<Vec<UserAlias>> {
    if let Ok(guard) = ALIAS_CACHE.lock() {
        if let Some((time, ref aliases)) = *guard {
            if time.elapsed() < Duration::from_secs(30) {
                return Arc::clone(aliases);
            }
        }
    }

    let mut aliases = Vec::new();
    let mut config_paths = Vec::new();
    let home = crate::ui::session::get_home_dir();
    config_paths.push(home.join(".zshrc"));
    config_paths.push(home.join(".zsh_aliases"));
    config_paths.push(home.join(".bashrc"));
    config_paths.push(home.join(".bash_aliases"));
    config_paths.push(home.join(".config/fish/config.fish"));

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

    // Built-in default aliases
    let defaults: &[(&str, &str)] = &[
        ("gss", "git status -s"),
        ("gco", "git checkout"),
        ("gcb", "git checkout -b"),
        ("gp", "git push"),
        ("gpl", "git pull --rebase"),
        ("gl", "git log --oneline -n 20"),
        ("gd", "git diff"),
        ("gds", "git diff --staged"),
        ("ga", "git add"),
        ("gaa", "git add ."),
        ("gcm", "git commit -m"),
        ("ll", "ls -lah"),
        ("la", "ls -A"),
        ("l", "ls -CF"),
        ("nr", "npm run"),
        ("ni", "npm install"),
        ("nb", "npm run build"),
        ("nd", "npm run dev"),
        ("cr", "cargo run"),
        ("cb", "cargo build"),
        ("cc", "cargo check"),
        ("ct", "cargo test"),
        ("cf", "cargo fmt"),
        ("clr", "clear"),
        ("k", "kubectl"),
        ("kgp", "kubectl get pods"),
        ("kgs", "kubectl get services"),
        ("mkd", "mkdir -p"),
        ("..","cd .."),
        ("...","cd ../.."),
    ];

    for (name, target) in defaults {
        if !aliases.iter().any(|a| a.name == *name) {
            aliases.push(UserAlias {
                name: name.to_string(),
                target: target.to_string(),
            });
        }
    }

    let arc_aliases = Arc::new(aliases);
    if let Ok(mut guard) = ALIAS_CACHE.lock() {
        *guard = Some((Instant::now(), Arc::clone(&arc_aliases)));
    }

    arc_aliases
}

// ─────────────────────────────────────────────────────────
// Shell history loader
// ─────────────────────────────────────────────────────────

pub fn load_user_shell_history() -> Arc<Vec<String>> {
    if let Ok(guard) = HISTORY_CACHE.lock() {
        if let Some((time, ref history)) = *guard {
            if time.elapsed() < Duration::from_secs(30) {
                return Arc::clone(history);
            }
        }
    }

    let mut history = Vec::new();
    let mut hist_paths = Vec::new();
    let home = crate::ui::session::get_home_dir();
    hist_paths.push(home.join(".zsh_history"));
    hist_paths.push(home.join(".bash_history"));

    for path in hist_paths {
        if let Ok(bytes) = fs::read(&path) {
            let content = String::from_utf8_lossy(&bytes);
            for line in content.lines().rev().take(300) {
                let cmd = if let Some(idx) = line.find(';') {
                    &line[idx + 1..]
                } else {
                    line
                };
                let trimmed = cmd.trim().to_string();
                if !trimmed.is_empty() && trimmed.len() > 1 && !history.contains(&trimmed) {
                    history.push(trimmed);
                }
            }
        }
    }

    let arc_history = Arc::new(history);
    if let Ok(mut guard) = HISTORY_CACHE.lock() {
        *guard = Some((Instant::now(), Arc::clone(&arc_history)));
    }

    arc_history
}

// ─────────────────────────────────────────────────────────
// Fuzzy scoring helpers
// ─────────────────────────────────────────────────────────

/// Scores how well `candidate` matches `query`:
/// - Returns None if there's no meaningful match.
/// - Higher = better match.
fn fuzzy_score(query: &str, candidate: &str) -> Option<f64> {
    let q = query.to_lowercase();
    let c = candidate.to_lowercase();

    if c.starts_with(&q) {
        // Exact prefix – best
        return Some(100.0 + (1.0 / (candidate.len() as f64 + 1.0)));
    }

    // Word-boundary prefix: each space-separated word in candidate starts with a query char
    if q.len() >= 2 {
        let words: Vec<&str> = c.split_whitespace().collect();
        if words.iter().any(|w| w.starts_with(&q)) {
            return Some(60.0);
        }
    }

    // Subsequence match (fish-shell style)
    let mut qi = 0;
    let qb: Vec<char> = q.chars().collect();
    let mut consecutive = 0u32;
    let mut score = 0.0f64;
    for ch in c.chars() {
        if qi < qb.len() && ch == qb[qi] {
            qi += 1;
            consecutive += 1;
            score += consecutive as f64 * 2.0;
        } else {
            consecutive = 0;
        }
    }
    if qi == qb.len() && score > 0.0 {
        Some(score)
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────
// Main public entry point
// ─────────────────────────────────────────────────────────

/// Returns ranked suggestions for `input` using all available sources.
///
/// `session_history` – commands typed in the current terminal session.
/// `frecency` – optional frecency store to boost recently-used commands.
pub fn get_suggestions(
    input: &str,
    session_history: &[String],
    frecency: Option<&FrecencyStore>,
) -> Vec<SuggestionItem> {
    let q = input.trim();
    if q.is_empty() {
        return Vec::new();
    }

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut results: Vec<SuggestionItem> = Vec::new();

    // ── 1. Session history (most recent first, highest priority) ──────────
    for cmd in session_history.iter().rev() {
        if let Some(sc) = fuzzy_score(q, cmd) {
            if seen.insert(cmd.clone()) {
                let frecency_bonus = frecency.map(|f| f.score(cmd)).unwrap_or(0.0);
                results.push(SuggestionItem {
                    display: cmd.clone(),
                    fill_cmd: cmd.clone(),
                    is_alias: false,
                    detail: "session".to_string(),
                    source: SuggestionSource::Frecency, // show star for session hits
                    score: sc + 90.0 + frecency_bonus * 3.0,
                });
            }
        }
    }

    // ── 2. Shell history (~/.zsh_history / ~/.bash_history) ───────────────
    for cmd in load_user_shell_history().iter() {
        if let Some(sc) = fuzzy_score(q, cmd) {
            if seen.insert(cmd.clone()) {
                let frecency_bonus = frecency.map(|f| f.score(cmd)).unwrap_or(0.0);
                results.push(SuggestionItem {
                    display: cmd.clone(),
                    fill_cmd: cmd.clone(),
                    is_alias: false,
                    detail: "history".to_string(),
                    source: SuggestionSource::History,
                    score: sc + 60.0 + frecency_bonus,
                });
            }
        }
    }

    // ── 3. Shell aliases ───────────────────────────────────────────────────
    for alias in load_user_shell_aliases().iter() {
        if let Some(sc) = fuzzy_score(q, &alias.name).or_else(|| fuzzy_score(q, &alias.target)) {
            if seen.insert(alias.target.clone()) {
                let frecency_bonus = frecency.map(|f| f.score(&alias.name)).unwrap_or(0.0);
                results.push(SuggestionItem {
                    display: format!("{} → {}", alias.name, alias.target),
                    fill_cmd: alias.target.clone(),
                    is_alias: true,
                    detail: "alias".to_string(),
                    source: SuggestionSource::Alias,
                    score: sc + 50.0 + frecency_bonus,
                });
            }
        }
    }

    // ── 4. Frecency boost for commands not caught above ────────────────────
    if let Some(store) = frecency {
        for (cmd, _) in &store.entries {
            if let Some(sc) = fuzzy_score(q, cmd) {
                if seen.insert(cmd.clone()) {
                    let freq_score = store.score(cmd);
                    results.push(SuggestionItem {
                        display: cmd.clone(),
                        fill_cmd: cmd.clone(),
                        is_alias: false,
                        detail: "frecency".to_string(),
                        source: SuggestionSource::Frecency,
                        score: sc + freq_score * 3.0,
                    });
                }
            }
        }
    }

    // ── 5. Context-aware sub-commands ─────────────────────────────────────
    let first_token = q.split_whitespace().next().unwrap_or("");
    if let Some(subs) = known_subcommands(first_token) {
        for &(cmd, desc) in subs {
            if let Some(sc) = fuzzy_score(q, cmd) {
                if seen.insert(cmd.to_string()) {
                    let frecency_bonus = frecency.map(|f| f.score(cmd)).unwrap_or(0.0);
                    results.push(SuggestionItem {
                        display: cmd.to_string(),
                        fill_cmd: cmd.to_string(),
                        is_alias: false,
                        detail: desc.to_string(),
                        source: SuggestionSource::SubCommand,
                        score: sc + 30.0 + frecency_bonus,
                    });
                }
            }
        }
    } else if q.len() >= 1 && !q.contains(' ') {
        // Suggest known base commands when typing a single token
        let known_bases = [
            "git", "docker", "cargo", "npm", "npx", "kubectl",
            "ssh", "python3", "python", "pip3", "pip", "systemctl",
            "ls", "find", "grep", "curl", "tar",
        ];
        for base in known_bases {
            if let Some(sc) = fuzzy_score(q, base) {
                if seen.insert(base.to_string()) {
                    results.push(SuggestionItem {
                        display: base.to_string(),
                        fill_cmd: base.to_string(),
                        is_alias: false,
                        detail: "command".to_string(),
                        source: SuggestionSource::SubCommand,
                        score: sc + 20.0,
                    });
                }
            }
        }
    }

    // ── 6. File/directory path completions (last, only when relevant) ──────
    let path_items = path_completions(q);
    for item in path_items {
        if seen.insert(item.fill_cmd.clone()) {
            results.push(item);
        }
    }

    // ── Sort by score descending, truncate ────────────────────────────────
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(10);
    results
}
