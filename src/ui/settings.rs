use eframe::egui;
use crate::ui::icons::Icons;
use crate::ui::window_framework::{WindowAction, WindowApp};

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AppTheme {
    DefaultDark,
    DefaultLight,
    WhiteSurDark,
    OrchisDark,
    CandySweet,
    Dracula,
    Nord,
    Monokai,
    SolarizedDark,
    SolarizedLight,
    GitHubDark,
    TokyoNight,
    Catppuccin,
    OneDark,
    GruvboxDark,
    GruvboxLight,
    AyuDark,
    AyuLight,
    AyuMirage,
    MaterialDark,
    MaterialOcean,
    Palenight,
    SynthWave84,
    Cyberpunk,
    RosePine,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::DefaultDark
    }
}

impl AppTheme {
    pub fn name_and_icon(&self) -> (&'static str, &'static str) {
        match self {
            Self::DefaultDark => ("Default Dark", "[D]"),
            Self::DefaultLight => ("Default Light", "[L]"),
            Self::WhiteSurDark => ("WhiteSur (macOS)", "[WS]"),
            Self::OrchisDark => ("Orchis Dark", "[OR]"),
            Self::CandySweet => ("Candy / Sweet", "[CS]"),
            Self::Dracula => ("Dracula", "[V]"),
            Self::Nord => ("Nordic", "[N]"),
            Self::Monokai => ("Monokai", "[M]"),
            Self::SolarizedDark => ("Solarized Dark", "[SD]"),
            Self::SolarizedLight => ("Solarized Light", "[SL]"),
            Self::GitHubDark => ("GitHub Dark", "[GH]"),
            Self::TokyoNight => ("Tokyo Night", "[TN]"),
            Self::Catppuccin => ("Catppuccin", "[C]"),
            Self::OneDark => ("One Dark", "[OD]"),
            Self::GruvboxDark => ("Gruvbox Dark", "[GD]"),
            Self::GruvboxLight => ("Gruvbox Light", "[GL]"),
            Self::AyuDark => ("Ayu Dark", "[AD]"),
            Self::AyuLight => ("Ayu Light", "[AL]"),
            Self::AyuMirage => ("Ayu Mirage", "[AM]"),
            Self::MaterialDark => ("Material Dark", "[MD]"),
            Self::MaterialOcean => ("Material Ocean", "[MO]"),
            Self::Palenight => ("Palenight", "[PN]"),
            Self::SynthWave84 => ("SynthWave '84", "[SW]"),
            Self::Cyberpunk => ("Cyberpunk", "[CP]"),
            Self::RosePine => ("Rosé Pine", "[RP]"),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub font_size: f32,
    pub blur_level: f32,
    pub window_rounding: f32,
    pub theme: AppTheme,
    pub sync_os_theme: bool,
    pub font_family: String,
    pub shell_program: String,
    pub cursor_style: String,
    pub auto_refocus: bool,
    pub tab_size: usize,
    pub show_line_numbers: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            blur_level: 0.15,
            window_rounding: 12.0,
            theme: AppTheme::DefaultDark,
            sync_os_theme: false,
            font_family: "System Default".to_string(),
            shell_program: "/bin/zsh".to_string(),
            cursor_style: "Block █".to_string(),
            auto_refocus: true,
            tab_size: 4,
            show_line_numbers: true,
        }
    }
}

pub struct SettingsApp;

