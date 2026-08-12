/// Phosphor Icons — https://phosphoricons.com
/// Font: assets/fonts/Phosphor.ttf (Regular weight)
/// All codepoints are from the Unicode Private Use Area (PUA)
pub struct Icons;

impl Icons {
    // UI Actions
    pub const FOLDER: &'static str = "\u{e24a}";
    pub const FOLDER_OPEN: &'static str = "\u{e256}";
    pub const FILE: &'static str = "\u{e230}";
    pub const FILE_CODE: &'static str = "\u{e914}";
    pub const FILE_TEXT: &'static str = "\u{e23a}";
    pub const FILE_ZIP: &'static str = "\u{e958}";
    pub const FILE_MEDIA: &'static str = "\u{ea24}";
    pub const TERMINAL: &'static str = "\u{eae8}";
    pub const SERVER: &'static str = "\u{e288}";     // globe
    pub const GEAR: &'static str = "\u{e272}";       // gear-six
    pub const SEARCH: &'static str = "\u{e30c}";     // magnifying-glass
    pub const EDIT: &'static str = "\u{e3b4}";       // pencil-simple
    pub const EYE: &'static str = "\u{e220}";
    pub const SAVE: &'static str = "\u{e248}";       // floppy-disk
    pub const ADD: &'static str = "\u{e3d4}";        // plus
    pub const CLOSE: &'static str = "\u{e4f6}";      // x
    pub const BACK: &'static str = "\u{e058}";       // arrow-left
    pub const FORWARD: &'static str = "\u{e06c}";    // arrow-right
    pub const REFRESH: &'static str = "\u{e19e}";    // clock-clockwise
    pub const COMMAND: &'static str = "\u{e1c4}";
    pub const HISTORY: &'static str = "\u{e19a}";    // clock

    // Extra UI icons
    pub const TRASH: &'static str = "\u{e4a6}";
    pub const COPY: &'static str = "\u{e1ca}";
    pub const CLIPBOARD: &'static str = "\u{e196}";
    pub const DESKTOP: &'static str = "\u{e560}";
    pub const MONITOR: &'static str = "\u{e32e}";
    pub const WRENCH: &'static str = "\u{e5d4}";
    pub const SLIDERS: &'static str = "\u{e432}";
    pub const LIST: &'static str = "\u{e2f0}";
    pub const HOUSE: &'static str = "\u{e2c2}";
    pub const PALETTE: &'static str = "\u{e6c8}";
    pub const PLAY: &'static str = "\u{e3d0}";
    pub const STOP: &'static str = "\u{e46c}";
    pub const PAUSE: &'static str = "\u{e39e}";
    pub const MINUS: &'static str = "\u{e32a}";
    pub const DOTS: &'static str = "\u{e1fe}";       // dots-three
    pub const PACKAGE: &'static str = "\u{e390}";
    pub const DATABASE: &'static str = "\u{e1de}";
    pub const INFO: &'static str = "\u{e2ce}";
    pub const WARNING: &'static str = "\u{e4e0}";
    pub const CHECK: &'static str = "\u{e182}";
    pub const STAR: &'static str = "\u{e46a}";
    pub const HEART: &'static str = "\u{e2a8}";
    pub const SHIELD: &'static str = "\u{e40a}";
    pub const LIGHTNING: &'static str = "\u{e2de}";
    pub const SUN: &'static str = "\u{e472}";
    pub const MOON: &'static str = "\u{e330}";
    pub const APP_WINDOW: &'static str = "\u{e5da}";
    pub const SQUARE: &'static str = "\u{e456}";
    pub const BROWSERS: &'static str = "\u{e0f6}";
    pub const TABS: &'static str = "\u{e778}";
    pub const CARET_LEFT: &'static str = "\u{e138}";
    pub const CARET_RIGHT: &'static str = "\u{e13a}";
    pub const CARET_UP: &'static str = "\u{e13c}";
    pub const NOTE: &'static str = "\u{e348}";
    pub const IMAGE: &'static str = "\u{e2ca}";

    // Git icons
    pub const GIT_BRANCH: &'static str = "\u{e278}";
    pub const GIT_COMMIT: &'static str = "\u{e27a}";
    pub const GIT_DIFF: &'static str = "\u{e27c}";
    pub const GIT_FORK: &'static str = "\u{e27e}";
    pub const GIT_MERGE: &'static str = "\u{e280}";

