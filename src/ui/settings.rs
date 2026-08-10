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
    Everforest,
    Kanagawa,
    Nightfox,
    Moonlight,
    VitesseDark,
    Horizon,
    Poimandres,
    BlulocoDark,
    ChallengerDeep,
    SnazzyLight,
    WinterIsComing,
    Vesper,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::DefaultDark
    }
}

impl AppTheme {
    pub fn name_and_icon(&self) -> (&'static str, &'static str) {
        match self {
            Self::DefaultDark => ("Default Dark", Icons::MOON),
            Self::DefaultLight => ("Default Light", Icons::SUN),
            Self::WhiteSurDark => ("WhiteSur (macOS)", Icons::DESKTOP),
            Self::OrchisDark => ("Orchis Dark", Icons::PALETTE),
            Self::CandySweet => ("Candy / Sweet", Icons::HEART),
            Self::Dracula => ("Dracula", Icons::MOON),
            Self::Nord => ("Nordic", Icons::MOON),
            Self::Monokai => ("Monokai", Icons::LIGHTNING),
            Self::SolarizedDark => ("Solarized Dark", Icons::MOON),
            Self::SolarizedLight => ("Solarized Light", Icons::SUN),
            Self::GitHubDark => ("GitHub Dark", Icons::GIT_BRANCH),
            Self::TokyoNight => ("Tokyo Night", Icons::STAR),
            Self::Catppuccin => ("Catppuccin", Icons::HEART),
            Self::OneDark => ("One Dark", Icons::MOON),
            Self::GruvboxDark => ("Gruvbox Dark", Icons::MOON),
            Self::GruvboxLight => ("Gruvbox Light", Icons::SUN),
            Self::AyuDark => ("Ayu Dark", Icons::MOON),
            Self::AyuLight => ("Ayu Light", Icons::SUN),
            Self::AyuMirage => ("Ayu Mirage", Icons::MOON),
            Self::MaterialDark => ("Material Dark", Icons::PALETTE),
            Self::MaterialOcean => ("Material Ocean", Icons::MOON),
            Self::Palenight => ("Palenight", Icons::STAR),
            Self::SynthWave84 => ("SynthWave '84", Icons::LIGHTNING),
            Self::Cyberpunk => ("Cyberpunk", Icons::LIGHTNING),
            Self::RosePine => ("Rosé Pine", Icons::HEART),
            Self::Everforest => ("Everforest", Icons::MOON),
            Self::Kanagawa => ("Kanagawa", Icons::MOON),
            Self::Nightfox => ("Nightfox", Icons::MOON),
            Self::Moonlight => ("Moonlight", Icons::STAR),
            Self::VitesseDark => ("Vitesse Dark", Icons::MOON),
            Self::Horizon => ("Horizon", Icons::SUN),
            Self::Poimandres => ("Poimandres", Icons::STAR),
            Self::BlulocoDark => ("Bluloco Dark", Icons::MOON),
            Self::ChallengerDeep => ("Challenger Deep", Icons::STAR),
            Self::SnazzyLight => ("Snazzy Light", Icons::SUN),
            Self::WinterIsComing => ("Winter Is Coming", Icons::MOON),
            Self::Vesper => ("Vesper", Icons::MOON),
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
    pub mcp_port: u16,
    pub mcp_enabled: bool,
    pub mcp_auto_start: bool,
    pub mcp_api_key: String,
    // New settings
    pub cursor_blink: bool,
    pub scroll_sensitivity: f32,
    pub copy_on_select: bool,
    pub bell_enabled: bool,
    pub scrollback_lines: usize,
    pub open_links_on_click: bool,
    pub bold_is_bright: bool,
    pub ligatures_enabled: bool,
    pub show_tab_close_button: bool,
    pub confirm_on_close: bool,
    pub natural_scrolling: bool,
    pub gpu_acceleration: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            blur_level: 0.15,
            window_rounding: 12.0,
            theme: AppTheme::DefaultDark,
            sync_os_theme: false,
            font_family: "Fira Code".to_string(),
            shell_program: "/bin/zsh".to_string(),
            cursor_style: "Block".to_string(),
            auto_refocus: true,
            tab_size: 4,
            show_line_numbers: true,
            mcp_port: 3000,
            mcp_enabled: true,
            mcp_auto_start: true,
            mcp_api_key: String::new(),
            cursor_blink: true,
            scroll_sensitivity: 3.0,
            copy_on_select: false,
            bell_enabled: true,
            scrollback_lines: 10000,
            open_links_on_click: true,
            bold_is_bright: false,
            ligatures_enabled: true,
            show_tab_close_button: true,
            confirm_on_close: true,
            natural_scrolling: true,
            gpu_acceleration: true,
        }
    }
}