impl WindowApp for SettingsApp {
    fn title(&self) -> String {
        format!("{} Settings", Icons::GEAR)
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: &mut AppConfig,
    ) -> Option<WindowAction> {
        let mut active_tab = ui.data(|d| d.get_temp::<usize>(egui::Id::new("settings_tab")).unwrap_or(0));
        let is_dark = config.theme != AppTheme::DefaultLight && config.theme != AppTheme::SolarizedLight;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("settings_scroll")
            .show(ui, |ui| {
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    // LEFT SIDEBAR TABS
                    ui.vertical(|ui| {
                        ui.set_width(140.0);

                        if ui.add(egui::SelectableLabel::new(active_tab == 0, "🎨 Appearance")).clicked() {
                            active_tab = 0;
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::SelectableLabel::new(active_tab == 1, "🎭 Themes & Icons")).clicked() {
                            active_tab = 1;
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::SelectableLabel::new(active_tab == 2, "🖥️ Terminal Engine")).clicked() {
                            active_tab = 2;
                        }
                        ui.add_space(4.0);
                        if ui.add(egui::SelectableLabel::new(active_tab == 3, "📝 Code Editor")).clicked() {
                            active_tab = 3;
                        }
                    });

                    ui.separator();
                    ui.add_space(8.0);

                    // RIGHT TAB CONTENT
                    ui.vertical(|ui| {
                        let card_frame = egui::Frame::default()
                            .fill(ctx.style().visuals.faint_bg_color)
                            .rounding(8.0)
                            .inner_margin(12.0)
                            .stroke(egui::Stroke::new(
                                1.0,
                                egui::Color32::from_gray(if is_dark { 40 } else { 220 }),
                            ));

                        match active_tab {
                            0 => {
                                ui.heading(egui::RichText::new("Appearance Customization").size(16.0).strong());
                                ui.add_space(10.0);

                                card_frame.show(ui, |ui| {
                                    egui::Grid::new("appearance_grid")
                                        .num_columns(2)
                                        .spacing([16.0, 12.0])
                                        .show(ui, |ui| {
                                            ui.label("Font Family:");
                                            egui::ComboBox::from_id_salt("font_family")
                                                .selected_text(&config.font_family)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut config.font_family, "System Default".to_string(), "System Default");
                                                    ui.selectable_value(&mut config.font_family, "JetBrains Mono".to_string(), "JetBrains Mono");
                                                    ui.selectable_value(&mut config.font_family, "Fira Code".to_string(), "Fira Code");
                                                });
                                            ui.end_row();

                                            ui.label("Font Size:");
                                            ui.add(egui::Slider::new(&mut config.font_size, 10.0..=32.0).suffix(" px"));
                                            ui.end_row();

                                            ui.label("Background Transparency:");
                                            ui.add(egui::Slider::new(&mut config.blur_level, 0.0..=0.9).show_value(false).text("Alpha"));
                                            ui.end_row();

                                            ui.label("Window Rounding:");
                                            ui.add(egui::Slider::new(&mut config.window_rounding, 0.0..=24.0).suffix(" px"));
                                            ui.end_row();
                                        });
                                });
                            }
                            1 => {
                                ui.heading(egui::RichText::new("Themes & Icon Packs (Awesome Linux Software Inspired)").size(16.0).strong());
                                ui.add_space(10.0);

                                card_frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Sync with System OS Theme");
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.checkbox(&mut config.sync_os_theme, "");
                                        });
                                    });
                                });

                                ui.add_space(10.0);

                                ui.add_enabled_ui(!config.sync_os_theme, |ui| {
                                    let themes = [
                                        AppTheme::DefaultDark,
                                        AppTheme::DefaultLight,
                                        AppTheme::WhiteSurDark,
                                        AppTheme::OrchisDark,
                                        AppTheme::CandySweet,
                                        AppTheme::Dracula,
                                        AppTheme::Nord,
                                        AppTheme::Monokai,
                                        AppTheme::TokyoNight,
                                        AppTheme::GitHubDark,
                                        AppTheme::Catppuccin,
                                        AppTheme::SolarizedDark,
                                        AppTheme::SolarizedLight,
                                        AppTheme::OneDark,
                                        AppTheme::GruvboxDark,
                                        AppTheme::GruvboxLight,
                                        AppTheme::AyuDark,
                                        AppTheme::AyuLight,
                                        AppTheme::AyuMirage,
                                        AppTheme::MaterialDark,
                                        AppTheme::MaterialOcean,
                                        AppTheme::Palenight,
                                        AppTheme::SynthWave84,
                                        AppTheme::Cyberpunk,
                                        AppTheme::RosePine,
                                    ];

                                    egui::ScrollArea::vertical()
                                        .id_salt("theme_list_scroll")
                                        .max_height(250.0)
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                                                for theme in themes {
                                                    let (name, icon) = theme.name_and_icon();
                                                    let is_selected = config.theme == theme;

                                                    let btn_fill = if is_selected {
                                                        ctx.style().visuals.selection.bg_fill
                                                    } else {
                                                        egui::Color32::TRANSPARENT
                                                    };

                                                    let text_color = if is_selected {
                                                        egui::Color32::WHITE
                                                    } else {
                                                        ctx.style().visuals.text_color()
                                                    };

                                                    let (rect, response) = ui.allocate_exact_size(
                                                        egui::vec2(ui.available_width(), 28.0),
                                                        egui::Sense::click(),
                                                    );

                                                    if ui.is_rect_visible(rect) {
                                                        if is_selected || response.hovered() {
                                                            let fill = if is_selected { btn_fill } else { ctx.style().visuals.faint_bg_color };
                                                            ui.painter().rect_filled(rect, 4.0, fill);
                                                        }
                                                        ui.painter().text(
                                                            rect.min + egui::vec2(8.0, 14.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            format!("{}  {}", icon, name),
                                                            egui::FontId::proportional(14.0),
                                                            text_color,
                                                        );
                                                    }

                                                    if response.clicked() {
                                                        config.theme = theme.clone();
                                                    }
                                                }
                                            });
                                        });
                                });
                            }
                            2 => {
                                ui.heading(egui::RichText::new("Terminal Engine Settings").size(16.0).strong());
                                ui.add_space(10.0);

                                card_frame.show(ui, |ui| {
                                    egui::Grid::new("terminal_grid")
                                        .num_columns(2)
                                        .spacing([16.0, 12.0])
                                        .show(ui, |ui| {
                                            ui.label("Default Shell Program:");
                                            egui::ComboBox::from_id_salt("shell_program")
                                                .selected_text(&config.shell_program)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut config.shell_program, "/bin/zsh".to_string(), "/bin/zsh");
                                                    ui.selectable_value(&mut config.shell_program, "/bin/bash".to_string(), "/bin/bash");
                                                    ui.selectable_value(&mut config.shell_program, "/bin/sh".to_string(), "/bin/sh");
                                                });
                                            ui.end_row();

                                            ui.label("Cursor Style:");
                                            egui::ComboBox::from_id_salt("cursor_style")
                                                .selected_text(&config.cursor_style)
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut config.cursor_style, "Block █".to_string(), "Block █");
                                                    ui.selectable_value(&mut config.cursor_style, "Underline _".to_string(), "Underline _");
                                                    ui.selectable_value(&mut config.cursor_style, "Bar |".to_string(), "Bar |");
                                                });
                                            ui.end_row();

                                            ui.label("Auto-refocus Command Line:");
                                            ui.checkbox(&mut config.auto_refocus, "Refocus after Enter");
                                            ui.end_row();
                                        });
                                });
                            }
                            3 => {
                                ui.heading(egui::RichText::new("Code Editor Customization").size(16.0).strong());
                                ui.add_space(10.0);

                                card_frame.show(ui, |ui| {
                                    egui::Grid::new("editor_grid")
                                        .num_columns(2)
                                        .spacing([16.0, 12.0])
                                        .show(ui, |ui| {
                                            ui.label("Tab Indent Size:");
                                            egui::ComboBox::from_id_salt("tab_size")
                                                .selected_text(format!("{} spaces", config.tab_size))
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut config.tab_size, 2, "2 spaces");
                                                    ui.selectable_value(&mut config.tab_size, 4, "4 spaces");
                                                    ui.selectable_value(&mut config.tab_size, 8, "8 spaces");
                                                });
                                            ui.end_row();

                                            ui.label("Show Line Numbers:");
                                            ui.checkbox(&mut config.show_line_numbers, "Enable");
                                            ui.end_row();
                                        });
                                });
                            }
                            _ => {}
                        }
                    });
                });
            });

        ui.data_mut(|d| d.insert_temp(egui::Id::new("settings_tab"), active_tab));
        None
    }
}
