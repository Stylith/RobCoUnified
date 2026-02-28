use crate::config::{current_theme_color, get_settings};
use eframe::egui::{
    self, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui,
    Vec2,
};
use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct RetroPalette {
    pub fg: Color32,
    pub dim: Color32,
    pub bg: Color32,
    pub panel: Color32,
    pub selected_bg: Color32,
    pub selected_fg: Color32,
    pub hovered_bg: Color32,
    pub active_bg: Color32,
    pub selection_bg: Color32,
}

fn color32_from_theme(color: Color) -> Color32 {
    match color {
        Color::Black => Color32::from_rgb(0, 0, 0),
        Color::DarkGray => Color32::from_rgb(85, 85, 85),
        Color::Gray => Color32::from_rgb(170, 170, 170),
        Color::White => Color32::from_rgb(240, 240, 240),
        Color::Red | Color::LightRed => Color32::from_rgb(255, 90, 90),
        Color::Green | Color::LightGreen => Color32::from_rgb(111, 255, 84),
        Color::Yellow | Color::LightYellow => Color32::from_rgb(255, 191, 74),
        Color::Blue | Color::LightBlue => Color32::from_rgb(105, 180, 255),
        Color::Magenta | Color::LightMagenta => Color32::from_rgb(214, 112, 255),
        Color::Cyan | Color::LightCyan => Color32::from_rgb(110, 235, 255),
        Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        Color::Indexed(_) | Color::Reset => Color32::from_rgb(111, 255, 84),
    }
}

fn scale(color: Color32, factor: f32) -> Color32 {
    let [r, g, b, a] = color.to_array();
    Color32::from_rgba_unmultiplied(
        ((r as f32) * factor).clamp(0.0, 255.0) as u8,
        ((g as f32) * factor).clamp(0.0, 255.0) as u8,
        ((b as f32) * factor).clamp(0.0, 255.0) as u8,
        a,
    )
}

pub fn current_palette() -> RetroPalette {
    let fg = color32_from_theme(current_theme_color());
    let brightness = (fg.r() as u16 + fg.g() as u16 + fg.b() as u16) / 3;
    let selected_fg = if brightness > 96 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    RetroPalette {
        fg,
        dim: scale(fg, 0.52),
        bg: Color32::from_rgb(0, 0, 0),
        panel: scale(fg, 0.06),
        selected_bg: fg,
        selected_fg,
        hovered_bg: scale(fg, 0.18),
        active_bg: scale(fg, 0.26),
        selection_bg: scale(fg, 0.26),
    }
}

pub struct RetroScreen {
    pub rect: Rect,
    cols: usize,
    cell: Vec2,
    font: FontId,
}

impl RetroScreen {
    pub fn new(ui: &mut Ui, cols: usize, rows: usize) -> (Self, Response) {
        let desired = ui.available_size();
        Self::new_sized(ui, cols, rows, desired)
    }

    pub fn new_sized(
        ui: &mut Ui,
        cols: usize,
        rows: usize,
        desired: Vec2,
    ) -> (Self, Response) {
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        let cell_w = (rect.width() / cols.max(1) as f32).floor().max(8.0);
        let cell_h = (rect.height() / rows.max(1) as f32).floor().max(12.0);
        let scale = get_settings().native_ui_scale.clamp(0.75, 1.75);
        (
            Self {
                rect,
                cols,
                cell: egui::vec2(cell_w, cell_h),
                font: FontId::monospace(((cell_h - 2.0).max(10.0) * scale).max(10.0)),
            },
            response,
        )
    }

    pub fn paint_bg(&self, painter: &Painter, color: Color32) {
        painter.rect_filled(self.rect, 0.0, color);
    }

    pub fn text(&self, painter: &Painter, col: usize, row: usize, text: &str, color: Color32) {
        let pos = Pos2::new(
            self.rect.left() + col as f32 * self.cell.x,
            self.rect.top() + row as f32 * self.cell.y,
        );
        painter.text(pos, Align2::LEFT_TOP, text, self.font.clone(), color);
    }

