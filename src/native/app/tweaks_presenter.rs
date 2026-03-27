use super::super::background::BackgroundResult;
use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::super::installed_theme_packs;
use super::super::desktop_settings_service::persist_settings_draft;
use super::super::desktop_status_service::saved_settings_status;
use super::super::desktop_surface_service::{
    desktop_builtin_icons, set_builtin_icon_visible, set_desktop_icon_style,
    set_wallpaper_size_mode as set_desktop_wallpaper_size_mode, wallpaper_browser_start_dir,
};
use super::super::retro_ui::current_palette;
use super::desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowRectTracking};
use super::RobcoNativeApp;
use crate::config::{
    CliAcsMode, CliColorMode, DesktopIconStyle, NativeStartupWindowMode, WallpaperSizeMode,
    CUSTOM_THEME_NAME, THEMES,
};
use crate::theme::{ColorStyle, LayoutProfile, MonochromePreset, TerminalLayoutProfile, ThemePack};
use eframe::egui::{self, Context, RichText, TextEdit};

fn theme_name_for_color_style(style: &ColorStyle) -> &'static str {
    match style {
        ColorStyle::Monochrome { preset, .. } => match preset {
            MonochromePreset::Green => "Green (Default)",
            MonochromePreset::White => "White",
            MonochromePreset::Amber => "Amber",
            MonochromePreset::Blue => "Blue",
            MonochromePreset::LightBlue => "Light Blue",
            MonochromePreset::Custom => CUSTOM_THEME_NAME,
        },
        ColorStyle::FullColor { .. } => "Green (Default)",
    }
}

fn custom_rgb_for_color_style(style: &ColorStyle) -> [u8; 3] {
    match style {
        ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb,
        } => custom_rgb.unwrap_or([0, 255, 0]),
        _ => [0, 255, 0],
    }
}

fn color_style_from_theme_name(name: &str, custom_rgb: [u8; 3]) -> ColorStyle {
    match name {
        "Green (Default)" => ColorStyle::Monochrome {
            preset: MonochromePreset::Green,
            custom_rgb: None,
        },
        "White" => ColorStyle::Monochrome {
            preset: MonochromePreset::White,
            custom_rgb: None,
        },
        "Amber" => ColorStyle::Monochrome {
            preset: MonochromePreset::Amber,
            custom_rgb: None,
        },
        "Blue" => ColorStyle::Monochrome {
            preset: MonochromePreset::Blue,
            custom_rgb: None,
        },
        "Light Blue" => ColorStyle::Monochrome {
            preset: MonochromePreset::LightBlue,
            custom_rgb: None,
        },
        CUSTOM_THEME_NAME => ColorStyle::Monochrome {
            preset: MonochromePreset::Custom,
            custom_rgb: Some(custom_rgb),
        },
        _ => ColorStyle::Monochrome {
            preset: MonochromePreset::Green,
            custom_rgb: None,
        },
    }
}

fn selected_theme_pack_name(selected_id: Option<&str>, theme_packs: &[ThemePack]) -> String {
    selected_id
        .and_then(|id| theme_packs.iter().find(|theme| theme.id == id))
        .map(|theme| theme.name.clone())
        .unwrap_or_else(|| "Manual".to_string())
}

