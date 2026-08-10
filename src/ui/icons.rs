pub struct Icons;

impl Icons {
    // REAL Nerd Font Symbol Unicode Codepoints
    pub const FOLDER: &'static str = "\u{f07b}";
    pub const FOLDER_OPEN: &'static str = "\u{f07c}";
    pub const FILE: &'static str = "\u{f15c}";
    pub const FILE_CODE: &'static str = "\u{f121}";
    pub const FILE_TEXT: &'static str = "\u{f15c}";
    pub const FILE_ZIP: &'static str = "\u{f1c6}";
    pub const FILE_MEDIA: &'static str = "\u{f1c5}";
    pub const TERMINAL: &'static str = "\u{ea85}";
    pub const SERVER: &'static str = "\u{f233}";
    pub const GEAR: &'static str = "\u{f013}";
    pub const SEARCH: &'static str = "\u{f002}";
    pub const EDIT: &'static str = "\u{f044}";
    pub const EYE: &'static str = "\u{f06e}";
    pub const SAVE: &'static str = "\u{f0c7}";
    pub const ADD: &'static str = "\u{f067}";
    pub const CLOSE: &'static str = "\u{f00d}";
    pub const BACK: &'static str = "\u{f060}";
    pub const FORWARD: &'static str = "\u{f061}";
    pub const REFRESH: &'static str = "\u{f021}";
    pub const COMMAND: &'static str = "\u{f120}";
    pub const HISTORY: &'static str = "\u{f1da}";

    /// Returns the exact REAL Nerd Font glyph for any file type extension
    pub fn get_file_icon(file_name: &str) -> &'static str {
        let name_lower = file_name.to_lowercase();
        if name_lower == "dockerfile" || name_lower.starts_with("dockerfile.") {
            return "\u{e7b0}"; // Docker logo 🐳
        }
        if name_lower == ".gitignore" || name_lower.starts_with(".git") {
            return "\u{f1d3}"; // Git logo 🌿
        }

        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext.to_lowercase().as_str() {
            "rs" => "\u{e7a8}",                                      // Rust 🦀
            "py" => "\u{e73c}",                                      // Python 🐍
            "js" | "mjs" | "cjs" => "\u{e74e}",                       // JavaScript 🟨
            "ts" | "mts" | "cts" => "\u{e628}",                       // TypeScript 🔷
            "jsx" | "tsx" => "\u{e7ba}",                             // React ⚛️
            "html" | "htm" => "\u{e736}",                            // HTML5 🌐
            "css" | "scss" | "less" => "\u{e749}",                    // CSS3 🎨
            "json" => "\u{e60b}",                                    // JSON ⚙️
            "md" | "markdown" => "\u{e609}",                         // Markdown 📝
            "toml" | "yaml" | "yml" | "ini" | "cfg" | "env" => "\u{f013}", // Config ⚙️
            "sh" | "zsh" | "bash" => "\u{e795}",                     // Shell 🖥️
            "c" | "cpp" | "h" | "hpp" | "cc" => "\u{e61d}",          // C/C++
            "go" => "\u{e627}",                                      // Go 🐹
            "java" | "jar" => "\u{e738}",                            // Java ☕
            "kt" | "kts" => "\u{e634}",                              // Kotlin
            "swift" => "\u{e755}",                                   // Swift
            "php" => "\u{e73d}",                                     // PHP
            "rb" => "\u{e739}",                                      // Ruby
            "lua" => "\u{e620}",                                     // Lua
            "sql" => "\u{f1c0}",                                     // Database 🗄️
            "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => "\u{f1c6}", // Zip 📦
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => "\u{f1c5}", // Image 🖼️
            "mp4" | "mkv" | "avi" | "mp3" | "wav" | "flac" => "\u{f1c8}", // Media 🎵
            _ => "\u{f15c}",                                         // Default File 📄
        }
    }
}
