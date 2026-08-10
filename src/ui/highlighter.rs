use eframe::egui;

/// Universal syntax highlighter supporting all major programming languages
pub fn highlight_code(ui: &egui::Ui, code: &str, ext: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let is_dark = ui.style().visuals.dark_mode;

    let default_color = if is_dark { egui::Color32::from_rgb(220, 220, 225) } else { egui::Color32::from_rgb(30, 30, 35) };
    let keyword_color = egui::Color32::from_rgb(204, 120, 50);  // Orange/Amber
    let type_color = egui::Color32::from_rgb(78, 201, 176);     // Teal/Cyan
    let string_color = egui::Color32::from_rgb(106, 135, 89);   // Olive/Green
    let comment_color = egui::Color32::from_rgb(128, 128, 128); // Gray

    let keywords: Vec<&str> = match ext {
        "rs" => vec!["fn", "pub", "struct", "enum", "impl", "let", "mut", "use", "mod", "return", "match", "if", "else", "for", "in", "while", "loop", "const", "static", "type", "where", "async", "await", "trait", "self", "Self"],
        "py" => vec!["def", "class", "import", "from", "return", "if", "else", "elif", "for", "while", "in", "with", "as", "pass", "None", "True", "False", "lambda", "try", "except", "raise", "async", "await"],
        "js" | "ts" | "jsx" | "tsx" => vec!["function", "const", "let", "var", "return", "if", "else", "import", "export", "from", "class", "async", "await", "true", "false", "null", "undefined", "new", "this", "interface", "type"],
        "c" | "cpp" | "h" | "hpp" => vec!["int", "float", "double", "char", "void", "struct", "class", "public", "private", "protected", "template", "typename", "using", "namespace", "if", "else", "for", "while", "return", "const", "auto", "include"],
        "go" => vec!["func", "package", "import", "struct", "interface", "type", "var", "const", "return", "if", "else", "for", "range", "go", "select", "chan", "defer", "nil", "true", "false"],
        "java" | "kt" => vec!["public", "private", "protected", "class", "interface", "void", "int", "double", "boolean", "return", "if", "else", "for", "while", "import", "package", "new", "this", "super", "fun", "val"],
        "sh" | "bash" | "zsh" => vec!["if", "then", "else", "fi", "for", "in", "do", "done", "while", "case", "esac", "echo", "export", "local", "return", "function", "sudo"],
        "sql" => vec!["SELECT", "FROM", "WHERE", "INSERT", "INTO", "UPDATE", "DELETE", "JOIN", "LEFT", "RIGHT", "INNER", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "CREATE", "TABLE", "DROP", "ALTER", "select", "from", "where"],
        "html" | "xml" => vec!["html", "head", "body", "div", "span", "p", "a", "script", "style", "h1", "h2", "h3", "table", "tr", "td", "form", "input", "button"],
        "css" => vec!["margin", "padding", "color", "background", "border", "font-family", "font-size", "display", "flex", "grid", "position", "width", "height"],
        _ => vec!["fn", "let", "pub", "struct", "def", "function", "const", "var", "return", "if", "else", "import", "class"],
    };

    for line in code.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") || trimmed.starts_with("<!--") {
            job.append(line, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(13.0),
                color: comment_color,
                ..Default::default()
            });
            job.append("\n", 0.0, egui::TextFormat::default());
            continue;
        }

        let mut current = String::new();
        for ch in line.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch);
            } else {
                if !current.is_empty() {
                    let color = if keywords.contains(&current.as_str()) {
                        keyword_color
                    } else if current.chars().next().map_or(false, |c| c.is_uppercase()) {
                        type_color
                    } else {
                        default_color
                    };

                    job.append(&current, 0.0, egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color,
                        ..Default::default()
                    });
                    current.clear();
                }

                let ch_str = ch.to_string();
                let color = if ch == '"' || ch == '\'' { string_color } else { default_color };
                job.append(&ch_str, 0.0, egui::TextFormat {
                    font_id: egui::FontId::monospace(13.0),
                    color,
                    ..Default::default()
                });
            }
        }

        if !current.is_empty() {
            let color = if keywords.contains(&current.as_str()) {
                keyword_color
            } else if current.chars().next().map_or(false, |c| c.is_uppercase()) {
                type_color
            } else {
                default_color
            };

            job.append(&current, 0.0, egui::TextFormat {
                font_id: egui::FontId::monospace(13.0),
                color,
                ..Default::default()
            });
        }

        job.append("\n", 0.0, egui::TextFormat::default());
    }

    job
}
