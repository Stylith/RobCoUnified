use super::retro_theme::{RetroColor, RetroColors};
use crate::pty::{CellColor, CommittedFrame, PtyStyledCell};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry};
use iced::{Color, Font, Pixels, Point, Rectangle, Renderer, Size, Theme};

pub const TERMINAL_CELL_WIDTH: f32 = 11.5;
pub const TERMINAL_CELL_HEIGHT: f32 = 20.0;

pub struct PtyCanvas {
    frame: CommittedFrame,
    palette: RetroColors,
}

impl PtyCanvas {
    pub fn new(frame: CommittedFrame, palette: RetroColors) -> Self {
        Self { frame, palette }
    }
}

impl<Message> canvas::Program<Message> for PtyCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let cols = self.frame.cols.max(1) as usize;
        let rows = self.frame.rows.max(1) as usize;
        let cell_w = (bounds.width / cols as f32).max(1.0);
        let cell_h = (bounds.height / rows as f32).max(1.0);
        let font_size = Pixels((cell_h * 0.78).max(8.0));

        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), self.palette.bg.to_iced());

        for row in 0..rows {
            for col in 0..cols {
                let cell = self
                    .frame
                    .styled
                    .cells
                    .get(row)
                    .and_then(|line| line.get(col))
                    .copied()
                    .unwrap_or_else(blank_cell);
                let (fg, bg) = resolve_cell_colors(cell, self.palette);
                let origin = Point::new(col as f32 * cell_w, row as f32 * cell_h);

                if bg != self.palette.bg.to_iced() {
                    frame.fill_rectangle(origin, Size::new(cell_w, cell_h), bg);
                }

                if cell.ch != ' ' {
                    let baseline = Point::new(origin.x + 1.0, origin.y + cell_h * 0.82);
                    frame.fill_text(canvas::Text {
                        content: cell.ch.to_string(),
                        position: baseline,
                        color: fg,
                        size: font_size,
                        font: Font::MONOSPACE,
                        ..canvas::Text::default()
                    });
                    if cell.bold {
                        frame.fill_text(canvas::Text {
                            content: cell.ch.to_string(),
                            position: Point::new(baseline.x + 0.7, baseline.y),
                            color: fg,
                            size: font_size,
                            font: Font::MONOSPACE,
                            ..canvas::Text::default()
                        });
                    }
                    if cell.underline {
                        frame.fill_rectangle(
                            Point::new(origin.x, origin.y + cell_h - 2.0),
                            Size::new(cell_w, 1.0),
                            fg,
                        );
                    }
                }
            }
        }

        if !self.frame.styled.cursor_hidden {
            let row = self.frame.styled.cursor_row.min(self.frame.rows.saturating_sub(1)) as usize;
            let col = self.frame.styled.cursor_col.min(self.frame.cols.saturating_sub(1)) as usize;
            let cell = self
                .frame
                .styled
                .cells
                .get(row)
                .and_then(|line| line.get(col))
                .copied()
                .unwrap_or_else(blank_cell);
            let origin = Point::new(col as f32 * cell_w, row as f32 * cell_h);
            frame.fill_rectangle(
                origin,
                Size::new(cell_w, cell_h),
                self.palette.selected_bg.to_iced(),
            );
            if cell.ch != ' ' {
                frame.fill_text(canvas::Text {
                    content: cell.ch.to_string(),
                    position: Point::new(origin.x + 1.0, origin.y + cell_h * 0.82),
                    color: self.palette.selected_fg.to_iced(),
                    size: font_size,
                    font: Font::MONOSPACE,
                    ..canvas::Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

fn blank_cell() -> PtyStyledCell {
    PtyStyledCell {
        ch: ' ',
        fg: CellColor::Reset,
        bg: CellColor::Black,
        bold: false,
        italic: false,
        underline: false,
        reversed: false,
    }
}

fn resolve_cell_colors(cell: PtyStyledCell, palette: RetroColors) -> (Color, Color) {
    let mut fg = cell_color_to_iced(cell.fg, palette.fg);
    let mut bg = cell_color_to_iced(cell.bg, palette.bg);
    if cell.reversed {
        std::mem::swap(&mut fg, &mut bg);
    }
    (fg, bg)
}

fn cell_color_to_iced(color: CellColor, fallback: RetroColor) -> Color {
    match color {
        CellColor::Reset => fallback.to_iced(),
        CellColor::Black => Color::from_rgb8(0, 0, 0),
        CellColor::DarkGray => Color::from_rgb8(85, 85, 85),
        CellColor::Gray => Color::from_rgb8(170, 170, 170),
        CellColor::White => Color::from_rgb8(240, 240, 240),
        CellColor::Red | CellColor::LightRed => Color::from_rgb8(255, 90, 90),
        CellColor::Green | CellColor::LightGreen => Color::from_rgb8(111, 255, 84),
        CellColor::Yellow | CellColor::LightYellow => Color::from_rgb8(255, 191, 74),
        CellColor::Blue | CellColor::LightBlue => Color::from_rgb8(105, 180, 255),
        CellColor::Magenta | CellColor::LightMagenta => Color::from_rgb8(214, 112, 255),
        CellColor::Cyan | CellColor::LightCyan => Color::from_rgb8(110, 235, 255),
        CellColor::Rgb(r, g, b) => Color::from_rgb8(r, g, b),
        CellColor::Indexed(idx) => {
            let ramp = idx.saturating_mul(10);
            Color::from_rgb8(ramp, ramp, ramp)
        }
    }
}