    /// Returns Phosphor icon for file type based on extension
    pub fn get_file_icon(file_name: &str) -> &'static str {
        let name_lower = file_name.to_lowercase();
        if name_lower == "dockerfile" || name_lower.starts_with("dockerfile.") {
            return "\u{e390}"; // package
        }
        if name_lower == ".gitignore" || name_lower.starts_with(".git") {
            return "\u{e278}"; // git-branch
        }

        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext.to_lowercase().as_str() {
            "rs" => "\u{eb28}",                                        // file-rs
            "py" => "\u{eb2c}",                                        // file-py
            "js" | "mjs" | "cjs" => "\u{eb24}",                        // file-js
            "ts" | "mts" | "cts" => "\u{eb26}",                        // file-ts
            "jsx" | "tsx" => "\u{e5e4}",                               // atom
            "html" | "htm" => "\u{eb38}",                              // file-html
            "css" | "scss" | "less" => "\u{eb34}",                     // file-css
            "json" => "\u{e23a}",                                      // file-text
            "md" | "markdown" => "\u{ed50}",                           // file-md
            "toml" | "yaml" | "yml" | "ini" | "cfg" | "env" => "\u{e272}", // gear-six
            "sh" | "zsh" | "bash" => "\u{eae8}",                      // terminal-window
            "c" | "cpp" | "h" | "hpp" | "cc" => "\u{e1bc}",           // code
            "go" => "\u{e914}",                                        // file-code
            "java" | "jar" => "\u{e914}",                              // file-code
            "kt" | "kts" => "\u{e914}",                                // file-code
            "swift" => "\u{e914}",                                     // file-code
            "php" => "\u{e914}",                                       // file-code
            "rb" => "\u{e1ec}",                                        // diamond
            "lua" => "\u{e330}",                                       // moon
            "sql" => "\u{e1de}",                                       // database
            "svg" => "\u{ed08}",                                       // file-svg
            "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => "\u{e958}", // file-zip
            "png" => "\u{eb18}",                                       // file-png
            "jpg" | "jpeg" => "\u{eb1a}",                              // file-jpg
            "gif" | "webp" => "\u{ea24}",                              // file-image
            "pdf" => "\u{e702}",                                       // file-pdf
            "doc" | "docx" => "\u{eb1e}",                              // file-doc
            "mp4" | "mkv" | "avi" | "mov" => "\u{ea22}",              // file-video
            "mp3" | "wav" | "flac" | "ogg" => "\u{ea20}",             // file-audio
            _ => "\u{e230}",                                           // file (generic)
        }
    }

    /// Returns a [`egui::RichText`] that renders `glyph` using the dedicated
    /// Creates a RichText for a single icon.
    /// Note: we use FontFamily::Name("phosphor") to avoid collisions with
    /// which UI font is currently active.
    pub fn rich(glyph: &'static str, size: f32) -> egui::RichText {
        egui::RichText::new(glyph)
            .family(egui::FontFamily::Name("phosphor".into()))
            .size(size)
    }

    /// Creates a LayoutJob for an icon followed by text.
    /// Prevents the icon from being overridden by normal text fonts that have PUA glyphs.
    pub fn job(glyph: &'static str, text: &str, size: f32) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            glyph,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(size, egui::FontFamily::Name("phosphor".into())),
                color: egui::Color32::PLACEHOLDER,
                ..Default::default()
            },
        );
        job.append(
            &format!(" {}", text),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(size, egui::FontFamily::Proportional),
                color: egui::Color32::PLACEHOLDER,
                ..Default::default()
            },
        );
        job
    }

    /// Builds a LayoutJob for an icon string + label string with proper Phosphor font family
    pub fn label_job(icon: &str, text: &str, size: f32, color: egui::Color32) -> egui::text::LayoutJob {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            icon,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(size, egui::FontFamily::Name("phosphor".into())),
                color,
                ..Default::default()
            },
        );
        job.append(
            " ",
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(size),
                color,
                ..Default::default()
            },
        );
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(size),
                color,
                ..Default::default()
            },
        );
        job
    }
}
