use eframe::egui::{
    self, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, Ui,
    Vec2,
};

pub const RETRO_GREEN: Color32 = Color32::from_rgb(111, 255, 84);
pub const RETRO_GREEN_DIM: Color32 = Color32::from_rgb(56, 130, 42);
pub const RETRO_BG: Color32 = Color32::from_rgb(6, 10, 6);
pub const RETRO_PANEL: Color32 = Color32::from_rgb(10, 14, 10);
pub const RETRO_SEL_BG: Color32 = Color32::from_rgb(111, 255, 84);
pub const RETRO_SEL_FG: Color32 = Color32::from_rgb(0, 0, 0);

pub struct RetroScreen {
    pub rect: Rect,
    cols: usize,
    rows: usize,
    cell: Vec2,
    font: FontId,
}

impl RetroScreen {
    pub fn new(ui: &mut Ui, cols: usize, rows: usize) -> (Self, Response) {
        let desired = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        let cell_w = (rect.width() / cols.max(1) as f32).floor().max(8.0);
        let cell_h = (rect.height() / rows.max(1) as f32).floor().max(12.0);
        (
            Self {
                rect,
                cols,
                rows,
                cell: egui::vec2(cell_w, cell_h),
                font: FontId::monospace((cell_h - 2.0).max(10.0)),
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

    pub fn separator(&self, painter: &Painter, row: usize) {
        let text = "=".repeat(self.cols.saturating_sub(4));
        self.centered_text(painter, row, &text, RETRO_GREEN_DIM, false);
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
        col: usize,
        row: usize,
        text: &str,
        selected: bool,
    ) -> Response {
        let rect = self.row_rect(col, row, text.chars().count());
        if selected {
            painter.rect_filled(rect, 0.0, RETRO_SEL_BG);
        }
        painter.text(
            rect.left_top(),
            Align2::LEFT_TOP,
            text,
            self.font.clone(),
            if selected { RETRO_SEL_FG } else { RETRO_GREEN },
        );
        ui.interact(rect, ui.id().with(("retro_row", row, col, text)), Sense::click())
    }

    pub fn footer_bar(
        &self,
        painter: &Painter,
        left: &str,
        center: &str,
        right: &str,
    ) {
        let row = self.rows.saturating_sub(1);
        let rect = self.row_rect(0, row, self.cols);
        painter.rect_filled(rect, 0.0, RETRO_SEL_BG);
        painter.text(
            rect.left_top(),
            Align2::LEFT_TOP,
            left,
            self.font.clone(),
            RETRO_SEL_FG,
        );
        painter.text(
            Pos2::new(rect.center().x, rect.top()),
            Align2::CENTER_TOP,
            center,
            self.font.clone(),
            RETRO_SEL_FG,
        );
        painter.text(
            Pos2::new(rect.right(), rect.top()),
            Align2::RIGHT_TOP,
            right,
            self.font.clone(),
            RETRO_SEL_FG,
        );
    }

    pub fn boxed_panel(&self, painter: &Painter, col: usize, row: usize, w: usize, h: usize) {
        let rect = self.row_rect(col, row, w);
        let rect = Rect::from_min_size(rect.min, egui::vec2(w as f32 * self.cell.x, h as f32 * self.cell.y));
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, RETRO_GREEN));
    }
}

pub fn configure_visuals(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(RETRO_GREEN);
    visuals.window_fill = RETRO_BG;
    visuals.panel_fill = RETRO_PANEL;
    visuals.widgets.noninteractive.bg_fill = RETRO_PANEL;
    visuals.widgets.noninteractive.fg_stroke.color = RETRO_GREEN;
    visuals.widgets.inactive.bg_fill = RETRO_BG;
    visuals.widgets.inactive.fg_stroke.color = RETRO_GREEN;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(25, 45, 20);
    visuals.widgets.hovered.fg_stroke.color = RETRO_GREEN;
    visuals.widgets.active.bg_fill = Color32::from_rgb(35, 65, 25);
    visuals.widgets.active.fg_stroke.color = RETRO_GREEN;
    visuals.selection.bg_fill = Color32::from_rgb(35, 65, 25);
    visuals.selection.stroke.color = RETRO_GREEN;
    visuals.extreme_bg_color = RETRO_BG;
    visuals.faint_bg_color = RETRO_PANEL;
    ctx.set_visuals(visuals);
}