pub struct SettingsApp;

impl WindowApp for SettingsApp {
    fn title(&self) -> String {
        format!("{} Settings", Icons::GEAR)
    }

    fn window_type(&self) -> &'static str {
        "settings"
    }

    fn default_size(&self) -> [f32; 2] {
        [900.0, 620.0]
    }

    fn min_size(&self) -> [f32; 2] {
        [600.0, 400.0]
    }

    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: &mut AppConfig,
    ) -> Option<WindowAction> {
        let mut active_tab = ui.data(|d| d.get_temp::<usize>(egui::Id::new("settings_tab")).unwrap_or(0));
        let is_dark = ctx.style().visuals.dark_mode;

        // Fill available space
        let available = ui.available_size();

        ui.horizontal(|ui| {
            // LEFT SIDEBAR
            ui.vertical(|ui| {
                ui.set_width(160.0);
                ui.set_min_height(available.y - 16.0);
                ui.add_space(4.0);

                let tabs = [
                    (Icons::PALETTE, "Appearance"),
                    (Icons::MOON, "Themes"),
                    (Icons::TERMINAL, "Terminal"),
                    (Icons::EDIT, "Editor"),
                    (Icons::SERVER, "MCP Server"),
                    (Icons::GEAR, "Advanced"),
                ];

                for (i, (icon, label)) in tabs.iter().enumerate() {
                    let selected = active_tab == i;
                    let text = format!("  {}  {}", icon, label);

                    let text_color = if selected {
                        ctx.style().visuals.text_color()
                    } else if is_dark {
                        egui::Color32::from_gray(140)
                    } else {
                        egui::Color32::from_gray(100)
                    };

                    let resp = ui.add(
                        egui::Button::new(egui::RichText::new(&text).size(13.0).color(text_color))
                            .fill(if selected {
                                if is_dark { egui::Color32::from_white_alpha(12) } else { egui::Color32::from_black_alpha(8) }
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .stroke(egui::Stroke::NONE)
                            .rounding(6.0)
                            .min_size(egui::vec2(152.0, 32.0))
                    );

                    if selected {
                        let r = resp.rect;
                        let accent = ctx.style().visuals.selection.bg_fill;
                        ui.painter().vline(r.left() + 1.0, r.y_range(), egui::Stroke::new(2.0, accent));
                    }

                    if resp.clicked() {
                        active_tab = i;
                    }
                    ui.add_space(2.0);
                }
            });

            // Vertical separator
            let sep_color = if is_dark { egui::Color32::from_white_alpha(12) } else { egui::Color32::from_black_alpha(10) };
            let sep_rect = ui.available_rect_before_wrap();
            ui.painter().vline(sep_rect.left(), sep_rect.y_range(), egui::Stroke::new(0.5, sep_color));
            ui.add_space(8.0);

            // RIGHT CONTENT — fill remaining width
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width() - 8.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .id_salt("settings_content_scroll")
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width() - 16.0);
                        ui.add_space(4.0);

                        match active_tab {
                            0 => self.render_appearance(ui, ctx, config, is_dark),
                            1 => self.render_themes(ui, ctx, config, is_dark),
                            2 => self.render_terminal(ui, ctx, config, is_dark),
                            3 => self.render_editor(ui, ctx, config, is_dark),
                            4 => self.render_mcp(ui, ctx, config, is_dark),
                            5 => self.render_advanced(ui, ctx, config, is_dark),
                            _ => {}
                        }

                        ui.add_space(16.0);
                    });
            });
        });

        ui.data_mut(|d| d.insert_temp(egui::Id::new("settings_tab"), active_tab));
        None
    }
}

impl SettingsApp {
    fn section_heading(ui: &mut egui::Ui, title: &str) {
        ui.heading(egui::RichText::new(title).size(16.0).strong());
        ui.add_space(10.0);
    }

