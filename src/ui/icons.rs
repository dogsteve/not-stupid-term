pub struct Icons;

impl Icons {
    // Standard Unicode Emoji — works everywhere, no font dependency
    pub const FOLDER: &'static str = "📁";
    pub const FOLDER_OPEN: &'static str = "📂";
    pub const FILE: &'static str = "📄";
    pub const FILE_CODE: &'static str = "📝";
    pub const FILE_TEXT: &'static str = "📄";
    pub const FILE_ZIP: &'static str = "📦";
    pub const FILE_MEDIA: &'static str = "🖼";
    pub const TERMINAL: &'static str = "🖥";
    pub const SERVER: &'static str = "🌐";
    pub const GEAR: &'static str = "⚙";
    pub const SEARCH: &'static str = "🔍";
    pub const EDIT: &'static str = "✏";
    pub const EYE: &'static str = "👁";
    pub const SAVE: &'static str = "💾";
    pub const ADD: &'static str = "+";
    pub const CLOSE: &'static str = "✕";
    pub const BACK: &'static str = "◀";
    pub const FORWARD: &'static str = "▶";
    pub const REFRESH: &'static str = "⟳";
    pub const COMMAND: &'static str = "⌘";
    pub const HISTORY: &'static str = "🕐";

    /// Returns a Unicode emoji icon for any file type extension
    pub fn get_file_icon(file_name: &str) -> &'static str {
        let name_lower = file_name.to_lowercase();
        if name_lower == "dockerfile" || name_lower.starts_with("dockerfile.") {
            return "🐳";
        }
        if name_lower == ".gitignore" || name_lower.starts_with(".git") {
            return "🌿";
        }

        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext.to_lowercase().as_str() {
            "rs" => "🦀",
            "py" => "🐍",
            "js" | "mjs" | "cjs" => "🟨",
            "ts" | "mts" | "cts" => "🔷",
            "jsx" | "tsx" => "⚛",
            "html" | "htm" => "🌐",
            "css" | "scss" | "less" => "🎨",
            "json" => "📋",
            "md" | "markdown" => "📝",
            "toml" | "yaml" | "yml" | "ini" | "cfg" | "env" => "⚙",
            "sh" | "zsh" | "bash" => "🖥",
            "c" | "cpp" | "h" | "hpp" | "cc" => "🔧",
            "go" => "🐹",
            "java" | "jar" => "☕",
            "kt" | "kts" => "🟣",
            "swift" => "🐦",
            "php" => "🐘",
            "rb" => "💎",
            "lua" => "🌙",
            "sql" => "🗄",
            "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => "📦",
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => "🖼",
            "mp4" | "mkv" | "avi" | "mp3" | "wav" | "flac" => "🎵",
            _ => "📄",
        }
    }
}
