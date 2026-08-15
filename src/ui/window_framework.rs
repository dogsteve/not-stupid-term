use eframe::egui;
use super::icons::Icons;
use super::undo_manager::UndoManager;

pub enum WindowAction {
    ConnectSsh(String),
    OpenSftp(String),
    OpenFile(String),
}

/// Generic trait for any application running inside a floating window
pub trait WindowApp {
    fn title(&self) -> String;
    fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
        undo: &mut UndoManager,
    ) -> Option<WindowAction>;

    fn min_size(&self) -> [f32; 2] {
        [200.0, 150.0]
    }

    fn default_size(&self) -> [f32; 2] {
        [700.0, 480.0]
    }

    fn window_type(&self) -> &'static str {
        "unknown"
    }

    fn save_state(&self) -> Option<serde_json::Value> {
        None
    }

    /// Returns a `&dyn Any` for downcasting to the concrete type.
    /// Override in types that need to be downcast (e.g. TerminalApp).
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }

    /// Returns a `&mut dyn Any` for mutable downcasting.
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        None
    }

    /// Send Ctrl+C (interrupt) to the app if it supports it.
    /// Default is a no-op; TerminalApp overrides this.
    fn interrupt(&mut self) {}

    /// Returns true if this app has a command actively running.
    fn is_running(&self) -> bool {
        false
    }

    /// Forcefully kill all processes owned by this app (e.g. PTY child processes).
    /// Called before the app window is closed. Default is a no-op.
    fn kill_all_processes(&mut self) {}

    /// Triggered when the window takes active user focus.
    fn on_focus(&mut self, _ctx: &egui::Context) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileTarget {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
}

impl TileTarget {
    pub fn compute_rect(&self, screen: egui::Rect) -> egui::Rect {
        let avail = egui::Rect::from_min_max(
            egui::pos2(screen.min.x, screen.min.y + 36.0),
            screen.max,
        );
        let w = avail.width();
        let h = avail.height();
        let min_x = avail.min.x;
        let min_y = avail.min.y;

        match self {
            TileTarget::LeftHalf => egui::Rect::from_min_max(avail.min, egui::pos2(min_x + w / 2.0, avail.max.y)),
            TileTarget::RightHalf => egui::Rect::from_min_max(egui::pos2(min_x + w / 2.0, min_y), avail.max),
            TileTarget::TopHalf => egui::Rect::from_min_max(avail.min, egui::pos2(avail.max.x, min_y + h / 2.0)),
            TileTarget::BottomHalf => egui::Rect::from_min_max(egui::pos2(min_x, min_y + h / 2.0), avail.max),
            TileTarget::TopLeft => egui::Rect::from_min_max(avail.min, egui::pos2(min_x + w / 2.0, min_y + h / 2.0)),
            TileTarget::TopRight => egui::Rect::from_min_max(egui::pos2(min_x + w / 2.0, min_y), egui::pos2(avail.max.x, min_y + h / 2.0)),
            TileTarget::BottomLeft => egui::Rect::from_min_max(egui::pos2(min_x, min_y + h / 2.0), egui::pos2(min_x + w / 2.0, avail.max.y)),
            TileTarget::BottomRight => egui::Rect::from_min_max(egui::pos2(min_x + w / 2.0, min_y + h / 2.0), avail.max),
            TileTarget::Maximize => avail,
        }
    }
}

pub struct FloatingWindow {
    pub id: String,
    pub is_open: bool,
    pub app: Box<dyn WindowApp>,
    pub custom_title: Option<String>,
    pub is_editing_title: bool,
    pub temp_title: String,
    pub focus_requested: bool,
    pub is_maximized: bool,
    pub restore_pos: Option<egui::Pos2>,
    pub restore_size: Option<egui::Vec2>,
    pub set_pos_size: Option<(egui::Pos2, egui::Vec2)>,
    pub pending_snap_target: Option<TileTarget>,
    pub last_known_rect: Option<egui::Rect>,
    /// Animation progress: 0.0 = just opened, 1.0 = fully visible.
    /// Lerps from 0 → 1 each frame while the window opens.
    open_anim: f32,
}

