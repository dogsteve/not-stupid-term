use eframe::egui;

use crate::ui::settings::AppTheme;

pub fn apply_theme(ctx: &egui::Context, config: &crate::ui::settings::AppConfig) {
    let is_dark = match config.theme {
        AppTheme::DefaultLight | AppTheme::SolarizedLight | AppTheme::GruvboxLight | AppTheme::AyuLight | AppTheme::SnazzyLight => false,
        _ => true,
    };

    let mut visuals = if is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    let alpha = (255.0 * (1.0 - config.blur_level)) as u8;

    let (bg, panel_bg, text, accent) = match config.theme {
        AppTheme::DefaultDark => ((18, 18, 22), (12, 12, 14), (230, 230, 230), (60, 140, 255)),
        AppTheme::DefaultLight => ((240, 240, 245), (220, 220, 225), (40, 40, 40), (40, 120, 240)),
        AppTheme::WhiteSurDark => ((32, 34, 38), (24, 25, 28), (240, 240, 245), (10, 132, 255)),
        AppTheme::OrchisDark => ((25, 27, 34), (18, 19, 24), (225, 230, 240), (100, 110, 245)),
        AppTheme::CandySweet => ((28, 20, 40), (20, 14, 30), (250, 230, 255), (255, 60, 140)),
        AppTheme::Dracula => ((40, 42, 54), (33, 34, 44), (248, 248, 242), (189, 147, 249)),
        AppTheme::Nord => ((46, 52, 64), (59, 66, 82), (216, 222, 233), (136, 192, 208)),
        AppTheme::Monokai => ((39, 40, 34), (30, 31, 28), (248, 248, 242), (249, 38, 114)),
        AppTheme::SolarizedDark => ((0, 43, 54), (7, 54, 66), (131, 148, 150), (181, 137, 0)),
        AppTheme::SolarizedLight => ((253, 246, 227), (238, 232, 213), (101, 123, 131), (181, 137, 0)),
        AppTheme::GitHubDark => ((13, 17, 23), (1, 4, 9), (201, 209, 217), (88, 166, 255)),
        AppTheme::TokyoNight => ((26, 27, 38), (22, 22, 30), (192, 202, 245), (122, 162, 247)),
        AppTheme::Catppuccin => ((30, 30, 46), (24, 24, 37), (205, 214, 244), (137, 180, 250)),
        AppTheme::OneDark => ((40, 44, 52), (33, 37, 43), (171, 178, 191), (97, 175, 239)),
        AppTheme::GruvboxDark => ((40, 40, 40), (29, 32, 33), (235, 219, 178), (254, 128, 25)),
        AppTheme::GruvboxLight => ((253, 244, 193), (238, 224, 176), (60, 56, 54), (175, 58, 3)),
        AppTheme::AyuDark => ((15, 20, 25), (11, 15, 20), (230, 225, 207), (255, 180, 84)),
        AppTheme::AyuLight => ((250, 250, 250), (240, 240, 240), (92, 97, 102), (255, 153, 64)),
        AppTheme::AyuMirage => ((33, 39, 51), (25, 30, 42), (217, 215, 206), (255, 204, 102)),
        AppTheme::MaterialDark => ((33, 33, 33), (48, 48, 48), (238, 255, 255), (130, 170, 255)),
        AppTheme::MaterialOcean => ((15, 17, 26), (9, 11, 16), (143, 147, 162), (130, 170, 255)),
        AppTheme::Palenight => ((41, 45, 62), (30, 34, 49), (166, 172, 205), (199, 146, 234)),
        AppTheme::SynthWave84 => ((38, 35, 53), (25, 21, 38), (255, 255, 255), (255, 126, 219)),
        AppTheme::Cyberpunk => ((26, 9, 35), (16, 5, 21), (255, 239, 13), (0, 255, 153)),
        AppTheme::RosePine => ((25, 23, 36), (31, 29, 46), (224, 222, 244), (235, 188, 186)),
        AppTheme::Everforest => ((43, 51, 57), (30, 35, 38), (211, 198, 170), (167, 192, 128)),
        AppTheme::Kanagawa => ((31, 31, 40), (22, 22, 29), (220, 215, 186), (126, 156, 216)),
        AppTheme::Nightfox => ((25, 35, 48), (17, 24, 39), (205, 214, 244), (113, 156, 214)),
        AppTheme::Moonlight => ((34, 36, 54), (30, 32, 48), (200, 208, 224), (130, 170, 255)),
        AppTheme::VitesseDark => ((18, 18, 18), (24, 24, 24), (219, 219, 219), (77, 147, 117)),
        AppTheme::Horizon => ((28, 30, 38), (21, 23, 29), (213, 216, 218), (229, 192, 123)),
        AppTheme::Poimandres => ((27, 30, 40), (23, 25, 34), (166, 172, 205), (93, 238, 214)),
        AppTheme::BlulocoDark => ((40, 44, 52), (33, 37, 43), (171, 178, 191), (97, 175, 239)),
        AppTheme::ChallengerDeep => ((30, 28, 49), (20, 19, 34), (198, 200, 209), (145, 221, 255)),
        AppTheme::SnazzyLight => ((250, 251, 252), (243, 244, 246), (56, 58, 66), (255, 92, 87)),
        AppTheme::WinterIsComing => ((1, 22, 39), (1, 19, 33), (214, 222, 235), (126, 206, 253)),
        AppTheme::Vesper => ((16, 16, 16), (10, 10, 10), (200, 200, 200), (255, 120, 100)),
    };

    visuals.window_fill = egui::Color32::from_rgba_unmultiplied(bg.0, bg.1, bg.2, alpha);
    visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(panel_bg.0, panel_bg.1, panel_bg.2, alpha);
    visuals.override_text_color = Some(egui::Color32::from_rgb(text.0, text.1, text.2));
    visuals.selection.bg_fill = egui::Color32::from_rgb(accent.0, accent.1, accent.2);
    visuals.selection.stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);

    let text_c = egui::Color32::from_rgb(text.0, text.1, text.2);
    let accent_c = egui::Color32::from_rgb(accent.0, accent.1, accent.2);

    // Extreme & faint backgrounds
    visuals.extreme_bg_color = if is_dark {
        egui::Color32::from_rgb(panel_bg.0.saturating_sub(5), panel_bg.1.saturating_sub(5), panel_bg.2.saturating_sub(5))
    } else {
        egui::Color32::from_rgb(255, 255, 255)
    };
    visuals.faint_bg_color = if is_dark {
        egui::Color32::from_rgb(bg.0.saturating_add(8), bg.1.saturating_add(8), bg.2.saturating_add(8))
    } else {
        egui::Color32::from_rgb(bg.0.saturating_sub(8), bg.1.saturating_sub(8), bg.2.saturating_sub(8))
    };

    // === Widget styling: buttons, dropdowns, menus, sliders ===
    let rounding = egui::Rounding::same(6.0);
    let subtle_stroke = egui::Stroke::new(1.0, if is_dark {
        egui::Color32::from_white_alpha(15)
    } else {
        egui::Color32::from_black_alpha(20)
    });
    let hover_stroke = egui::Stroke::new(1.0, accent_c.linear_multiply(0.5));

    // Non-interactive widgets (labels, separators)
    visuals.widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::TRANSPARENT;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_c.linear_multiply(0.7));
    visuals.widgets.noninteractive.rounding = rounding;

    // Inactive widgets (buttons at rest)
    visuals.widgets.inactive.bg_fill = if is_dark {
        egui::Color32::from_white_alpha(10)
    } else {
        egui::Color32::from_black_alpha(8)
    };
    visuals.widgets.inactive.weak_bg_fill = if is_dark {
        egui::Color32::from_white_alpha(10)
    } else {
        egui::Color32::from_black_alpha(8)
    };
    visuals.widgets.inactive.bg_stroke = subtle_stroke;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_c.linear_multiply(0.85));
    visuals.widgets.inactive.rounding = rounding;

    // Hovered widgets — KEY: visible highlight
    visuals.widgets.hovered.bg_fill = if is_dark {
        accent_c.linear_multiply(0.2)
    } else {
        accent_c.linear_multiply(0.12)
    };
    visuals.widgets.hovered.weak_bg_fill = if is_dark {
        accent_c.linear_multiply(0.2)
    } else {
        accent_c.linear_multiply(0.12)
    };
    visuals.widgets.hovered.bg_stroke = hover_stroke;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, text_c);
    visuals.widgets.hovered.rounding = rounding;

    // Active/pressed widgets
    visuals.widgets.active.bg_fill = accent_c.linear_multiply(0.35);
    visuals.widgets.active.weak_bg_fill = accent_c.linear_multiply(0.35);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent_c.linear_multiply(0.7));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, text_c);
    visuals.widgets.active.rounding = rounding;

    // Open (combo box expanded, menu open)
    visuals.widgets.open.bg_fill = if is_dark {
        egui::Color32::from_white_alpha(18)
    } else {
        egui::Color32::from_black_alpha(12)
    };
    visuals.widgets.open.weak_bg_fill = if is_dark {
        egui::Color32::from_white_alpha(18)
    } else {
        egui::Color32::from_black_alpha(12)
    };
    visuals.widgets.open.bg_stroke = hover_stroke;
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.5, text_c);
    visuals.widgets.open.rounding = rounding;

    // Window & menu chrome
    visuals.window_stroke = egui::Stroke::new(1.0, if is_dark {
        egui::Color32::from_white_alpha(20)
    } else {
        egui::Color32::from_black_alpha(15)
    });
    visuals.window_rounding = egui::Rounding::same(config.window_rounding);
    visuals.menu_rounding = egui::Rounding::same(8.0);
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0.0, 4.0].into(),
        blur: 12.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(if is_dark { 120 } else { 40 }),
    };
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0.0, 6.0].into(),
        blur: 20.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(if is_dark { 160 } else { 50 }),
    };

    // Separator & striped rows
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.5_f32, if is_dark {
        egui::Color32::from_white_alpha(12)
    } else {
        egui::Color32::from_black_alpha(10)
    });

    let mut style = (*ctx.style()).clone();
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);

    style.text_styles = [
        (egui::TextStyle::Small, egui::FontId::proportional((config.ui_font_size - 2.0).max(9.0))),
        (egui::TextStyle::Body, egui::FontId::proportional(config.ui_font_size)),
        (egui::TextStyle::Button, egui::FontId::proportional(config.ui_font_size)),
        (egui::TextStyle::Heading, egui::FontId::proportional(config.ui_font_size + 5.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(config.mono_font_size)),
    ].into();

    ctx.set_style(style);
    ctx.set_visuals(visuals);
}

