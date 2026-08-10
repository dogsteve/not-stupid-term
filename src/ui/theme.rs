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
    
    let dark_factor = if is_dark { 5 } else { 0 };
    visuals.extreme_bg_color = egui::Color32::from_rgba_unmultiplied(panel_bg.0.saturating_sub(dark_factor), panel_bg.1.saturating_sub(dark_factor), panel_bg.2.saturating_sub(dark_factor), alpha);
    let light_factor = if is_dark { 10 } else { 0 };
    visuals.faint_bg_color = egui::Color32::from_rgba_unmultiplied(bg.0.saturating_add(light_factor), bg.1.saturating_add(light_factor), bg.2.saturating_add(light_factor), alpha);
    
    visuals.widgets.noninteractive.bg_fill = visuals.panel_fill;
    visuals.widgets.inactive.bg_fill = visuals.faint_bg_color;
    visuals.widgets.hovered.bg_fill = visuals.selection.bg_fill.linear_multiply(0.3);
    visuals.widgets.active.bg_fill = visuals.selection.bg_fill.linear_multiply(0.5);

    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(if is_dark { 45 } else { 200 }));

    visuals.window_rounding = egui::Rounding::same(config.window_rounding);
    visuals.menu_rounding = egui::Rounding::same(8.0);

    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0.0, 8.0].into(),
        blur: 24.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(180),
    };

    let mut style = (*ctx.style()).clone();
    style.spacing.window_margin = egui::Margin::same(12.0);

    for (text_style, font_id) in style.text_styles.iter_mut() {
        match text_style {
            egui::TextStyle::Body | egui::TextStyle::Button => {
                font_id.size = 13.0;
            }
            egui::TextStyle::Heading => {
                font_id.size = 18.0;
            }
            _ => {}
        }
    }

    ctx.set_style(style);
    ctx.set_visuals(visuals);
}

pub fn apply_font(ctx: &egui::Context, font_family: &str) {
    let mut fonts = egui::FontDefinitions::default();

    match font_family {
        "JetBrains Mono" => {
            fonts.font_data.insert(
                "JetBrainsMono".to_owned(),
                egui::FontData::from_static(include_bytes!("../../assets/JetBrainsMono-Regular.ttf")),
            );
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "JetBrainsMono".to_owned());
            fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "JetBrainsMono".to_owned());
        }
        "Fira Code" | _ => {
            fonts.font_data.insert(
                "FiraCode".to_owned(),
                egui::FontData::from_static(include_bytes!("../../assets/FiraCode-Regular.ttf")),
            );
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "FiraCode".to_owned());
            fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "FiraCode".to_owned());
        }
    }

    ctx.set_fonts(fonts);
}