    fn card_frame(ctx: &egui::Context, is_dark: bool) -> egui::Frame {
        egui::Frame::default()
            .fill(ctx.style().visuals.faint_bg_color)
            .rounding(8.0)
            .inner_margin(14.0)
            .stroke(egui::Stroke::new(0.5, if is_dark {
                egui::Color32::from_white_alpha(12)
            } else {
                egui::Color32::from_black_alpha(10)
            }))
    }

    fn setting_row(ui: &mut egui::Ui, label: &str) {
        ui.label(egui::RichText::new(label).size(13.0));
    }

    fn render_appearance(&self, ui: &mut egui::Ui, ctx: &egui::Context, config: &mut AppConfig, is_dark: bool) {
        Self::section_heading(ui, "Appearance");

        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            egui::Grid::new("appearance_grid")
                .num_columns(2)
                .spacing([20.0, 14.0])
                .min_col_width(140.0)
                .show(ui, |ui| {
                    Self::setting_row(ui, "Font Family");
                    egui::ComboBox::from_id_salt("font_family")
                        .selected_text(&config.font_family)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            for font in ["System Default", "Fira Code", "JetBrains Mono", "Menlo", "Monaco",
                                         "SF Mono", "Cascadia Code", "Inconsolata", "Source Code Pro",
                                         "IBM Plex Mono", "Hack", "Consolas"] {
                                ui.selectable_value(&mut config.font_family, font.to_string(), font);
                            }
                        });
                    ui.end_row();

                    Self::setting_row(ui, "Font Size");
                    ui.horizontal(|ui| {
                        ui.add(egui::Slider::new(&mut config.font_size, 8.0..=36.0).suffix(" px").step_by(1.0));
                        if ui.small_button(Icons::REFRESH).on_hover_text("Reset to 12px").clicked() {
                            config.font_size = 12.0;
                        }
                    });
                    ui.end_row();

                    Self::setting_row(ui, "Font Ligatures");
                    ui.checkbox(&mut config.ligatures_enabled, "Enable ligatures (=> != ===)");
                    ui.end_row();

                    Self::setting_row(ui, "Window Rounding");
                    ui.add(egui::Slider::new(&mut config.window_rounding, 0.0..=24.0).suffix(" px"));
                    ui.end_row();

                    Self::setting_row(ui, "Background Opacity");
                    ui.add(egui::Slider::new(&mut config.blur_level, 0.0..=0.9).show_value(true));
                    ui.end_row();
                });
        });

        ui.add_space(12.0);

        // Live preview
        ui.label(egui::RichText::new("Live Preview").size(12.0).color(
            if is_dark { egui::Color32::from_gray(100) } else { egui::Color32::from_gray(160) }
        ));
        ui.add_space(4.0);
        let preview_bg = if is_dark { egui::Color32::from_gray(16) } else { egui::Color32::from_gray(248) };
        egui::Frame::default().fill(preview_bg).rounding(6.0).inner_margin(12.0).show(ui, |ui| {
            ui.label(egui::RichText::new("~ > echo \"Hello, Smart Terminal!\"")
                .family(egui::FontFamily::Monospace).size(config.font_size));
            ui.label(egui::RichText::new("Hello, Smart Terminal!")
                .family(egui::FontFamily::Monospace).size(config.font_size));
            ui.label(egui::RichText::new("0O 1lI {} [] () <> => != ++ -- === !==")
                .family(egui::FontFamily::Monospace).size(config.font_size)
                .color(if is_dark { egui::Color32::from_gray(80) } else { egui::Color32::from_gray(170) }));
        });
    }

    fn render_themes(&self, ui: &mut egui::Ui, ctx: &egui::Context, config: &mut AppConfig, is_dark: bool) {
        Self::section_heading(ui, "Themes");

        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                Self::setting_row(ui, "Sync with OS Theme");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut config.sync_os_theme, "");
                });
            });
        });

        ui.add_space(10.0);

        ui.add_enabled_ui(!config.sync_os_theme, |ui| {
            let themes = [
                AppTheme::DefaultDark, AppTheme::DefaultLight,
                AppTheme::Dracula, AppTheme::Nord, AppTheme::Monokai,
                AppTheme::TokyoNight, AppTheme::GitHubDark, AppTheme::Catppuccin,
                AppTheme::OneDark, AppTheme::RosePine,
                AppTheme::SolarizedDark, AppTheme::SolarizedLight,
                AppTheme::GruvboxDark, AppTheme::GruvboxLight,
                AppTheme::AyuDark, AppTheme::AyuLight, AppTheme::AyuMirage,
                AppTheme::MaterialDark, AppTheme::MaterialOcean, AppTheme::Palenight,
                AppTheme::WhiteSurDark, AppTheme::OrchisDark, AppTheme::CandySweet,
                AppTheme::SynthWave84, AppTheme::Cyberpunk,
                AppTheme::Everforest, AppTheme::Kanagawa, AppTheme::Nightfox,
                AppTheme::Moonlight, AppTheme::VitesseDark, AppTheme::Horizon,
                AppTheme::Poimandres, AppTheme::BlulocoDark,
                AppTheme::ChallengerDeep, AppTheme::SnazzyLight,
                AppTheme::WinterIsComing, AppTheme::Vesper,
            ];

            egui::ScrollArea::vertical()
                .id_salt("theme_list_scroll")
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 20.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 3.0);
                    for theme in themes {
                        let (name, icon) = theme.name_and_icon();
                        let is_selected = config.theme == theme;

                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 30.0),
                            egui::Sense::click(),
                        );

                        if ui.is_rect_visible(rect) {
                            if is_selected {
                                ui.painter().rect_filled(rect, 6.0, ctx.style().visuals.selection.bg_fill);
                            } else if response.hovered() {
                                let hover = if is_dark {
                                    egui::Color32::from_white_alpha(10)
                                } else {
                                    egui::Color32::from_black_alpha(8)
                                };
                                ui.painter().rect_filled(rect, 6.0, hover);
                            }

                            let text_color = if is_selected {
                                egui::Color32::WHITE
                            } else {
                                ctx.style().visuals.text_color()
                            };
                            ui.painter().text(
                                rect.min + egui::vec2(10.0, 15.0),
                                egui::Align2::LEFT_CENTER,
                                format!("{}  {}", icon, name),
                                egui::FontId::proportional(13.0),
                                text_color,
                            );
                        }

                        if response.clicked() {
                            config.theme = theme.clone();
                        }
                    }
                });
        });
    }

    fn render_terminal(&self, ui: &mut egui::Ui, ctx: &egui::Context, config: &mut AppConfig, is_dark: bool) {
        Self::section_heading(ui, "Terminal");

        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            egui::Grid::new("terminal_grid")
                .num_columns(2)
                .spacing([20.0, 14.0])
                .min_col_width(160.0)
                .show(ui, |ui| {
                    Self::setting_row(ui, "Default Shell");
                    egui::ComboBox::from_id_salt("shell_program")
                        .selected_text(&config.shell_program)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for shell in ["/bin/zsh", "/bin/bash", "/bin/sh", "/bin/fish", "/usr/local/bin/fish"] {
                                ui.selectable_value(&mut config.shell_program, shell.to_string(), shell);
                            }
                        });
                    ui.end_row();

                    Self::setting_row(ui, "Cursor Style");
                    egui::ComboBox::from_id_salt("cursor_style")
                        .selected_text(&config.cursor_style)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for style in ["Block", "Underline", "Bar"] {
                                ui.selectable_value(&mut config.cursor_style, style.to_string(), style);
                            }
                        });
                    ui.end_row();

                    Self::setting_row(ui, "Cursor Blink");
                    ui.checkbox(&mut config.cursor_blink, "Enable blinking cursor");
                    ui.end_row();

                    Self::setting_row(ui, "Scrollback Lines");
                    ui.add(egui::Slider::new(&mut config.scrollback_lines, 1000..=100000).logarithmic(true));
                    ui.end_row();

                    Self::setting_row(ui, "Scroll Sensitivity");
                    ui.add(egui::Slider::new(&mut config.scroll_sensitivity, 1.0..=10.0).step_by(0.5));
                    ui.end_row();

                    Self::setting_row(ui, "Natural Scrolling");
                    ui.checkbox(&mut config.natural_scrolling, "Reverse scroll direction");
                    ui.end_row();

                    Self::setting_row(ui, "Bell Sound");
                    ui.checkbox(&mut config.bell_enabled, "Enable terminal bell");
                    ui.end_row();

                    Self::setting_row(ui, "Bold is Bright");
                    ui.checkbox(&mut config.bold_is_bright, "Use bright colors for bold text");
                    ui.end_row();

                    Self::setting_row(ui, "Copy on Select");
                    ui.checkbox(&mut config.copy_on_select, "Auto-copy selected text");
                    ui.end_row();

                    Self::setting_row(ui, "Clickable Links");
                    ui.checkbox(&mut config.open_links_on_click, "Open URLs on click");
                    ui.end_row();

                    Self::setting_row(ui, "Auto-refocus Input");
                    ui.checkbox(&mut config.auto_refocus, "Refocus after running command");
                    ui.end_row();
                });
        });
    }

    fn render_editor(&self, ui: &mut egui::Ui, ctx: &egui::Context, config: &mut AppConfig, is_dark: bool) {
        Self::section_heading(ui, "Code Editor");

        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            egui::Grid::new("editor_grid")
                .num_columns(2)
                .spacing([20.0, 14.0])
                .min_col_width(160.0)
                .show(ui, |ui| {
                    Self::setting_row(ui, "Tab Size");
                    egui::ComboBox::from_id_salt("tab_size")
                        .selected_text(format!("{} spaces", config.tab_size))
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for size in [2, 4, 8] {
                                ui.selectable_value(&mut config.tab_size, size, format!("{} spaces", size));
                            }
                        });
                    ui.end_row();

                    Self::setting_row(ui, "Line Numbers");
                    ui.checkbox(&mut config.show_line_numbers, "Show line numbers");
                    ui.end_row();
                });
        });
    }

    fn render_mcp(&self, ui: &mut egui::Ui, ctx: &egui::Context, config: &mut AppConfig, is_dark: bool) {
        Self::section_heading(ui, "MCP Server");

        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            egui::Grid::new("mcp_grid")
                .num_columns(2)
                .spacing([20.0, 14.0])
                .min_col_width(160.0)
                .show(ui, |ui| {
                    Self::setting_row(ui, "Server Enabled");
                    ui.checkbox(&mut config.mcp_enabled, "Enable MCP server");
                    ui.end_row();

                    Self::setting_row(ui, "Auto-start");
                    ui.checkbox(&mut config.mcp_auto_start, "Start on app launch");
                    ui.end_row();

                    Self::setting_row(ui, "Server Port");
                    ui.add(egui::Slider::new(&mut config.mcp_port, 1024..=65535));
                    ui.end_row();

                    Self::setting_row(ui, "API Key");
                    ui.add(egui::TextEdit::singleline(&mut config.mcp_api_key).password(true).desired_width(200.0));
                    ui.end_row();
                });
        });
    }

    fn render_advanced(&self, ui: &mut egui::Ui, ctx: &egui::Context, config: &mut AppConfig, is_dark: bool) {
        Self::section_heading(ui, "Advanced");

        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            egui::Grid::new("advanced_grid")
                .num_columns(2)
                .spacing([20.0, 14.0])
                .min_col_width(160.0)
                .show(ui, |ui| {
                    Self::setting_row(ui, "GPU Acceleration");
                    ui.checkbox(&mut config.gpu_acceleration, "Use hardware rendering");
                    ui.end_row();

                    Self::setting_row(ui, "Tab Close Button");
                    ui.checkbox(&mut config.show_tab_close_button, "Show close button on tabs");
                    ui.end_row();

                    Self::setting_row(ui, "Confirm on Close");
                    ui.checkbox(&mut config.confirm_on_close, "Warn before closing with running process");
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        // Reset & About section
        Self::card_frame(ctx, is_dark).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button(format!("{} Reset All Settings", Icons::REFRESH)).clicked() {
                    *config = AppConfig::default();
                }
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("Smart Terminal v0.1.0  •  Phosphor Icons  •  egui")
                        .size(11.0)
                        .color(if is_dark { egui::Color32::from_gray(70) } else { egui::Color32::from_gray(160) })
                );
            });
        });
    }
}