pub fn apply_font(ctx: &egui::Context, ui_font_family: &str, mono_font_family: &str) {
    let mut fonts = egui::FontDefinitions::default();

    // ── Register Phosphor Icons font (PUA glyphs U+E000–U+F8FF) ──────────
    // Phosphor is registered in two ways:
    //   1. As a dedicated named family "phosphor" → used explicitly for icon widgets
    //      via FontFamily::Name("phosphor".into()), guaranteeing correct glyph lookup.
    //   2. As the last fallback on Proportional/Monospace (for any embedded icon chars
    //      that don't specify an explicit font).
    fonts.font_data.insert(
        "phosphor".to_owned(),
        egui::FontData::from_static(include_bytes!("../../assets/fonts/Phosphor.ttf")),
    );
    // Named family for explicit icon rendering
    fonts.families.insert(
        egui::FontFamily::Name("phosphor".into()),
        vec!["phosphor".to_owned()],
    );

    // ── UI Proportional Font (inserted at index 0 = highest priority) ─────
    match ui_font_family {
        "Inter" => {
            fonts.font_data.insert(
                "Inter".to_owned(),
                egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Regular.ttf")),
            );
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Inter".to_owned());
        }
        "Noto Sans" => {
            fonts.font_data.insert(
                "NotoSans".to_owned(),
                egui::FontData::from_static(include_bytes!("../../assets/fonts/NotoSans-Regular.ttf")),
            );
            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "NotoSans".to_owned());
        }
        _ => {
            // "System Default" → egui's built-in Ubuntu-Light stays at index 0.
        }
    }

    // ── Monospace Font (terminal + code editor) ───────────────────────────
    match mono_font_family {
        "JetBrains Mono" => {
            fonts.font_data.insert(
                "JetBrainsMono".to_owned(),
                egui::FontData::from_static(include_bytes!("../../assets/JetBrainsMono-Regular.ttf")),
            );
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "JetBrainsMono".to_owned());
        }
        "SF Mono" => {
            let candidates = [
                "/System/Applications/Utilities/Terminal.app/Contents/Resources/Fonts/SFMono-Regular.otf",
                "/Library/Fonts/SF-Mono-Regular.otf",
                "/System/Library/Fonts/SFMono-Regular.otf",
            ];
            let mut loaded = false;
            for path in candidates {
                if let Ok(data) = std::fs::read(path) {
                    fonts.font_data.insert("SFMono".to_owned(), egui::FontData::from_owned(data));
                    fonts.families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .insert(0, "SFMono".to_owned());
                    loaded = true;
                    break;
                }
            }
            if !loaded {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
        }
        "Monaco" => {
            let loaded = if let Ok(data) = std::fs::read("/System/Library/Fonts/Monaco.ttf") {
                fonts.font_data.insert("Monaco".to_owned(), egui::FontData::from_owned(data));
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Monaco".to_owned());
                true
            } else {
                false
            };
            if !loaded {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
        }
        "Menlo" => {
            let loaded = if let Ok(data) = std::fs::read("/System/Library/Fonts/Menlo.ttc") {
                fonts.font_data.insert("Menlo".to_owned(), egui::FontData::from_owned(data));
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Menlo".to_owned());
                true
            } else {
                false
            };
            if !loaded {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
        }
        "Consolas" => {
            let loaded = if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\consola.ttf") {
                fonts.font_data.insert("Consolas".to_owned(), egui::FontData::from_owned(data));
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "Consolas".to_owned());
                true
            } else {
                false
            };
            if !loaded {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
        }
        "Cascadia Code" => {
            let loaded = if let Ok(data) = std::fs::read(r"C:\Windows\Fonts\CascadiaCode.ttf") {
                fonts.font_data.insert("CascadiaCode".to_owned(), egui::FontData::from_owned(data));
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "CascadiaCode".to_owned());
                true
            } else {
                false
            };
            if !loaded {
                fonts.font_data.insert(
                    "FiraCode".to_owned(),
                    egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
                );
                fonts.families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "FiraCode".to_owned());
            }
        }
        "System Default" | "System Monospace" => {
            // Egui built-in monospace
        }
        _ => {
            // Default: Fira Code
            fonts.font_data.insert(
                "FiraCode".to_owned(),
                egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
            );
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "FiraCode".to_owned());
        }
    }

    // ── Phosphor as LAST fallback for both families ───────────────────────
    // This is the correct position: only reached when the primary font
    // has no glyph for a codepoint (e.g. PUA icon characters).
    fonts.families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("phosphor".to_owned());
    fonts.families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("phosphor".to_owned());

    ctx.set_fonts(fonts);
}