impl RobcoNativeApp {
    pub(super) fn open_tweaks_from_settings(&mut self) {
        self.tweaks_open = true;
        self.prime_desktop_window_defaults(DesktopWindow::Tweaks);
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Tweaks));
    }

    pub(super) fn draw_tweaks(&mut self, ctx: &Context) {
        if !self.tweaks_open || self.desktop_window_is_minimized(DesktopWindow::Tweaks) {
            return;
        }
        let wid = self.current_window_id(DesktopWindow::Tweaks);
        let mut open = self.tweaks_open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Tweaks);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Tweaks);
        let mut header_action = DesktopHeaderAction::None;
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Tweaks);
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let mut window = egui::Window::new("Tweaks")
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(false)
            .default_pos(default_pos)
            .fixed_size(default_size);
        if maximized {
            let rect = self.active_desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, _size)) = restore {
            let pos = self.active_desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos);
        }
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(ui, "Tweaks", maximized);
            let mut persist_changed = false;
            let mut window_mode_changed = false;
            let mut desktop_runtime_changed = false;
            let palette = current_palette();
            let theme_packs = installed_theme_packs();

            ui.add_space(4.0);
            ui.label(RichText::new("Tweaks").strong().size(28.0));
            ui.add_space(14.0);

            let body_max_height = ui.available_height().max(120.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(body_max_height)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, label) in ["Desktop", "Terminal"].iter().enumerate() {
                            let active = self.tweaks_surface_tab == i as u8;
                            let color = if active { palette.fg } else { palette.dim };
                            let btn = ui.add(
                                egui::Button::new(RichText::new(*label).color(color).strong())
                                .stroke(egui::Stroke::new(
                                    if active { 2.0 } else { 1.0 },
                                    color,
                                ))
                                .fill(if active { palette.panel } else { palette.bg }),
                            );
                            if btn.clicked() {
                                self.tweaks_surface_tab = i as u8;
                            }
                        }
                    });
                    ui.add_space(10.0);
                    Self::retro_separator(ui);
                    ui.add_space(8.0);
                    match self.tweaks_surface_tab {
                        0 => {
                            ui.horizontal(|ui| {
                                for (i, label) in
                                    ["Background", "Display", "Colors", "Icons", "Layout"]
                                        .iter()
                                        .enumerate()
                                {
                                    let active = self.desktop_tweaks_tab == i as u8;
                                    let color = if active { palette.fg } else { palette.dim };
                                    let btn = ui.add(
                                        egui::Button::new(
                                            RichText::new(*label).color(color).strong(),
                                        )
                                        .stroke(egui::Stroke::new(
                                            if active { 2.0 } else { 1.0 },
                                            color,
                                        ))
                                        .fill(if active { palette.panel } else { palette.bg }),
                                    );
                                    if btn.clicked() {
                                        self.desktop_tweaks_tab = i as u8;
                                    }
                                }
                            });
                            ui.add_space(10.0);
                            Self::retro_separator(ui);
                            ui.add_space(8.0);
                            match self.desktop_tweaks_tab {
                                0 => {
                                    Self::settings_section(ui, "Wallpaper", |ui| {
                                        ui.label("Wallpaper Path");
                                        ui.horizontal(|ui| {
                                            let w =
                                                Self::responsive_input_width(ui, 0.72, 160.0, 400.0);
                                            if ui
                                                .add(
                                                    TextEdit::singleline(
                                                        &mut self.settings.draft.desktop_wallpaper,
                                                    )
                                                    .desired_width(w)
                                                    .hint_text("/path/to/image.png"),
                                                )
                                                .changed()
                                            {
                                                persist_changed = true;
                                            }
                                            if ui.button("Browse…").clicked() {
                                                let start = wallpaper_browser_start_dir(
                                                    &self.settings.draft.desktop_wallpaper,
                                                );
                                                self.picking_wallpaper = true;
                                                self.open_embedded_file_manager_at(start);
                                            }
                                        });
                                        ui.add_space(8.0);
                                        ui.horizontal(|ui| {
                                            ui.label("Wallpaper Mode");
                                            let selected = match self
                                                .settings
                                                .draft
                                                .desktop_wallpaper_size_mode
                                            {
                                                WallpaperSizeMode::DefaultSize => "Default Size",
                                                WallpaperSizeMode::FitToScreen => "Fit To Screen",
                                                WallpaperSizeMode::Centered => "Centered",
                                                WallpaperSizeMode::Tile => "Tile",
                                                WallpaperSizeMode::Stretch => "Stretch",
                                            };
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_wallpaper_mode",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for (mode, label) in [
                                                    (
                                                        WallpaperSizeMode::DefaultSize,
                                                        "Default Size",
                                                    ),
                                                    (
                                                        WallpaperSizeMode::FitToScreen,
                                                        "Fit To Screen",
                                                    ),
                                                    (WallpaperSizeMode::Centered, "Centered"),
                                                    (WallpaperSizeMode::Tile, "Tile"),
                                                    (WallpaperSizeMode::Stretch, "Stretch"),
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        label,
                                                        self.settings
                                                            .draft
                                                            .desktop_wallpaper_size_mode
                                                            == mode,
                                                    )
                                                    .clicked()
                                                    {
                                                        set_desktop_wallpaper_size_mode(
                                                            &mut self.settings.draft,
                                                            mode,
                                                        );
                                                        persist_changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                    });
                                }
                                1 => {
                                    Self::settings_section(ui, "Window", |ui| {
                                        ui.label("Window Mode");
                                        ui.horizontal_wrapped(|ui| {
                                            for mode in [
                                                NativeStartupWindowMode::Windowed,
                                                NativeStartupWindowMode::Maximized,
                                                NativeStartupWindowMode::BorderlessFullscreen,
                                                NativeStartupWindowMode::Fullscreen,
                                            ] {
                                                if Self::retro_choice_button(
                                                    ui,
                                                    mode.label(),
                                                    self.settings.draft.native_startup_window_mode
                                                        == mode,
                                                )
                                                .clicked()
                                                    && self.settings.draft.native_startup_window_mode
                                                        != mode
                                                {
                                                    self.settings.draft.native_startup_window_mode =
                                                        mode;
                                                    persist_changed = true;
                                                    window_mode_changed = true;
                                                }
                                            }
                                        });
                                        ui.add_space(8.0);
                                        ui.small(
                                            "Applies immediately and persists across launches. Windowed is the safest mode on older GPUs.",
                                        );
                                    });
                                    ui.add_space(10.0);
                                    persist_changed |= self.draw_settings_display_effects_panel(ui);
                                }
                                2 => {
                                    Self::settings_section(ui, "Desktop Theme Pack", |ui| {
                                        egui::ComboBox::from_id_salt(
                                            "native_desktop_theme_pack",
                                        )
                                        .selected_text(
                                            RichText::new(selected_theme_pack_name(
                                                self.desktop_active_theme_pack_id.as_deref(),
                                                &theme_packs,
                                            ))
                                            .color(palette.fg),
                                        )
                                        .show_ui(ui, |ui| {
                                            Self::apply_settings_control_style(ui);
                                            if Self::retro_choice_button(
                                                ui,
                                                "Manual",
                                                self.desktop_active_theme_pack_id.is_none(),
                                            )
                                            .clicked()
                                            {
                                                self.desktop_active_theme_pack_id = None;
                                                ui.close_menu();
                                            }
                                            for theme in &theme_packs {
                                                let selected = self
                                                    .desktop_active_theme_pack_id
                                                    .as_deref()
                                                    == Some(theme.id.as_str());
                                                if Self::retro_choice_button(
                                                    ui,
                                                    &theme.name,
                                                    selected,
                                                )
                                                .clicked()
                                                {
                                                    self.desktop_active_theme_pack_id =
                                                        Some(theme.id.clone());
                                                    self.desktop_active_color_style =
                                                        theme.color_style.clone();
                                                    self.desktop_active_layout =
                                                        theme.layout_profile.clone();
                                                    desktop_runtime_changed = true;
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    });
                                    ui.add_space(10.0);
                                    Self::settings_section(ui, "Desktop Theme Color", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Theme");
                                            let mut current_idx = THEMES
                                                .iter()
                                                .position(|(name, _)| {
                                                    *name
                                                        == theme_name_for_color_style(
                                                            &self.desktop_active_color_style,
                                                        )
                                                })
                                                .unwrap_or(0);
                                            egui::ComboBox::from_id_salt("native_desktop_theme")
                                                .selected_text(
                                                    RichText::new(THEMES[current_idx].0)
                                                        .color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (idx, (name, _)) in
                                                        THEMES.iter().enumerate()
                                                    {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            *name,
                                                            current_idx == idx,
                                                        )
                                                        .clicked()
                                                        {
                                                            current_idx = idx;
                                                            self.desktop_active_color_style =
                                                                color_style_from_theme_name(
                                                                    name,
                                                                    custom_rgb_for_color_style(
                                                                        &self.desktop_active_color_style,
                                                                    ),
                                                                );
                                                            self.desktop_active_theme_pack_id =
                                                                None;
                                                            desktop_runtime_changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                        });
                                        if theme_name_for_color_style(&self.desktop_active_color_style)
                                            == CUSTOM_THEME_NAME
                                        {
                                            let mut rgb = custom_rgb_for_color_style(
                                                &self.desktop_active_color_style,
                                            );
                                            let preview_color =
                                                egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                            ui.visuals_mut().selection.bg_fill = preview_color;
                                            ui.visuals_mut().widgets.inactive.bg_fill = palette.dim;
                                            let mut changed_rgb = false;
                                            changed_rgb |= ui
                                                .add(
                                                    egui::Slider::new(&mut rgb[0], 0..=255)
                                                        .text("Red"),
                                                )
                                                .changed();
                                            changed_rgb |= ui
                                                .add(
                                                    egui::Slider::new(&mut rgb[1], 0..=255)
                                                        .text("Green"),
                                                )
                                                .changed();
                                            changed_rgb |= ui
                                                .add(
                                                    egui::Slider::new(&mut rgb[2], 0..=255)
                                                        .text("Blue"),
                                                )
                                                .changed();
                                            if changed_rgb {
                                                self.desktop_active_color_style =
                                                    ColorStyle::Monochrome {
                                                        preset: MonochromePreset::Custom,
                                                        custom_rgb: Some(rgb),
                                                    };
                                                self.desktop_active_theme_pack_id = None;
                                                desktop_runtime_changed = true;
                                            }
                                        }
                                    });
                                }
                                3 => {
                                    Self::settings_section(ui, "Desktop Icons", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Icon Style");
                                            let selected = match self.settings.draft.desktop_icon_style
                                            {
                                                DesktopIconStyle::Dos => "DOS",
                                                DesktopIconStyle::Win95 => "Win95",
                                                DesktopIconStyle::Minimal => "Minimal",
                                                DesktopIconStyle::NoIcons => "No Icons",
                                            };
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_desktop_icons",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for (style, label) in [
                                                    (DesktopIconStyle::Dos, "DOS"),
                                                    (DesktopIconStyle::Win95, "Win95"),
                                                    (DesktopIconStyle::Minimal, "Minimal"),
                                                    (DesktopIconStyle::NoIcons, "No Icons"),
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        label,
                                                        self.settings.draft.desktop_icon_style
                                                            == style,
                                                    )
                                                    .clicked()
                                                    {
                                                        set_desktop_icon_style(
                                                            &mut self.settings.draft,
                                                            style,
                                                        );
                                                        persist_changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new("Built-in Desktop Icons")
                                                .color(palette.fg)
                                                .strong(),
                                        );
                                        ui.add_space(4.0);
                                        for entry in desktop_builtin_icons() {
                                            let mut visible = !self
                                                .settings
                                                .draft
                                                .desktop_hidden_builtin_icons
                                                .contains(entry.key);
                                            if Self::retro_checkbox_row(
                                                ui,
                                                &mut visible,
                                                &format!("Show {}", entry.label),
                                            )
                                            .clicked()
                                            {
                                                set_builtin_icon_visible(
                                                    &mut self.settings.draft,
                                                    entry.key,
                                                    visible,
                                                );
                                                persist_changed = true;
                                            }
                                        }
                                        ui.add_space(8.0);
                                        if Self::retro_checkbox_row(
                                            ui,
                                            &mut self.settings.draft.desktop_show_cursor,
                                            "Show desktop cursor",
                                        )
                                        .clicked()
                                        {
                                            persist_changed = true;
                                        }
                                        if self.settings.draft.desktop_show_cursor {
                                            ui.add_space(6.0);
                                            ui.scope(|ui| {
                                                ui.visuals_mut().selection.bg_fill = palette.fg;
                                                ui.visuals_mut().widgets.inactive.bg_fill =
                                                    palette.dim;
                                                persist_changed |= ui
                                                    .add(
                                                        egui::Slider::new(
                                                            &mut self
                                                                .settings
                                                                .draft
                                                                .desktop_cursor_scale,
                                                            0.5..=2.5,
                                                        )
                                                        .text("Cursor Size"),
                                                    )
                                                    .changed();
                                            });
                                        }
                                    });
                                }
                                _ => {
                                    Self::settings_section(ui, "Desktop Layout", |ui| {
                                        for profile in LayoutProfile::builtin_layouts() {
                                            let selected =
                                                self.desktop_active_layout.id == profile.id;
                                            let description = match profile.id.as_str() {
                                                "classic" => {
                                                    "Classic - Panel at top, taskbar at bottom"
                                                }
                                                "minimal" => {
                                                    "Minimal - Panel at bottom, no taskbar"
                                                }
                                                _ => profile.name.as_str(),
                                            };
                                            if Self::retro_choice_button(
                                                ui,
                                                description,
                                                selected,
                                            )
                                            .clicked()
                                            {
                                                self.desktop_active_layout = profile;
                                                self.desktop_active_theme_pack_id = None;
                                                desktop_runtime_changed = true;
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        _ => {
                            ui.horizontal(|ui| {
                                for (i, label) in ["Colors", "Layout", "Terminal"]
                                    .iter()
                                    .enumerate()
                                {
                                    let active = self.terminal_tweaks_tab == i as u8;
                                    let color = if active { palette.fg } else { palette.dim };
                                    let btn = ui.add(
                                        egui::Button::new(
                                            RichText::new(*label).color(color).strong(),
                                        )
                                        .stroke(egui::Stroke::new(
                                            if active { 2.0 } else { 1.0 },
                                            color,
                                        ))
                                        .fill(if active { palette.panel } else { palette.bg }),
                                    );
                                    if btn.clicked() {
                                        self.terminal_tweaks_tab = i as u8;
                                    }
                                }
                            });
                            ui.add_space(10.0);
                            Self::retro_separator(ui);
                            ui.add_space(8.0);
                            match self.terminal_tweaks_tab {
                                0 => {
                                    Self::settings_section(ui, "Terminal Theme Pack", |ui| {
                                        egui::ComboBox::from_id_salt(
                                            "native_terminal_theme_pack",
                                        )
                                        .selected_text(
                                            RichText::new(selected_theme_pack_name(
                                                self.terminal_active_theme_pack_id.as_deref(),
                                                &theme_packs,
                                            ))
                                            .color(palette.fg),
                                        )
                                        .show_ui(ui, |ui| {
                                            Self::apply_settings_control_style(ui);
                                            if Self::retro_choice_button(
                                                ui,
                                                "Manual",
                                                self.terminal_active_theme_pack_id.is_none(),
                                            )
                                            .clicked()
                                            {
                                                self.terminal_active_theme_pack_id = None;
                                                ui.close_menu();
                                            }
                                            for theme in &theme_packs {
                                                let selected = self
                                                    .terminal_active_theme_pack_id
                                                    .as_deref()
                                                    == Some(theme.id.as_str());
                                                if Self::retro_choice_button(
                                                    ui,
                                                    &theme.name,
                                                    selected,
                                                )
                                                .clicked()
                                                {
                                                    self.terminal_active_theme_pack_id =
                                                        Some(theme.id.clone());
                                                    self.terminal_active_color_style =
                                                        theme.color_style.clone();
                                                    ui.close_menu();
                                                }
                                            }
                                        });
                                    });
                                    ui.add_space(10.0);
                                    Self::settings_section(ui, "Terminal Theme Color", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Theme");
                                            let mut current_idx = THEMES
                                                .iter()
                                                .position(|(name, _)| {
                                                    *name
                                                        == theme_name_for_color_style(
                                                            &self.terminal_active_color_style,
                                                        )
                                                })
                                                .unwrap_or(0);
                                            egui::ComboBox::from_id_salt("native_terminal_theme")
                                                .selected_text(
                                                    RichText::new(THEMES[current_idx].0)
                                                        .color(palette.fg),
                                                )
                                                .show_ui(ui, |ui| {
                                                    Self::apply_settings_control_style(ui);
                                                    for (idx, (name, _)) in
                                                        THEMES.iter().enumerate()
                                                    {
                                                        if Self::retro_choice_button(
                                                            ui,
                                                            *name,
                                                            current_idx == idx,
                                                        )
                                                        .clicked()
                                                        {
                                                            current_idx = idx;
                                                            self.terminal_active_color_style =
                                                                color_style_from_theme_name(
                                                                    name,
                                                                    custom_rgb_for_color_style(
                                                                        &self.terminal_active_color_style,
                                                                    ),
                                                                );
                                                            self.terminal_active_theme_pack_id =
                                                                None;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                });
                                        });
                                        if theme_name_for_color_style(
                                            &self.terminal_active_color_style,
                                        ) == CUSTOM_THEME_NAME
                                        {
                                            let mut rgb = custom_rgb_for_color_style(
                                                &self.terminal_active_color_style,
                                            );
                                            let preview_color =
                                                egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                            ui.visuals_mut().selection.bg_fill = preview_color;
                                            ui.visuals_mut().widgets.inactive.bg_fill = palette.dim;
                                            let mut changed_rgb = false;
                                            changed_rgb |= ui
                                                .add(
                                                    egui::Slider::new(&mut rgb[0], 0..=255)
                                                        .text("Red"),
                                                )
                                                .changed();
                                            changed_rgb |= ui
                                                .add(
                                                    egui::Slider::new(&mut rgb[1], 0..=255)
                                                        .text("Green"),
                                                )
                                                .changed();
                                            changed_rgb |= ui
                                                .add(
                                                    egui::Slider::new(&mut rgb[2], 0..=255)
                                                        .text("Blue"),
                                                )
                                                .changed();
                                            if changed_rgb {
                                                self.terminal_active_color_style =
                                                    ColorStyle::Monochrome {
                                                        preset: MonochromePreset::Custom,
                                                        custom_rgb: Some(rgb),
                                                    };
                                                self.terminal_active_theme_pack_id = None;
                                            }
                                        }
                                    });
                                }
                                1 => {
                                    Self::settings_section(ui, "Terminal Layout", |ui| {
                                        for profile in TerminalLayoutProfile::builtin_layouts() {
                                            let selected =
                                                self.terminal_active_layout.id == profile.id;
                                            let description = match profile.id.as_str() {
                                                "classic-terminal" => {
                                                    "Classic Terminal - bottom status bar"
                                                }
                                                "minimal-terminal" => {
                                                    "Minimal Terminal - no status bar"
                                                }
                                                _ => profile.name.as_str(),
                                            };
                                            if Self::retro_choice_button(
                                                ui,
                                                description,
                                                selected,
                                            )
                                            .clicked()
                                            {
                                                self.terminal_active_layout = profile;
                                            }
                                        }
                                    });
                                }
                                _ => {
                                    Self::settings_section(ui, "PTY Display", |ui| {
                                        if Self::retro_checkbox_row(
                                            ui,
                                            &mut self.settings.draft.cli_styled_render,
                                            "Styled PTY rendering",
                                        )
                                        .clicked()
                                        {
                                            persist_changed = true;
                                        }
                                        ui.add_space(8.0);
                                        ui.horizontal(|ui| {
                                            ui.label("PTY Color Mode");
                                            let selected = match self.settings.draft.cli_color_mode
                                            {
                                                CliColorMode::ThemeLock => "Theme Lock",
                                                CliColorMode::PaletteMap => "Palette-map",
                                                CliColorMode::Color => "Color",
                                                CliColorMode::Monochrome => "Monochrome",
                                            };
                                            egui::ComboBox::from_id_salt(
                                                "native_settings_cli_color",
                                            )
                                            .selected_text(
                                                RichText::new(selected).color(palette.fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                Self::apply_settings_control_style(ui);
                                                for (mode, label) in [
                                                    (CliColorMode::ThemeLock, "Theme Lock"),
                                                    (CliColorMode::PaletteMap, "Palette-map"),
                                                    (CliColorMode::Color, "Color"),
                                                    (CliColorMode::Monochrome, "Monochrome"),
                                                ] {
                                                    if Self::retro_choice_button(
                                                        ui,
                                                        label,
                                                        self.settings.draft.cli_color_mode == mode,
                                                    )
                                                    .clicked()
                                                        && self.settings.draft.cli_color_mode
                                                            != mode
                                                    {
                                                        self.settings.draft.cli_color_mode = mode;
                                                        persist_changed = true;
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                        ui.add_space(8.0);
                                        if ui
                                            .button(match self.settings.draft.cli_acs_mode {
                                                CliAcsMode::Ascii => {
                                                    "Border Glyphs: ASCII"
                                                }
                                                CliAcsMode::Unicode => {
                                                    "Border Glyphs: Unicode Smooth"
                                                }
                                            })
                                            .clicked()
                                        {
                                            self.settings.draft.cli_acs_mode =
                                                match self.settings.draft.cli_acs_mode {
                                                    CliAcsMode::Ascii => CliAcsMode::Unicode,
                                                    CliAcsMode::Unicode => CliAcsMode::Ascii,
                                                };
                                            persist_changed = true;
                                        }
                                    });
                                }
                            }
                        }
                    }
                });

            ui.separator();
            if persist_changed {
                {
                    let draft = self.settings.draft.clone();
                    let tx = self.background.sender();
                    std::thread::spawn(move || {
                        persist_settings_draft(&draft);
                        let _ = tx.send(BackgroundResult::SettingsPersisted);
                    });
                }
                self.sync_runtime_settings_cache();
                self.invalidate_desktop_icon_layout_cache();
                self.invalidate_program_catalog_cache();
                self.invalidate_saved_connections_cache();
                self.refresh_settings_sync_marker();
                if window_mode_changed {
                    self.apply_native_window_mode(ctx);
                }
                self.apply_status_update(saved_settings_status());
            }
            if desktop_runtime_changed {
                self.invalidate_desktop_icon_layout_cache();
            }
            if !self.settings.status.is_empty() {
                ui.small(&self.settings.status);
            }
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Tweaks,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::PositionOnly,
            header_action,
        );
    }
}
