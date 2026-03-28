use super::super::desktop_app::DesktopWindow;
use super::super::retro_ui::current_palette;
use super::{CachedIcon, RobcoNativeApp};
use eframe::egui::{
    self, Align2, Color32, Context, FontFamily, FontId, Id, RichText, TextStyle, TextureHandle,
};
use std::path::Path;

impl RobcoNativeApp {
    pub(super) fn load_svg_icon(
        ctx: &Context,
        id: &str,
        svg_bytes: &[u8],
        size_px: Option<u32>,
    ) -> TextureHandle {
        let tree =
            usvg::Tree::from_data(svg_bytes, &usvg::Options::default()).expect("invalid SVG icon");
        let natural = tree.size().to_int_size();
        let target_size = size_px.unwrap_or(natural.width().max(natural.height()));
        let scale = target_size as f32 / natural.width().max(natural.height()) as f32;
        let width = (natural.width() as f32 * scale).round() as u32;
        let height = (natural.height() as f32 * scale).round() as u32;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("zero-sized SVG icon");
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for pixel in pixmap.pixels() {
            rgba.extend_from_slice(&[255, 255, 255, pixel.alpha()]);
        }
        let image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        ctx.load_texture(id, image, egui::TextureOptions::LINEAR)
    }

    pub(super) fn load_svg_icon_color(
        ctx: &Context,
        id: &str,
        svg_bytes: &[u8],
        size_px: Option<u32>,
    ) -> TextureHandle {
        let tree =
            usvg::Tree::from_data(svg_bytes, &usvg::Options::default()).expect("invalid SVG icon");
        let natural = tree.size().to_int_size();
        let target_size = size_px.unwrap_or(natural.width().max(natural.height()));
        let scale = target_size as f32 / natural.width().max(natural.height()) as f32;
        let width = (natural.width() as f32 * scale).round() as u32;
        let height = (natural.height() as f32 * scale).round() as u32;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("zero-sized SVG icon");
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(scale, scale),
            &mut pixmap.as_mut(),
        );

        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for pixel in pixmap.pixels() {
            let color = pixel.demultiply();
            rgba.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
        }
        let image =
            egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
        ctx.load_texture(id, image, egui::TextureOptions::LINEAR)
    }

    fn resolve_themed_icon_bytes(
        icon_name: &str,
        asset_pack_path: Option<&Path>,
        color_mode_is_full_color: bool,
    ) -> Option<(Vec<u8>, bool)> {
        let pack_path = asset_pack_path?;
        let ordered_dirs = if color_mode_is_full_color {
            [("icons_color", true), ("icons_mono", false)]
        } else {
            [("icons_mono", false), ("icons_color", true)]
        };
        for (dir, is_full_color) in ordered_dirs {
            let icon_path = pack_path.join(dir).join(icon_name);
            if let Ok(bytes) = std::fs::read(icon_path) {
                return Some((bytes, is_full_color));
            }
        }
        None
    }

    pub(super) fn load_themed_svg_icon(
        ctx: &Context,
        id: &str,
        icon_name: &str,
        builtin_svg_bytes: &[u8],
        size_px: Option<u32>,
        asset_pack_path: Option<&Path>,
        color_mode_is_full_color: bool,
    ) -> CachedIcon {
        if let Some((svg_bytes, is_full_color)) =
            Self::resolve_themed_icon_bytes(icon_name, asset_pack_path, color_mode_is_full_color)
        {
            let texture = if is_full_color {
                Self::load_svg_icon_color(ctx, id, &svg_bytes, size_px)
            } else {
                Self::load_svg_icon(ctx, id, &svg_bytes, size_px)
            };
            return CachedIcon {
                texture,
                is_full_color,
            };
        }

        CachedIcon {
            texture: Self::load_svg_icon(ctx, id, builtin_svg_bytes, size_px),
            is_full_color: false,
        }
    }

    pub(super) fn ensure_cached_svg_icon(
        slot: &mut Option<CachedIcon>,
        ctx: &Context,
        id: &str,
        icon_name: &str,
        builtin_svg_bytes: &[u8],
        size_px: Option<u32>,
        asset_pack_path: Option<&Path>,
        color_mode_is_full_color: bool,
    ) -> CachedIcon {
        slot.get_or_insert_with(|| {
            Self::load_themed_svg_icon(
                ctx,
                id,
                icon_name,
                builtin_svg_bytes,
                size_px,
                asset_pack_path,
                color_mode_is_full_color,
            )
        })
            .clone()
    }

    pub(super) fn paint_tinted_texture(
        painter: &egui::Painter,
        texture: &TextureHandle,
        rect: egui::Rect,
        tint: Color32,
    ) {
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        painter.image(texture.id(), rect, uv, tint);
    }

    pub(super) fn paint_cached_icon(
        painter: &egui::Painter,
        icon: &CachedIcon,
        rect: egui::Rect,
        tint: Color32,
    ) {
        let effective_tint = if icon.is_full_color {
            Color32::WHITE
        } else {
            tint
        };
        Self::paint_tinted_texture(painter, &icon.texture, rect, effective_tint);
    }

    pub(super) fn fit_texture_rect(texture: &TextureHandle, bounds: egui::Rect) -> egui::Rect {
        let image_size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
        if image_size.x <= 0.0
            || image_size.y <= 0.0
            || bounds.width() <= 0.0
            || bounds.height() <= 0.0
        {
            return bounds;
        }
        let scale = (bounds.width() / image_size.x)
            .min(bounds.height() / image_size.y)
            .min(1.0);
        egui::Rect::from_center_size(bounds.center(), image_size * scale)
    }

    pub(super) fn truncate_file_manager_label(text: &str, max_chars: usize) -> String {
        let total_chars = text.chars().count();
        if total_chars <= max_chars {
            return text.to_string();
        }
        if max_chars <= 3 {
            return ".".repeat(max_chars);
        }
        let suffix_budget = ((max_chars - 3) + 1) / 2;
        let mut suffix: String = text
            .chars()
            .skip(total_chars.saturating_sub(suffix_budget))
            .collect();
        if total_chars > suffix_budget && suffix.starts_with('.') {
            suffix.remove(0);
        }
        let prefix_budget = max_chars.saturating_sub(3 + suffix.chars().count());
        let prefix: String = text.chars().take(prefix_budget).collect();
        format!("{prefix}...{suffix}")
    }

    pub(super) fn active_editor_text_edit_id(&self) -> Id {
        let generation = self.desktop_window_generation(DesktopWindow::Editor.into());
        Id::new(("editor_text_edit", generation))
    }

    pub(super) fn retro_separator(ui: &mut egui::Ui) {
        let palette = current_palette();
        let desired = egui::vec2(ui.available_width().max(1.0), 2.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        ui.painter().rect_filled(rect, 0.0, palette.fg);
        ui.add_space(2.0);
    }

    pub(super) fn retro_disabled_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
    ) -> egui::Response {
        let palette = current_palette();
        ui.add(
            egui::Button::new(RichText::new(label.into()).color(palette.dim))
                .sense(egui::Sense::hover()),
        )
    }

    pub(super) fn apply_settings_control_style(ui: &mut egui::Ui) {
        let palette = current_palette();
        let mut style = ui.style().as_ref().clone();
        let stroke = egui::Stroke::new(2.0, palette.fg);
        style.visuals.override_text_color = None;
        style.visuals.window_fill = palette.bg;
        style.visuals.panel_fill = palette.bg;
        style.visuals.faint_bg_color = palette.bg;
        style.visuals.extreme_bg_color = palette.bg;
        style.visuals.code_bg_color = palette.bg;
        style.visuals.window_stroke = stroke;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;
        style.visuals.window_shadow = egui::epaint::Shadow::NONE;
        style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        style.visuals.selection.bg_fill = palette.selected_bg;
        style.visuals.selection.stroke = stroke;
        style.visuals.hyperlink_color = palette.fg;
        style.visuals.text_cursor.stroke = stroke;
        style.visuals.widgets.noninteractive.bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.weak_bg_fill = palette.bg;
        style.visuals.widgets.noninteractive.bg_stroke = stroke;
        style.visuals.widgets.noninteractive.fg_stroke = stroke;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.noninteractive.expansion = 0.0;
        style.visuals.widgets.inactive.bg_fill = palette.bg;
        style.visuals.widgets.inactive.weak_bg_fill = palette.bg;
        style.visuals.widgets.inactive.bg_stroke = stroke;
        style.visuals.widgets.inactive.fg_stroke = stroke;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.expansion = 0.0;
        style.visuals.widgets.hovered.bg_fill = palette.selected_bg;
        style.visuals.widgets.hovered.weak_bg_fill = palette.selected_bg;
        style.visuals.widgets.hovered.bg_stroke = stroke;
        style.visuals.widgets.hovered.fg_stroke.color = palette.selected_fg;
        style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.hovered.expansion = 0.0;
        style.visuals.widgets.active.bg_fill = palette.selected_bg;
        style.visuals.widgets.active.weak_bg_fill = palette.selected_bg;
        style.visuals.widgets.active.bg_stroke = stroke;
        style.visuals.widgets.active.fg_stroke.color = palette.selected_fg;
        style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.active.expansion = 0.0;
        style.visuals.widgets.open.bg_fill = palette.selected_bg;
        style.visuals.widgets.open.weak_bg_fill = palette.selected_bg;
        style.visuals.widgets.open.bg_stroke = stroke;
        style.visuals.widgets.open.fg_stroke.color = palette.selected_fg;
        style.visuals.widgets.open.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.open.expansion = 0.0;
        ui.set_style(style);
    }

    pub(super) fn retro_choice_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
        selected: bool,
    ) -> egui::Response {
        let palette = current_palette();
        let label = label.into();
        let button = if selected {
            egui::Button::new(label.clone())
                .fill(palette.selected_bg)
                .stroke(egui::Stroke::new(2.0, palette.fg))
        } else {
            egui::Button::new(label.clone()).stroke(egui::Stroke::new(2.0, palette.fg))
        };
        let response = ui.add(button);
        if selected {
            let font = TextStyle::Button.resolve(ui.style());
            ui.painter().text(
                response.rect.center(),
                Align2::CENTER_CENTER,
                label,
                font,
                palette.selected_fg,
            );
        }
        response
    }

    pub(super) fn retro_checkbox_row(
        ui: &mut egui::Ui,
        value: &mut bool,
        label: &str,
    ) -> egui::Response {
        let marker = if *value { "[x]" } else { "[ ]" };
        let response = ui.add(
            egui::Button::new(format!("{marker} {label}"))
                .stroke(egui::Stroke::new(2.0, current_palette().fg)),
        );
        if response.clicked() {
            *value = !*value;
        }
        response
    }

    pub(super) fn retro_settings_tile(
        ui: &mut egui::Ui,
        texture: Option<&CachedIcon>,
        icon: &str,
        label: &str,
        enabled: bool,
        desired: egui::Vec2,
        icon_font_size: f32,
        label_font_size: f32,
    ) -> egui::Response {
        let palette = current_palette();
        let sense = if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(desired, sense);
        let hovered = enabled && response.hovered();
        if hovered {
            ui.painter().rect_filled(rect, 0.0, palette.selected_bg);
        }
        let text_color = if hovered {
            palette.selected_fg
        } else {
            palette.fg
        };
        if let Some(texture) = texture {
            let icon_side = (desired.y * 0.34).clamp(24.0, 40.0);
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + desired.y * 0.34),
                egui::vec2(icon_side, icon_side),
            );
            Self::paint_cached_icon(ui.painter(), texture, icon_rect, text_color);
        } else {
            ui.painter().text(
                rect.left_top() + egui::vec2(8.0, desired.y * 0.18),
                Align2::LEFT_TOP,
                icon,
                FontId::new(icon_font_size, FontFamily::Monospace),
                text_color,
            );
        }
        ui.painter().text(
            egui::pos2(rect.center().x, rect.top() + desired.y * 0.70),
            Align2::CENTER_CENTER,
            label,
            FontId::new(label_font_size, FontFamily::Monospace),
            text_color,
        );
        response
    }

    pub(super) fn retro_full_width_button(
        ui: &mut egui::Ui,
        label: impl Into<String>,
    ) -> egui::Response {
        let palette = current_palette();
        ui.add_sized(
            [ui.available_width().max(160.0), 0.0],
            egui::Button::new(label.into()).stroke(egui::Stroke::new(2.0, palette.fg)),
        )
    }

    pub(super) fn responsive_input_width(ui: &egui::Ui, fraction: f32, min: f32, max: f32) -> f32 {
        (ui.available_width() * fraction).clamp(min, max)
    }

    pub(super) fn settings_two_columns<R>(
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui, &mut egui::Ui) -> R,
    ) -> R {
        let total_w = ui.available_width().min(860.0);
        let column_gap = 18.0;
        let column_w = ((total_w - column_gap) * 0.5).max(220.0);
        ui.columns(2, |columns| {
            let (left_slice, right_slice) = columns.split_at_mut(1);
            let left = &mut left_slice[0];
            let right = &mut right_slice[0];
            left.set_width(column_w);
            right.set_width(column_w);
            add_contents(left, right)
        })
    }

    pub(super) fn settings_section<R>(
        ui: &mut egui::Ui,
        title: &str,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let palette = current_palette();
        egui::Frame::none()
            .fill(palette.panel)
            .stroke(egui::Stroke::new(2.0, palette.fg))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.strong(title);
                ui.add_space(8.0);
                Self::retro_separator(ui);
                ui.add_space(8.0);
                add_contents(ui)
            })
            .inner
    }
}