    pub fn centered_text(
        &self,
        painter: &Painter,
        row: usize,
        text: &str,
        color: Color32,
        strong: bool,
    ) {
        let font = if strong {
            FontId::monospace(self.font.size + 1.0)
        } else {
            self.font.clone()
        };
        let pos = Pos2::new(self.rect.center().x, self.rect.top() + row as f32 * self.cell.y);
        painter.text(pos, Align2::CENTER_TOP, text, font, color);
    }

    pub fn separator(&self, painter: &Painter, row: usize, palette: &RetroPalette) {
        let text = "=".repeat(self.cols.saturating_sub(4));
        self.centered_text(painter, row, &text, palette.dim, false);
    }

    pub fn row_rect(&self, col: usize, row: usize, width_chars: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.rect.left() + col as f32 * self.cell.x,
                self.rect.top() + row as f32 * self.cell.y,
            ),
            egui::vec2(width_chars as f32 * self.cell.x, self.cell.y),
        )
    }

    pub fn selectable_row(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        palette: &RetroPalette,
        col: usize,
        row: usize,
        text: &str,
        selected: bool,
    ) -> Response {
        let rect = self.row_rect(col, row, text.chars().count());
        if selected {
            painter.rect_filled(rect, 0.0, palette.selected_bg);
        }
        painter.text(
            rect.left_top(),
            Align2::LEFT_TOP,
            text,
            self.font.clone(),
            if selected { palette.selected_fg } else { palette.fg },
        );
        ui.interact(rect, ui.id().with(("retro_row", row, col, text)), Sense::click())
    }

    pub fn footer_bar(
        &self,
        painter: &Painter,
        palette: &RetroPalette,
        left: &str,
        center: &str,
        right: &str,
    ) {
        let rect = self.rect;
        painter.rect_filled(rect, 0.0, palette.selected_bg);
        let left_pos = Pos2::new(rect.left() + 4.0, rect.top());
        let center_pos = Pos2::new(rect.center().x, rect.top());
        let right_pos = Pos2::new(rect.right() - 4.0, rect.top());
        painter.text(
            left_pos,
            Align2::LEFT_TOP,
            left,
            self.font.clone(),
            palette.selected_fg,
        );
        painter.text(
            center_pos,
            Align2::CENTER_TOP,
            center,
            self.font.clone(),
            palette.selected_fg,
        );
        painter.text(
            right_pos,
            Align2::RIGHT_TOP,
            right,
            self.font.clone(),
            palette.selected_fg,
        );
    }

    pub fn boxed_panel(
        &self,
        painter: &Painter,
        palette: &RetroPalette,
        col: usize,
        row: usize,
        w: usize,
        h: usize,
    ) {
        let rect = self.row_rect(col, row, w);
        let rect = Rect::from_min_size(rect.min, egui::vec2(w as f32 * self.cell.x, h as f32 * self.cell.y));
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, palette.fg));
    }
}

pub fn configure_visuals(ctx: &Context) {
    let palette = current_palette();
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(palette.fg);
    visuals.window_fill = palette.bg;
    visuals.panel_fill = palette.panel;
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.fg_stroke.color = palette.fg;
    visuals.widgets.inactive.bg_fill = palette.bg;
    visuals.widgets.inactive.fg_stroke.color = palette.fg;
    visuals.widgets.hovered.bg_fill = palette.hovered_bg;
    visuals.widgets.hovered.fg_stroke.color = palette.fg;
    visuals.widgets.active.bg_fill = palette.active_bg;
    visuals.widgets.active.fg_stroke.color = palette.fg;
    visuals.selection.bg_fill = palette.selection_bg;
    visuals.selection.stroke.color = palette.fg;
    visuals.extreme_bg_color = palette.bg;
    visuals.faint_bg_color = palette.panel;
    ctx.set_visuals(visuals);
}
