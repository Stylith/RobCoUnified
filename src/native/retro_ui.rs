use crate::config::current_theme_color;
use eframe::egui::{
    self, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
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

    pub fn new_sized(ui: &mut Ui, cols: usize, rows: usize, desired: Vec2) -> (Self, Response) {
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        let cell_w = (rect.width() / cols.max(1) as f32).floor().max(8.0);
        let cell_h = (rect.height() / rows.max(1) as f32).floor().max(12.0);
        // Terminal sizing is driven by the caller-provided grid (cols/rows).
        // Keeping glyph sizing grid-relative avoids double-scaling artifacts.
        let target_font = (cell_h * 0.80).max(8.0);
        let height_limit = (cell_h - 3.0).max(8.0);
        let width_limit = ((cell_w - 1.0).max(7.0) / 0.53).max(8.0);
        (
            Self {
                rect,
                cols,
                cell: egui::vec2(cell_w, cell_h),
                font: FontId::monospace(target_font.min(height_limit.min(width_limit))),
            },
            response,
        )
    }

    fn clip_text(&self, col: usize, text: &str) -> String {
        let max_chars = self.cols.saturating_sub(col);
        text.chars().take(max_chars).collect()
    }

    pub fn font(&self) -> &FontId {
        &self.font
    }

    fn row_top(&self, row: usize) -> f32 {
        self.rect.top() + row as f32 * self.cell.y
    }

    fn row_text_y(&self, row: usize) -> f32 {
        let top = self.row_top(row);
        let inset = ((self.cell.y - self.font.size).max(0.0) * 0.5).floor();
        top + inset
    }

    fn paint_text(
        &self,
        painter: &Painter,
        pos: Pos2,
        align: Align2,
        text: &str,
        color: Color32,
        faux_bold: bool,
    ) {
        painter.text(pos, align, text, self.font.clone(), color);
        if faux_bold {
            painter.text(
                Pos2::new(pos.x + 0.7, pos.y),
                align,
                text,
                self.font.clone(),
                color,
            );
        }
    }

    pub fn paint_bg(&self, painter: &Painter, color: Color32) {
        painter.rect_filled(self.rect, 0.0, color);
    }

    pub fn text(&self, painter: &Painter, col: usize, row: usize, text: &str, color: Color32) {
        let clipped = self.clip_text(col, text);
        let pos = Pos2::new(
            self.rect.left() + col as f32 * self.cell.x,
            self.row_text_y(row),
        );
        self.paint_text(painter, pos, Align2::LEFT_TOP, &clipped, color, false);
    }

    pub fn underlined_text(
        &self,
        painter: &Painter,
        col: usize,
        row: usize,
        text: &str,
        color: Color32,
    ) {
        let clipped = self.clip_text(col, text);
        let pos = Pos2::new(
            self.rect.left() + col as f32 * self.cell.x,
            self.row_text_y(row),
        );
        self.paint_text(painter, pos, Align2::LEFT_TOP, &clipped, color, false);
        // Force underline width to character-cell geometry so it stays stable
        // across fonts/scales and does not overshoot subtitle text.
        let width = clipped.trim_end_matches(' ').chars().count() as f32 * self.cell.x;
        if width > 0.0 {
            let row_bottom = self.row_top(row) + self.cell.y - 1.0;
            let y = (pos.y + self.font.size + 1.0).min(row_bottom);
            painter.line_segment(
                [Pos2::new(pos.x, y), Pos2::new(pos.x + width, y)],
                Stroke::new(2.0, color),
            );
        }
    }

    pub fn centered_text(
        &self,
        painter: &Painter,
        row: usize,
        text: &str,
        color: Color32,
        strong: bool,
    ) {
        let clipped = self.clip_text(1, text);
        let pos = Pos2::new(self.rect.center().x, self.row_text_y(row));
        self.paint_text(painter, pos, Align2::CENTER_TOP, &clipped, color, strong);
    }

    pub fn separator(&self, painter: &Painter, row: usize, palette: &RetroPalette) {
        let text = "=".repeat(self.cols.saturating_sub(6));
        self.centered_text(painter, row, &text, palette.dim, false);
    }

    pub fn row_rect(&self, col: usize, row: usize, width_chars: usize) -> Rect {
        let left = self.rect.left() + col as f32 * self.cell.x;
        let top = self.rect.top() + row as f32 * self.cell.y;
        let nominal_right = left + width_chars.max(1) as f32 * self.cell.x;
        let right = if col.saturating_add(width_chars.max(1)) >= self.cols {
            self.rect.right()
        } else {
            nominal_right.min(self.rect.right())
        };
        Rect::from_min_max(
            Pos2::new(left.min(self.rect.right()), top),
            Pos2::new(right.max(left), top + self.cell.y),
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
        let clipped = self.clip_text(col, text);
        let left = self.rect.left() + col as f32 * self.cell.x;
        let top = self.row_top(row);
        let measured = painter
            .layout_no_wrap(clipped.clone(), self.font.clone(), palette.fg)
            .size();
        let measured_w = measured.x;
        let measured_h = measured.y.max(1.0);
        // Legacy TUI highlight paints exactly the selected text span.
        let right = (left + measured_w).min(self.rect.right());
        let hit_rect = Rect::from_min_max(
            Pos2::new(left.min(self.rect.right()), top),
            Pos2::new(right.max(left), top + self.cell.y),
        );
        let text_top = self.row_text_y(row).max(hit_rect.top());
        let text_bottom = (text_top + measured_h).min(hit_rect.bottom());
        let paint_rect = Rect::from_min_max(
            Pos2::new(hit_rect.left(), text_top),
            Pos2::new(hit_rect.right(), text_bottom.max(text_top + 1.0)),
        );
        if selected {
            painter.rect_filled(paint_rect, 0.0, palette.selected_bg);
        }
        self.paint_text(
            painter,
            Pos2::new(hit_rect.left(), self.row_text_y(row)),
            Align2::LEFT_TOP,
            &clipped,
            if selected {
                palette.selected_fg
            } else {
                palette.fg
            },
            selected,
        );
        ui.interact(
            hit_rect,
            ui.id().with(("retro_row", row, col, text)),
            Sense::click(),
        )
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
        let footer_font = FontId::monospace(self.font.size + 0.4);
        let left_pos = Pos2::new(rect.left() + 4.0, rect.top());
        let center_pos = Pos2::new(rect.center().x, rect.top());
        let right_pos = Pos2::new(rect.right() - 4.0, rect.top());
        painter.text(
            left_pos,
            Align2::LEFT_TOP,
            left,
            footer_font.clone(),
            palette.selected_fg,
        );
        painter.text(
            Pos2::new(left_pos.x + 0.7, left_pos.y),
            Align2::LEFT_TOP,
            left,
            footer_font.clone(),
            palette.selected_fg,
        );
        painter.text(
            center_pos,
            Align2::CENTER_TOP,
            center,
            footer_font.clone(),
            palette.selected_fg,
        );
        painter.text(
            Pos2::new(center_pos.x + 0.7, center_pos.y),
            Align2::CENTER_TOP,
            center,
            footer_font.clone(),
            palette.selected_fg,
        );
        painter.text(
            right_pos,
            Align2::RIGHT_TOP,
            right,
            footer_font.clone(),
            palette.selected_fg,
        );
        painter.text(
            Pos2::new(right_pos.x + 0.7, right_pos.y),
            Align2::RIGHT_TOP,
            right,
            footer_font,
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
        let rect = Rect::from_min_size(
            rect.min,
            egui::vec2(w as f32 * self.cell.x, h as f32 * self.cell.y),
        );
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