#[cfg(test)]
mod tests {
    use super::RobcoNativeApp;
    use std::path::PathBuf;

    struct TempDirGuard {
        path: PathBuf,
    }

    impl TempDirGuard {
        fn new(prefix: &str) -> Self {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "nucleon_phase7_ui_helpers_{prefix}_{}_{}",
                std::process::id(),
                unique
            ));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }
    }

    impl Drop for TempDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn phase_7_icon_bytes_prefer_full_color_directory() {
        let temp = TempDirGuard::new("prefer_color");
        std::fs::create_dir_all(temp.path.join("icons_color")).expect("create color dir");
        std::fs::create_dir_all(temp.path.join("icons_mono")).expect("create mono dir");
        std::fs::write(
            temp.path.join("icons_color").join("pixel--folder-solid.svg"),
            b"color",
        )
        .expect("write color icon");
        std::fs::write(
            temp.path.join("icons_mono").join("pixel--folder-solid.svg"),
            b"mono",
        )
        .expect("write mono icon");

        let (bytes, is_full_color) = RobcoNativeApp::resolve_themed_icon_bytes(
            "pixel--folder-solid.svg",
            Some(temp.path.as_path()),
            true,
        )
        .expect("themed icon bytes");

        assert_eq!(bytes, b"color");
        assert!(is_full_color);
    }

    #[test]
    fn phase_7_icon_bytes_fall_back_to_color_when_mono_missing() {
        let temp = TempDirGuard::new("fallback_color");
        std::fs::create_dir_all(temp.path.join("icons_color")).expect("create color dir");
        std::fs::write(
            temp.path.join("icons_color").join("pixel--folder-solid.svg"),
            b"color-only",
        )
        .expect("write color icon");

        let (bytes, is_full_color) = RobcoNativeApp::resolve_themed_icon_bytes(
            "pixel--folder-solid.svg",
            Some(temp.path.as_path()),
            false,
        )
        .expect("themed icon bytes");

        assert_eq!(bytes, b"color-only");
        assert!(is_full_color);
    }

    #[test]
    fn phase_7_icon_bytes_return_none_when_icon_missing_everywhere() {
        let temp = TempDirGuard::new("missing");
        std::fs::create_dir_all(temp.path.join("icons_color")).expect("create color dir");
        std::fs::create_dir_all(temp.path.join("icons_mono")).expect("create mono dir");

        assert!(
            RobcoNativeApp::resolve_themed_icon_bytes(
                "pixel--folder-solid.svg",
                Some(temp.path.as_path()),
                true,
            )
            .is_none()
        );
    }
}
