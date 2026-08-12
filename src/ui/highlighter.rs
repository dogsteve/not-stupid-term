use eframe::egui;

/// High-performance syntax highlighter supporting major programming languages
pub fn highlight_code(ui: &egui::Ui, code: &str, ext: &str, font_size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let is_dark = ui.style().visuals.dark_mode;

    let font_id = egui::FontId::monospace(font_size);

    let default_color = if is_dark { egui::Color32::from_rgb(220, 220, 225) } else { egui::Color32::from_rgb(30, 30, 35) };
    let keyword_color = egui::Color32::from_rgb(204, 120, 50);  // Amber
    let type_color = egui::Color32::from_rgb(78, 201, 176);     // Cyan
    let comment_color = egui::Color32::from_rgb(128, 128, 128); // Gray

    let keywords: &[&str] = match ext {
        "rs" => &["fn", "pub", "struct", "enum", "impl", "let", "mut", "use", "mod", "return", "match", "if", "else", "for", "in", "while", "loop", "const", "static", "type", "where", "async", "await", "trait", "self", "Self"],
        "py" => &["def", "class", "import", "from", "return", "if", "else", "elif", "for", "while", "in", "with", "as", "pass", "None", "True", "False", "lambda", "try", "except", "raise", "async", "await"],
        "js" | "ts" | "jsx" | "tsx" => &["function", "const", "let", "var", "return", "if", "else", "import", "export", "from", "class", "async", "await", "true", "false", "null", "undefined", "new", "this", "interface", "type"],
        "c" | "cpp" | "h" | "hpp" => &["int", "float", "double", "char", "void", "struct", "class", "public", "private", "protected", "template", "typename", "using", "namespace", "if", "else", "for", "while", "return", "const", "auto", "include"],
        "go" => &["func", "package", "import", "struct", "interface", "type", "var", "const", "return", "if", "else", "for", "range", "go", "select", "chan", "defer", "nil", "true", "false"],
        "java" | "kt" => &["public", "private", "protected", "class", "interface", "void", "int", "double", "boolean", "return", "if", "else", "for", "while", "import", "package", "new", "this", "super", "fun", "val"],
        "sh" | "bash" | "zsh" => &["if", "then", "else", "fi", "for", "in", "do", "done", "while", "case", "esac", "echo", "export", "local", "return", "function", "sudo"],
        "sql" => &["SELECT", "FROM", "WHERE", "INSERT", "INTO", "UPDATE", "DELETE", "JOIN", "LEFT", "RIGHT", "INNER", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "CREATE", "TABLE", "DROP", "ALTER"],
        "html" | "xml" => &["html", "head", "body", "div", "span", "p", "a", "script", "style", "h1", "h2", "h3", "table", "tr", "td", "form", "input", "button"],
        "css" => &["margin", "padding", "color", "background", "border", "font-family", "font-size", "display", "flex", "grid", "position", "width", "height"],
        _ => &["fn", "let", "pub", "struct", "def", "function", "const", "var", "return", "if", "else", "import", "class"],
    };

    let tf_default = egui::TextFormat { font_id: font_id.clone(), color: default_color, ..Default::default() };
    let tf_keyword = egui::TextFormat { font_id: font_id.clone(), color: keyword_color, ..Default::default() };
    let tf_type = egui::TextFormat { font_id: font_id.clone(), color: type_color, ..Default::default() };
    let tf_comment = egui::TextFormat { font_id: font_id.clone(), color: comment_color, ..Default::default() };

    for line in code.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") || trimmed.starts_with("<!--") {
            job.append(line, 0.0, tf_comment.clone());
            continue;
        }

        let mut start_idx = 0;
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if b.is_ascii_alphabetic() || b == b'_' {
                if start_idx < i {
                    job.append(&line[start_idx..i], 0.0, tf_default.clone());
                }
                let word_start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[word_start..i];
                let tf = if keywords.contains(&word) {
                    tf_keyword.clone()
                } else if word.as_bytes().first().map_or(false, |c| c.is_ascii_uppercase()) {
                    tf_type.clone()
                } else {
                    tf_default.clone()
                };
                job.append(word, 0.0, tf);
                start_idx = i;
            } else {
                i += 1;
            }
        }
        if start_idx < len {
            job.append(&line[start_idx..len], 0.0, tf_default.clone());
        }
    }

    job
}