impl FloatingWindow {
    pub fn new(id: impl Into<String>, app: Box<dyn WindowApp>) -> Self {
        Self {
            id: id.into(),
            is_open: true,
            app,
            custom_title: None,
            is_editing_title: false,
            temp_title: String::new(),
            focus_requested: true,
            is_maximized: false,
            restore_pos: None,
            restore_size: None,
            set_pos_size: None,
            pending_snap_target: None,
            last_known_rect: None,
            open_anim: 0.0,
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        config: &mut crate::ui::settings::AppConfig,
        undo: &mut UndoManager,
    ) -> Option<WindowAction> {
        let mut is_open = self.is_open;
        let mut action = None;

        let display_title = self.custom_title.clone().unwrap_or_else(|| self.app.title());
        let min_s = self.app.min_size();
        let def_s = self.app.default_size();

        let is_dark = ctx.style().visuals.dark_mode;
        let window_fill = ctx.style().visuals.window_fill();

        // --- Animation: advance open_anim from 0 → 1 each frame while window is new ---
        let anim_speed = 14.0_f32; // frames to reach 1.0; feels instant but smooth
        let dt = ctx.input(|i| i.unstable_dt).min(0.1);
        let anim_enabled = config.animations_enabled;
        if anim_enabled && self.open_anim < 1.0 {
            self.open_anim = (self.open_anim + dt * anim_speed).min(1.0);
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // cap at ~60fps
        } else if !anim_enabled {
            self.open_anim = 1.0; // instant, no animation overhead
        }
        // Smooth ease-out: t = 1 - (1-t)^3
        let t = 1.0 - (1.0 - self.open_anim).powi(3);

        // Compute per-style alpha and vertical offset used below.
        let (alpha_f, y_offset) = match &config.animation_style {
            crate::ui::settings::WindowAnimationStyle::Fade => {
                (t, 0.0_f32)
            }
            crate::ui::settings::WindowAnimationStyle::Scale => {
                // Scale is applied via egui's transform but egui Window doesn’t support
                // arbitrary 2D transforms — we approximate with fade + tiny y nudge.
                (t, 0.0_f32)
            }
            crate::ui::settings::WindowAnimationStyle::SlideDown => {
                (t, -16.0 * (1.0 - t))
            }
            crate::ui::settings::WindowAnimationStyle::SlideUp => {
                (t, 16.0 * (1.0 - t))
            }
            crate::ui::settings::WindowAnimationStyle::Pop => {
                (t, -8.0 * (1.0 - t))
            }
        };

        // Apply per-frame scale for Scale/Pop styles via egui’s transform layer.
        let use_scale = anim_enabled && matches!(
            &config.animation_style,
            crate::ui::settings::WindowAnimationStyle::Scale
                | crate::ui::settings::WindowAnimationStyle::Pop
        );
        let scale = if use_scale {
            0.92 + 0.08 * t  // 92% → 100%
        } else {
            1.0
        };

        // Use the actual theme colors for consistency
        let title_bar_bg = if is_dark {
            egui::Color32::from_rgb(
                window_fill.r().saturating_add(8),
                window_fill.g().saturating_add(8),
                window_fill.b().saturating_add(8),
            )
        } else {
            egui::Color32::from_rgb(
                window_fill.r().saturating_sub(8),
                window_fill.g().saturating_sub(8),
                window_fill.b().saturating_sub(8),
            )
        };

        let border_color = if is_dark {
            egui::Color32::from_gray(50)
        } else {
            egui::Color32::from_gray(195)
        };

        let frame = egui::Frame::default()
            .fill(window_fill)
            .rounding(10.0)
            .inner_margin(0.0)
            .stroke(egui::Stroke::new(1.0, border_color))
            .shadow(egui::epaint::Shadow {
                offset: [0.0, 4.0].into(),
                blur: 20.0,
                spread: 0.0,
                color: egui::Color32::from_black_alpha(if is_dark { 140 } else { 50 }),
            });

        // Compute a stable cascade position so new windows spawn cleanly without frame-1 layout flicker
        let id_hash = self.id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let cascade_offset = ((id_hash % 8) as f32) * 26.0;
        let default_pos = egui::pos2(60.0 + cascade_offset, 50.0 + cascade_offset);

        let mut win_builder = egui::Window::new(&self.id)
            .id(egui::Id::new(&self.id))
            .title_bar(false)
            .frame(frame)
            .resizable(true)
            .min_size(min_s)
            .default_size(def_s)
            .default_pos(default_pos);

        if let Some((pos, size)) = self.set_pos_size.take() {
            win_builder = win_builder.fixed_pos(pos).fixed_size(size);
        }

        if self.focus_requested {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new(&self.id)));
            ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, egui::Id::new(&self.id)));
            self.app.on_focus(ctx);
            self.focus_requested = false;
        }

        let mut toggle_maximized = false;

        let win_layer = egui::LayerId::new(egui::Order::Middle, egui::Id::new(&self.id));

        // Apply y_offset / scale for open animations by shifting the window's layer transform.
        // Explicitly reset to IDENTITY when animation finishes to prevent dirty transform state.
        if anim_enabled && y_offset.abs() > 0.5 {
            let current_transform = egui::emath::TSTransform {
                translation: egui::vec2(0.0, y_offset),
                scaling: scale,
            };
            ctx.set_transform_layer(win_layer, current_transform);
        } else if anim_enabled && use_scale && scale < 0.999 {
            let current_transform = egui::emath::TSTransform {
                translation: egui::vec2(0.0, 0.0),
                scaling: scale,
            };
            ctx.set_transform_layer(win_layer, current_transform);
        } else {
            ctx.set_transform_layer(win_layer, egui::emath::TSTransform::IDENTITY);
        }

        let win_resp = win_builder.show(ctx, |ui| {
                // Apply fade opacity to the entire window content.
                if anim_enabled && alpha_f < 0.999 {
                    ui.multiply_opacity(alpha_f);
                }

                // === Title Bar ===
                let title_frame = egui::Frame::default()
                    .fill(title_bar_bg)
                    .inner_margin(egui::Margin {
                        left: 12.0,
                        right: 8.0,
                        top: 6.0,
                        bottom: 6.0,
                    })
                    .rounding(egui::Rounding {
                        nw: 10.0,
                        ne: 10.0,
                        sw: 0.0,
                        se: 0.0,
                    });

                let title_bar_resp = title_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if self.is_editing_title {
                            let edit_id = ui.id().with(("win_rename", &self.id));
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.temp_title)
                                    .id(edit_id)
                                    .desired_width(180.0)
                                    .font(egui::FontId::proportional(12.0)),
                            );
                            if !ui.memory(|m| m.has_focus(edit_id)) {
                                ui.memory_mut(|m| m.request_focus(edit_id));
                            }

                            let lost_focus = response.lost_focus();
                            let enter_esc = ui.input(|i| {
                                i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)
                            });
                            let clicked_outside = ui.input(|i| i.pointer.any_pressed()) && !response.hovered();

                            if lost_focus || enter_esc || clicked_outside {
                                self.is_editing_title = false;
                                if !self.temp_title.is_empty() {
                                    self.custom_title = Some(self.temp_title.clone());
                                } else {
                                    self.custom_title = None;
                                }
                            }
                        } else {
                            let title_color = if is_dark {
                                egui::Color32::from_gray(180)
                            } else {
                                egui::Color32::from_gray(70)
                            };

                            let mut chars = display_title.chars();
                            let title_response = if let Some(first_char) = chars.next() {
                                if (first_char as u32) >= 0xE000 && (first_char as u32) <= 0xF8FF {
                                    let rest: String = chars.collect();
                                    let job = Icons::label_job(&first_char.to_string(), rest.trim_start(), 12.0, title_color);
                                    ui.add(egui::Label::new(job).sense(egui::Sense::click()))
                                } else {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&display_title)
                                                .size(12.0)
                                                .color(title_color),
                                        )
                                        .sense(egui::Sense::click()),
                                    )
                                }
                            } else {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&display_title)
                                            .size(12.0)
                                            .color(title_color),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                            };

                            if title_response.double_clicked() {
                                toggle_maximized = true;
                            }
                        }

                        // Right-side window controls (Maximize & Close buttons)
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_color = if is_dark {
                                egui::Color32::from_gray(100)
                            } else {
                                egui::Color32::from_gray(150)
                            };

                            let close_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(Icons::CLOSE).size(11.0).color(btn_color),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .rounding(4.0)
                                .min_size(egui::vec2(20.0, 20.0)),
                            );
                            if close_btn.clicked() {
                                is_open = false;
                            }

                            let max_icon = if self.is_maximized { Icons::APP_WINDOW } else { Icons::SQUARE };
                            let max_btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(max_icon).size(11.0).color(btn_color),
                                )
                                .fill(egui::Color32::TRANSPARENT)
                                .rounding(4.0)
                                .min_size(egui::vec2(20.0, 20.0)),
                            );
                            if max_btn.clicked() {
                                toggle_maximized = true;
                            }
                        });
                    });
                }).response;

                if title_bar_resp.double_clicked() {
                    toggle_maximized = true;
                }

                // === Auto-Tiling Snap Preview & Edge Drag Detection ===
                // Only activate when the user is DRAGGING this window's title bar,
                // not on arbitrary clicks elsewhere (which would cause unwanted maximize).
                let title_being_dragged = title_bar_resp.dragged();

                if title_being_dragged {
                    if let Some(ptr) = ctx.pointer_latest_pos() {
                        let screen = ctx.screen_rect();
                        let avail_y_min = screen.min.y + 36.0;
                        let avail_height = (screen.height() - 36.0).max(1.0);
                        let avail_width = screen.width().max(1.0);

                        let rel_x = (ptr.x - screen.min.x) / avail_width;
                        let rel_y = (ptr.y - avail_y_min) / avail_height;

                        let edge_dist_left = ptr.x - screen.min.x;
                        let edge_dist_right = screen.max.x - ptr.x;
                        let edge_dist_top = ptr.y - screen.min.y;
                        let edge_dist_bottom = screen.max.y - ptr.y;

                        let target = if edge_dist_left <= 60.0 {
                            if rel_y < 0.35 {
                                Some(TileTarget::TopLeft)
                            } else if rel_y > 0.65 {
                                Some(TileTarget::BottomLeft)
                            } else {
                                Some(TileTarget::LeftHalf)
                            }
                        } else if edge_dist_right <= 60.0 {
                            if rel_y < 0.35 {
                                Some(TileTarget::TopRight)
                            } else if rel_y > 0.65 {
                                Some(TileTarget::BottomRight)
                            } else {
                                Some(TileTarget::RightHalf)
                            }
                        } else if edge_dist_top <= 55.0 {
                            if rel_x < 0.35 {
                                Some(TileTarget::TopLeft)
                            } else if rel_x > 0.65 {
                                Some(TileTarget::TopRight)
                            } else {
                                Some(TileTarget::Maximize)
                            }
                        } else if edge_dist_bottom <= 60.0 {
                            if rel_x < 0.35 {
                                Some(TileTarget::BottomLeft)
                            } else if rel_x > 0.65 {
                                Some(TileTarget::BottomRight)
                            } else {
                                Some(TileTarget::BottomHalf)
                            }
                        } else {
                            None
                        };

                        if let Some(t) = target {
                            self.pending_snap_target = Some(t);
                            let tile_rect = t.compute_rect(screen);
                            ctx.layer_painter(egui::LayerId::debug()).rect_filled(
                                tile_rect,
                                8.0,
                                egui::Color32::from_rgba_unmultiplied(60, 140, 240, 60),
                            );
                            ctx.layer_painter(egui::LayerId::debug()).rect_stroke(
                                tile_rect,
                                8.0,
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 170, 255)),
                            );
                            ctx.request_repaint();
                        } else {
                            self.pending_snap_target = None;
                        }
                    }
                } else if !ctx.input(|i| i.pointer.primary_down()) {
                    // Clear pending snap when mouse is not down and title bar is not dragged
                    self.pending_snap_target = None;
                }

                if title_bar_resp.drag_stopped() {
                    if let Some(t) = self.pending_snap_target.take() {
                        let screen = ctx.screen_rect();
                        let tile_rect = t.compute_rect(screen);
                        self.set_pos_size = Some((tile_rect.min, tile_rect.size()));
                        self.is_maximized = (t == TileTarget::Maximize);
                    }
                }

                // === Content Area ===
                let content_frame = egui::Frame::default()
                    .inner_margin(egui::Margin::same(4.0));

                content_frame.show(ui, |ui| {
                    action = self.app.render(ui, ctx, config, undo);
                });
            });

        if let Some(inner) = win_resp {
            let rect = inner.response.rect;
            if !self.is_maximized && rect.width() > 100.0 && rect.height() > 80.0 {
                self.last_known_rect = Some(rect);
                self.restore_pos = Some(rect.min);
                self.restore_size = Some(rect.size());
            }
        }

        if toggle_maximized {
            let screen = ctx.screen_rect();
            if self.is_maximized {
                if let (Some(pos), Some(size)) = (self.restore_pos, self.restore_size) {
                    self.set_pos_size = Some((pos, size));
                } else if let Some(last_rect) = self.last_known_rect {
                    self.set_pos_size = Some((last_rect.min, last_rect.size()));
                } else {
                    let def_size = egui::vec2(700.0, 480.0);
                    let def_pos = egui::pos2(screen.min.x + 50.0, screen.min.y + 50.0);
                    self.set_pos_size = Some((def_pos, def_size));
                }
                self.is_maximized = false;
            } else {
                let max_rect = TileTarget::Maximize.compute_rect(screen);
                self.set_pos_size = Some((max_rect.min, max_rect.size()));
                self.is_maximized = true;
            }
        }

        self.is_open = is_open;

        // Reset layer transform to identity once animation is complete.
        if anim_enabled && self.open_anim >= 1.0 && (use_scale || y_offset.abs() > 0.5) {
            ctx.set_transform_layer(win_layer, egui::emath::TSTransform::IDENTITY);
        }

        action
    }
}
