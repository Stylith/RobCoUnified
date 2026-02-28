use super::menu::TerminalScreen;
use super::retro_ui::{current_palette, RetroPalette, RetroScreen};
use crate::pty::{PtyLaunchOptions, PtySession, PtyStyledCell};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use eframe::egui::{self, Align2, Color32, Context, Key, Pos2, Rect, Stroke};
use ratatui::style::Color;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_NATIVE_PTY_COLS: usize = 80;
const MAX_NATIVE_PTY_ROWS: usize = 25;
const PERF_ALPHA: f32 = 0.18;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct LineConnections {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

pub struct NativePtyState {
    pub title: String,
    pub return_screen: TerminalScreen,
    pub session: PtySession,
    prev_plain_lines: Vec<String>,
    prev_plain_cursor: Option<(u16, u16)>,
    plain_texture: PlainTextureRenderer,
    plain_row_galleys: Vec<Option<Arc<egui::Galley>>>,
    plain_cache_font_size: f32,
    plain_cache_fg: Color32,
    perf: PtyPerfStats,
    show_perf_overlay: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct DirtyStats {
    changed_rows: usize,
    changed_cells: usize,
    total_rows: usize,
    total_cells: usize,
}

impl DirtyStats {
    fn changed_pct(self) -> f32 {
        if self.total_cells == 0 {
            0.0
        } else {
            (self.changed_cells as f32 / self.total_cells as f32) * 100.0
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PtyPerfStats {
    frame_avg_ms: f32,
    input_avg_ms: f32,
    snapshot_avg_ms: f32,
    draw_avg_ms: f32,
    frames: u64,
    last_dirty: DirtyStats,
}

impl PtyPerfStats {
    fn push_sample(
        &mut self,
        frame_ms: f32,
        input_ms: f32,
        snapshot_ms: f32,
        draw_ms: f32,
        dirty: DirtyStats,
    ) {
        self.frame_avg_ms = ewma(self.frame_avg_ms, frame_ms, self.frames);
        self.input_avg_ms = ewma(self.input_avg_ms, input_ms, self.frames);
        self.snapshot_avg_ms = ewma(self.snapshot_avg_ms, snapshot_ms, self.frames);
        self.draw_avg_ms = ewma(self.draw_avg_ms, draw_ms, self.frames);
        self.frames = self.frames.saturating_add(1);
        self.last_dirty = dirty;
    }
}

fn ewma(current: f32, sample: f32, frames: u64) -> f32 {
    if frames == 0 {
        return sample;
    }
    current + PERF_ALPHA * (sample - current)
}

struct PlainTextureRenderer {
    font: Option<fontdue::Font>,
    texture: Option<egui::TextureHandle>,
    image: Option<egui::ColorImage>,
    cols: usize,
    rows: usize,
    cell_w_px: usize,
    cell_h_px: usize,
    font_px: f32,
}

impl Default for PlainTextureRenderer {
    fn default() -> Self {
        Self {
            font: load_plain_texture_font(),
            texture: None,
            image: None,
            cols: 0,
            rows: 0,
            cell_w_px: 0,
            cell_h_px: 0,
            font_px: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyScreenEvent {
    None,
    CloseRequested,
    ProcessExited,
}

#[allow(dead_code)]
pub fn spawn_embedded_pty(
    title: &str,
    cmd: &[String],
    return_screen: TerminalScreen,
    cols: u16,
    rows: u16,
) -> Result<NativePtyState, String> {
    spawn_embedded_pty_with_options(
        title,
        cmd,
        return_screen,
        cols,
        rows,
        PtyLaunchOptions::default(),
    )
}

pub fn spawn_embedded_pty_with_options(
    title: &str,
    cmd: &[String],
    return_screen: TerminalScreen,
    cols: u16,
    rows: u16,
    options: PtyLaunchOptions,
) -> Result<NativePtyState, String> {
    if cmd.is_empty() {
        return Err("Error: empty command.".to_string());
    }
    let cmd = rewrite_legacy_command(cmd);
    let session = spawn_with_fallback(&cmd, cols.max(1), rows.max(1), &options)
        .map_err(|err| format!("Launch failed: {err}"))?;
    Ok(NativePtyState {
        title: title.to_string(),
        return_screen,
        session,
        prev_plain_lines: Vec::new(),
        prev_plain_cursor: None,
        plain_texture: PlainTextureRenderer::default(),
        plain_row_galleys: Vec::new(),
        plain_cache_font_size: 0.0,
        plain_cache_fg: Color32::TRANSPARENT,
        perf: PtyPerfStats::default(),
        show_perf_overlay: false,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn draw_embedded_pty(
    ctx: &Context,
    state: &mut NativePtyState,
    _shell_status: &str,
    cols: usize,
    rows: usize,
    _header_start_row: usize,
    _separator_top_row: usize,
    _title_row: usize,
    _separator_bottom_row: usize,
    _subtitle_row: usize,
    _content_start_row: usize,
    _status_row: usize,
    _content_col: usize,
) -> PtyScreenEvent {
    if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
        return PtyScreenEvent::CloseRequested;
    }
    if ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::P)) {
        state.show_perf_overlay = !state.show_perf_overlay;
    }
    let frame_started = Instant::now();

    let pty_cols = cols.min(MAX_NATIVE_PTY_COLS).max(1) as u16;
    // Keep one terminal row as safety padding above the global footer/status bar.
    let pty_rows = rows.saturating_sub(1).min(MAX_NATIVE_PTY_ROWS).max(1) as u16;
    let input_started = Instant::now();
    state.session.resize(pty_cols, pty_rows);
    let input_activity = handle_pty_input(ctx, &mut state.session);
    let input_ms = input_started.elapsed().as_secs_f32() * 1000.0;
    // Prefer output-driven repaints, but keep a regular repaint cadence so
    // animated apps (e.g. cmatrix) keep moving even when activity detection
    // is temporarily quiet.
    let output_activity = state.session.take_output_activity();
    if input_activity || output_activity {
        ctx.request_repaint();
    }
    let tick_ms = if input_activity || output_activity {
        8
    } else {
        33
    };
    ctx.request_repaint_after(Duration::from_millis(tick_ms));

    if !state.session.is_alive() {
        return PtyScreenEvent::ProcessExited;
    }
    let smooth_borders = matches!(
        crate::config::get_settings().cli_acs_mode,
        crate::config::CliAcsMode::Unicode
    );
    let mut snapshot_ms = 0.0f32;
    let mut draw_ms = 0.0f32;
    let mut dirty_stats = DirtyStats {
        total_rows: pty_rows as usize,
        total_cells: pty_rows as usize * pty_cols as usize,
        ..DirtyStats::default()
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let draw_started = Instant::now();
            let palette = current_palette();
            let (screen, response) = RetroScreen::new(ui, pty_cols as usize, pty_rows as usize);
            let painter = ui.painter_at(screen.rect);

            let plain_fast = state.session.prefers_plain_render();
            let plain_snapshot = if plain_fast {
                let started = Instant::now();
                let snap = state.session.snapshot_plain(pty_cols, pty_rows);
                snapshot_ms += started.elapsed().as_secs_f32() * 1000.0;
                Some(snap)
            } else {
                None
            };
            let content_rect = screen.rect;
            handle_pty_mouse(
                ui.ctx(),
                &response,
                content_rect,
                pty_cols,
                pty_rows,
                &mut state.session,
            );
            let content_painter = painter.with_clip_rect(content_rect);

            if plain_fast {
                let snap = plain_snapshot.as_ref().expect("plain snapshot present");
                let smoothed_lines =
                    if smooth_borders && plain_lines_have_ascii_borders(&snap.lines) {
                        let mut lines = snap.lines.clone();
                        smooth_ascii_borders_in_plain_lines(&mut lines);
                        Some(lines)
                    } else {
                        None
                    };
                let lines_ref: &[String] = smoothed_lines.as_deref().unwrap_or(&snap.lines);
                let dirty_rows =
                    collect_dirty_rows(&state.prev_plain_lines, lines_ref, pty_rows as usize);
                if state.show_perf_overlay {
                    dirty_stats = diff_plain_snapshot(
                        &state.prev_plain_lines,
                        lines_ref,
                        state.prev_plain_cursor,
                        (snap.cursor_row, snap.cursor_col),
                        pty_cols as usize,
                        pty_rows as usize,
                    );
                }
                let texture_drawn = render_plain_texture_if_possible(
                    state,
                    ctx,
                    &content_painter,
                    &screen,
                    &palette,
                    lines_ref,
                    dirty_rows.as_slice(),
                );
                if !texture_drawn {
                    ensure_plain_row_galleys(
                        state,
                        &content_painter,
                        screen.font(),
                        palette.fg,
                        lines_ref,
                    );
                    for (row_idx, galley) in state.plain_row_galleys.iter().enumerate() {
                        if let Some(galley) = galley {
                            content_painter.galley(
                                screen.row_rect(0, row_idx, 1).left_top(),
                                galley.clone(),
                                palette.fg,
                            );
                        }
                    }
                }
                state.prev_plain_lines = lines_ref.to_vec();
                state.prev_plain_cursor = Some((snap.cursor_row, snap.cursor_col));
                let row = snap.cursor_row as usize;
                let col = snap.cursor_col as usize;
                let cursor_rect = screen.row_rect(col, row, 1);
                let ch = lines_ref
                    .get(row)
                    .and_then(|line| line.chars().nth(col))
                    .unwrap_or(' ');
                content_painter.rect_filled(cursor_rect, 0.0, palette.fg);
                if ch != ' ' {
                    content_painter.text(
                        cursor_rect.left_top(),
                        Align2::LEFT_TOP,
                        ch.to_string(),
                        screen.font().clone(),
                        palette.bg,
                    );
                } else {
                    content_painter.text(
                        cursor_rect.left_top(),
                        Align2::LEFT_TOP,
                        "_",
                        screen.font().clone(),
                        palette.bg,
                    );
                }
            } else {
                let started = Instant::now();
                let snapshot = state.session.snapshot_styled(pty_cols, pty_rows);
                snapshot_ms += started.elapsed().as_secs_f32() * 1000.0;
                if state.show_perf_overlay {
                    dirty_stats.changed_rows = dirty_stats.total_rows;
                    dirty_stats.changed_cells = dirty_stats.total_cells;
                }
                state.prev_plain_lines.clear();
                state.prev_plain_cursor = None;
                state.plain_row_galleys.clear();
                for (row_idx, row) in snapshot.cells.iter().enumerate() {
                    for (col_idx, cell) in row.iter().enumerate() {
                        let mut cell_to_draw = *cell;
                        if smooth_borders {
                            cell_to_draw.ch = smooth_border_char_from_snapshot(
                                &snapshot.cells,
                                row_idx,
                                col_idx,
                                cell.ch,
                            );
                        }
                        let border_conn = if smooth_borders {
                            vector_border_connections(
                                &snapshot.cells,
                                row_idx,
                                col_idx,
                                cell_to_draw.ch,
                            )
                        } else {
                            None
                        };
                        draw_cell(
                            &screen,
                            &content_painter,
                            col_idx,
                            row_idx,
                            &cell_to_draw,
                            border_conn,
                        );
                    }
                }

                if !snapshot.cursor_hidden {
                    let row = snapshot.cursor_row as usize;
                    let col = snapshot.cursor_col as usize;
                    let cursor_rect = screen.row_rect(col, row, 1);
                    let cell = snapshot
                        .cells
                        .get(snapshot.cursor_row as usize)
                        .and_then(|line| line.get(snapshot.cursor_col as usize))
                        .copied()
                        .unwrap_or(PtyStyledCell {
                            ch: ' ',
                            fg: Color::White,
                            bg: Color::Black,
                            bold: false,
                            italic: false,
                            underline: false,
                            reversed: false,
                        });
                    let (cursor_fg, cursor_bg) = resolve_cell_colors(cell);
                    let fill = cursor_fg;
                    let text_color = cursor_bg;
                    content_painter.rect_filled(cursor_rect, 0.0, fill);
                    if cell.ch != ' ' {
                        content_painter.text(
                            cursor_rect.left_top(),
                            Align2::LEFT_TOP,
                            cell.ch.to_string(),
                            screen.font().clone(),
                            text_color,
                        );
                    } else {
                        content_painter.text(
                            cursor_rect.left_top(),
                            Align2::LEFT_TOP,
                            "_",
                            screen.font().clone(),
                            text_color,
                        );
                    }
                }
            }
            draw_ms = draw_started.elapsed().as_secs_f32() * 1000.0;
            if state.show_perf_overlay {
                draw_perf_overlay(
                    &screen,
                    &content_painter,
                    &palette,
                    &state.perf,
                    dirty_stats,
                    input_activity,
                    output_activity,
                    plain_fast,
                );
            }
        });

    let frame_ms = frame_started.elapsed().as_secs_f32() * 1000.0;
    state
        .perf
        .push_sample(frame_ms, input_ms, snapshot_ms, draw_ms, dirty_stats);

    PtyScreenEvent::None
}

fn draw_cell(
    screen: &RetroScreen,
    painter: &egui::Painter,
    col: usize,
    row: usize,
    cell: &PtyStyledCell,
    border_conn: Option<LineConnections>,
) {
    let rect = screen.row_rect(col, row, 1);
    let (fg, bg) = resolve_cell_colors(*cell);
    if bg != Color32::BLACK {
        painter.rect_filled(rect, 0.0, bg);
    }
    if let Some(conn) = border_conn {
        draw_vector_border_cell(painter, rect, conn, fg, cell.bold);
    } else if cell.ch != ' ' {
        let italic_x = if cell.italic { 0.25 } else { 0.0 };
        painter.text(
            Pos2::new(rect.left() + italic_x, rect.top()),
            Align2::LEFT_TOP,
            cell.ch.to_string(),
            screen.font().clone(),
            fg,
        );
        if cell.bold {
            painter.text(
                Pos2::new(rect.left() + 0.7, rect.top()),
                Align2::LEFT_TOP,
                cell.ch.to_string(),
                screen.font().clone(),
                fg,
            );
        }
    }
    if cell.underline {
        let y = rect.bottom() - 2.0;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            egui::Stroke::new(1.0, fg),
        );
    }
}

fn handle_pty_mouse(
    ctx: &Context,
    response: &egui::Response,
    content_rect: Rect,
    pty_cols: u16,
    pty_rows: u16,
    session: &mut PtySession,
) {
    if !response.hovered() {
        return;
    }
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        match event {
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } if content_rect.contains(pos) => {
                if let Some((col, row)) = pointer_to_pty_cell(content_rect, pty_cols, pty_rows, pos)
                {
                    if let Some(btn) = map_mouse_button(button) {
                        let kind = if pressed {
                            MouseEventKind::Down(btn)
                        } else {
                            MouseEventKind::Up(btn)
                        };
                        session.send_mouse_event(kind, egui_mods_to_crossterm(modifiers), col, row);
                    }
                }
            }
            egui::Event::PointerMoved(pos) if content_rect.contains(pos) => {
                if let Some((col, row)) = pointer_to_pty_cell(content_rect, pty_cols, pty_rows, pos)
                {
                    let mods = ctx.input(|i| egui_mods_to_crossterm(i.modifiers));
                    let pointer = ctx.input(|i| i.pointer.clone());
                    let kind = if pointer.button_down(egui::PointerButton::Primary) {
                        MouseEventKind::Drag(MouseButton::Left)
                    } else if pointer.button_down(egui::PointerButton::Secondary) {
                        MouseEventKind::Drag(MouseButton::Right)
                    } else if pointer.button_down(egui::PointerButton::Middle) {
                        MouseEventKind::Drag(MouseButton::Middle)
                    } else {
                        MouseEventKind::Moved
                    };
                    session.send_mouse_event(kind, mods, col, row);
                }
            }
            egui::Event::MouseWheel {
                delta, modifiers, ..
            } => {
                let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
                    continue;
                };
                if !content_rect.contains(pos) {
                    continue;
                }
                if let Some((col, row)) = pointer_to_pty_cell(content_rect, pty_cols, pty_rows, pos)
                {
                    let kind = if delta.y < 0.0 {
                        Some(MouseEventKind::ScrollDown)
                    } else if delta.y > 0.0 {
                        Some(MouseEventKind::ScrollUp)
                    } else if delta.x < 0.0 {
                        Some(MouseEventKind::ScrollRight)
                    } else if delta.x > 0.0 {
                        Some(MouseEventKind::ScrollLeft)
                    } else {
                        None
                    };
                    if let Some(kind) = kind {
                        session.send_mouse_event(kind, egui_mods_to_crossterm(modifiers), col, row);
                    }
                }
            }
            _ => {}
        }
    }
}

fn pointer_to_pty_cell(
    content_rect: Rect,
    pty_cols: u16,
    pty_rows: u16,
    pos: Pos2,
) -> Option<(u16, u16)> {
    if !content_rect.contains(pos) {
        return None;
    }
    let cell_w = (content_rect.width() / pty_cols.max(1) as f32).max(1.0);
    let cell_h = (content_rect.height() / pty_rows.max(1) as f32).max(1.0);
    let col = ((pos.x - content_rect.left()) / cell_w).floor().max(0.0) as u16;
    let row = ((pos.y - content_rect.top()) / cell_h).floor().max(0.0) as u16;
    Some((
        col.min(pty_cols.saturating_sub(1)),
        row.min(pty_rows.saturating_sub(1)),
    ))
}

fn map_mouse_button(button: egui::PointerButton) -> Option<MouseButton> {
    match button {
        egui::PointerButton::Primary => Some(MouseButton::Left),
        egui::PointerButton::Secondary => Some(MouseButton::Right),
        egui::PointerButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

fn egui_mods_to_crossterm(modifiers: egui::Modifiers) -> KeyModifiers {
    let mut mods = KeyModifiers::empty();
    if modifiers.ctrl {
        mods |= KeyModifiers::CONTROL;
    }
    if modifiers.alt {
        mods |= KeyModifiers::ALT;
    }
    if modifiers.shift {
        mods |= KeyModifiers::SHIFT;
    }
    mods
}

fn color32_from_tui(color: Color) -> Color32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_to_pty_cell_maps_and_clamps_coordinates() {
        let rect = Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(210.0, 120.0));
        assert_eq!(
            pointer_to_pty_cell(rect, 20, 10, Pos2::new(10.0, 20.0)),
            Some((0, 0))
        );
        assert_eq!(
            pointer_to_pty_cell(rect, 20, 10, Pos2::new(209.9, 119.9)),
            Some((19, 9))
        );
        assert_eq!(
            pointer_to_pty_cell(rect, 20, 10, Pos2::new(111.0, 70.0)),
            Some((10, 5))
        );
    }

    #[test]
    fn pointer_to_pty_cell_rejects_outside_points() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 50.0));
        assert_eq!(
            pointer_to_pty_cell(rect, 10, 5, Pos2::new(-1.0, 10.0)),
            None
        );
        assert_eq!(
            pointer_to_pty_cell(rect, 10, 5, Pos2::new(20.0, 51.0)),
            None
        );
    }

    #[test]
    fn resolve_cell_colors_swaps_on_reversed_attribute() {
        let normal = PtyStyledCell {
            ch: 'x',
            fg: Color::Green,
            bg: Color::Black,
            bold: false,
            italic: false,
            underline: false,
            reversed: false,
        };
        let reversed = PtyStyledCell {
            reversed: true,
            ..normal
        };
        let (nfg, nbg) = resolve_cell_colors(normal);
        let (rfg, rbg) = resolve_cell_colors(reversed);
        assert_eq!(nfg, rbg);
        assert_eq!(nbg, rfg);
    }
}

fn handle_pty_input(ctx: &Context, session: &mut PtySession) -> bool {
    let mut had_input = false;
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        match event {
            egui::Event::Paste(text) => {
                if !text.is_empty() {
                    session.send_paste(&text);
                    had_input = true;
                }
            }
            egui::Event::Text(text) => {
                if !text.is_empty() {
                    session.write(text.as_bytes());
                    had_input = true;
                }
            }
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.ctrl && key == Key::Q {
                    continue;
                }
                if (modifiers.command && key == Key::V) || (modifiers.shift && key == Key::Insert) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                    continue;
                }
                if let Some((code, mods)) = map_key_event(key, modifiers) {
                    session.send_key(code, mods);
                    had_input = true;
                }
            }
            _ => {}
        }
    }
    had_input
}

fn resolve_cell_colors(cell: PtyStyledCell) -> (Color32, Color32) {
    let mut fg = color32_from_tui(cell.fg);
    let mut bg = color32_from_tui(cell.bg);
    if cell.reversed {
        std::mem::swap(&mut fg, &mut bg);
    }
    (fg, bg)
}

fn smooth_border_char_from_snapshot(
    cells: &[Vec<PtyStyledCell>],
    row: usize,
    col: usize,
    ch: char,
) -> char {
    if !is_border_like(ch) {
        return ch;
    }
    let up = snapshot_char(cells, row as isize - 1, col as isize);
    let down = snapshot_char(cells, row as isize + 1, col as isize);
    let left = snapshot_char(cells, row as isize, col as isize - 1);
    let right = snapshot_char(cells, row as isize, col as isize + 1);
    let conn = LineConnections {
        up: line_connections(up).down,
        down: line_connections(down).up,
        left: line_connections(left).right,
        right: line_connections(right).left,
    };
    map_connections_to_unicode(conn).unwrap_or(ch)
}

fn vector_border_connections(
    cells: &[Vec<PtyStyledCell>],
    row: usize,
    col: usize,
    ch: char,
) -> Option<LineConnections> {
    let ch = smooth_border_char_from_snapshot(cells, row, col, ch);
    let conn = line_connections(ch);
    if conn == LineConnections::default() {
        return None;
    }
    // Avoid treating ordinary punctuation as borders unless it's connected.
    if matches!(ch, '-' | '|' | '+') {
        let up = snapshot_char(cells, row as isize - 1, col as isize);
        let down = snapshot_char(cells, row as isize + 1, col as isize);
        let left = snapshot_char(cells, row as isize, col as isize - 1);
        let right = snapshot_char(cells, row as isize, col as isize + 1);
        let neighbor_conn = LineConnections {
            up: line_connections(up).down,
            down: line_connections(down).up,
            left: line_connections(left).right,
            right: line_connections(right).left,
        };
        if neighbor_conn == LineConnections::default() {
            return None;
        }
    }
    Some(conn)
}

fn draw_vector_border_cell(
    painter: &egui::Painter,
    rect: Rect,
    conn: LineConnections,
    color: Color32,
    bold: bool,
) {
    let cx = rect.center().x;
    let cy = rect.center().y;
    let overscan = 0.7;
    let thickness = if bold {
        (rect.height() * 0.18).clamp(1.5, 3.0)
    } else {
        (rect.height() * 0.14).clamp(1.0, 2.2)
    };
    let stroke = Stroke::new(thickness, color);
    if conn.left {
        painter.line_segment(
            [Pos2::new(rect.left() - overscan, cy), Pos2::new(cx, cy)],
            stroke,
        );
    }
    if conn.right {
        painter.line_segment(
            [Pos2::new(cx, cy), Pos2::new(rect.right() + overscan, cy)],
            stroke,
        );
    }
    if conn.up {
        painter.line_segment(
            [Pos2::new(cx, rect.top() - overscan), Pos2::new(cx, cy)],
            stroke,
        );
    }
    if conn.down {
        painter.line_segment(
            [Pos2::new(cx, cy), Pos2::new(cx, rect.bottom() + overscan)],
            stroke,
        );
    }
    // Fill joint center to avoid tiny anti-aliased cracks.
    painter.circle_filled(Pos2::new(cx, cy), thickness * 0.45, color);
}

fn snapshot_char(cells: &[Vec<PtyStyledCell>], row: isize, col: isize) -> char {
    if row < 0 || col < 0 {
        return ' ';
    }
    cells
        .get(row as usize)
        .and_then(|r| r.get(col as usize))
        .map(|c| c.ch)
        .unwrap_or(' ')
}

fn is_border_like(ch: char) -> bool {
    line_connections(ch) != LineConnections::default()
}

fn line_connections(ch: char) -> LineConnections {
    match ch {
        '-' | '─' | '═' => LineConnections {
            left: true,
            right: true,
            ..LineConnections::default()
        },
        '|' | '│' | '║' => LineConnections {
            up: true,
            down: true,
            ..LineConnections::default()
        },
        '+' | '┼' | '╬' => LineConnections {
            up: true,
            down: true,
            left: true,
            right: true,
        },
        '┌' | '╔' | 'Ú' => LineConnections {
            down: true,
            right: true,
            ..LineConnections::default()
        },
        '┐' | '╗' | '¿' => LineConnections {
            down: true,
            left: true,
            ..LineConnections::default()
        },
        '└' | '╚' | 'À' => LineConnections {
            up: true,
            right: true,
            ..LineConnections::default()
        },
        '┘' | '╝' | 'Ù' => LineConnections {
            up: true,
            left: true,
            ..LineConnections::default()
        },
        '├' | '╠' | 'Ã' => LineConnections {
            up: true,
            down: true,
            right: true,
            ..LineConnections::default()
        },
        '┤' | '╣' | '´' => LineConnections {
            up: true,
            down: true,
            left: true,
            ..LineConnections::default()
        },
        '┬' | '╦' | 'Â' => LineConnections {
            down: true,
            left: true,
            right: true,
            ..LineConnections::default()
        },
        '┴' | '╩' | 'Á' => LineConnections {
            up: true,
            left: true,
            right: true,
            ..LineConnections::default()
        },
        '³' => LineConnections {
            up: true,
            down: true,
            ..LineConnections::default()
        },
        'Ä' => LineConnections {
            left: true,
            right: true,
            ..LineConnections::default()
        },
        'Å' => LineConnections {
            up: true,
            down: true,
            left: true,
            right: true,
        },
        _ => LineConnections::default(),
    }
}

fn map_connections_to_unicode(c: LineConnections) -> Option<char> {
    Some(match (c.up, c.down, c.left, c.right) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, false, true, true) => '┴',
        (false, true, true, true) => '┬',
        (true, true, false, false) => '│',
        (false, false, true, true) => '─',
        (false, true, false, true) => '┌',
        (false, true, true, false) => '┐',
        (true, false, false, true) => '└',
        (true, false, true, false) => '┘',
        (true, false, false, false) | (false, true, false, false) => '│',
        (false, false, true, false) | (false, false, false, true) => '─',
        _ => return None,
    })
}

fn map_key_event(key: Key, modifiers: egui::Modifiers) -> Option<(KeyCode, KeyModifiers)> {
    let mut mods = KeyModifiers::empty();
    if modifiers.ctrl {
        mods |= KeyModifiers::CONTROL;
    }
    if modifiers.alt {
        mods |= KeyModifiers::ALT;
    }
    if modifiers.shift {
        mods |= KeyModifiers::SHIFT;
    }

    let code = match key {
        Key::ArrowUp => KeyCode::Up,
        Key::ArrowDown => KeyCode::Down,
        Key::ArrowLeft => KeyCode::Left,
        Key::ArrowRight => KeyCode::Right,
        Key::Escape => KeyCode::Esc,
        Key::Tab => KeyCode::Tab,
        Key::Backspace => KeyCode::Backspace,
        Key::Enter => KeyCode::Enter,
        Key::Home => KeyCode::Home,
        Key::End => KeyCode::End,
        Key::Insert => KeyCode::Insert,
        Key::Delete => KeyCode::Delete,
        Key::PageUp => KeyCode::PageUp,
        Key::PageDown => KeyCode::PageDown,
        Key::Space => KeyCode::Char(' '),
        Key::A => KeyCode::Char('a'),
        Key::B => KeyCode::Char('b'),
        Key::C => KeyCode::Char('c'),
        Key::D => KeyCode::Char('d'),
        Key::E => KeyCode::Char('e'),
        Key::F => KeyCode::Char('f'),
        Key::G => KeyCode::Char('g'),
        Key::H => KeyCode::Char('h'),
        Key::I => KeyCode::Char('i'),
        Key::J => KeyCode::Char('j'),
        Key::K => KeyCode::Char('k'),
        Key::L => KeyCode::Char('l'),
        Key::M => KeyCode::Char('m'),
        Key::N => KeyCode::Char('n'),
        Key::O => KeyCode::Char('o'),
        Key::P => KeyCode::Char('p'),
        Key::Q => KeyCode::Char('q'),
        Key::R => KeyCode::Char('r'),
        Key::S => KeyCode::Char('s'),
        Key::T => KeyCode::Char('t'),
        Key::U => KeyCode::Char('u'),
        Key::V => KeyCode::Char('v'),
        Key::W => KeyCode::Char('w'),
        Key::X => KeyCode::Char('x'),
        Key::Y => KeyCode::Char('y'),
        Key::Z => KeyCode::Char('z'),
        Key::Num0 => KeyCode::Char('0'),
        Key::Num1 => KeyCode::Char('1'),
        Key::Num2 => KeyCode::Char('2'),
        Key::Num3 => KeyCode::Char('3'),
        Key::Num4 => KeyCode::Char('4'),
        Key::Num5 => KeyCode::Char('5'),
        Key::Num6 => KeyCode::Char('6'),
        Key::Num7 => KeyCode::Char('7'),
        Key::Num8 => KeyCode::Char('8'),
        Key::Num9 => KeyCode::Char('9'),
        _ => return None,
    };
    Some((code, mods))
}

fn collect_dirty_rows(prev_lines: &[String], next_lines: &[String], rows: usize) -> Vec<usize> {
    (0..rows)
        .filter(|row| {
            prev_lines.get(*row).map(String::as_str).unwrap_or("")
                != next_lines.get(*row).map(String::as_str).unwrap_or("")
        })
        .collect()
}

fn render_plain_texture_if_possible(
    state: &mut NativePtyState,
    ctx: &Context,
    painter: &egui::Painter,
    screen: &RetroScreen,
    palette: &RetroPalette,
    lines: &[String],
    dirty_rows: &[usize],
) -> bool {
    if state.plain_texture.font.is_none() {
        return false;
    }
    let cols = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(screen_cols(screen));
    let rows = lines.len();
    if rows == 0 {
        return false;
    }
    let cell_rect = screen.row_rect(0, 0, 1);
    let cell_w = cell_rect.width().round().max(1.0) as usize;
    let cell_h = cell_rect.height().round().max(1.0) as usize;
    if cell_w == 0 || cell_h == 0 {
        return false;
    }
    let font_px = (screen.font().size * 0.92).max(8.0);
    let resized = ensure_plain_texture_surface(
        &mut state.plain_texture,
        ctx,
        cols,
        rows,
        cell_w,
        cell_h,
        font_px,
    );
    let update_rows: Vec<usize> = if resized || state.plain_texture.texture.is_none() {
        (0..rows).collect()
    } else {
        dirty_rows.to_vec()
    };
    if !update_rows.is_empty() {
        let mut missing_font = false;
        if let Some(image) = state.plain_texture.image.as_mut() {
            for row in update_rows {
                clear_texture_row(image, row, cell_h, palette.bg);
                let line = lines.get(row).map(String::as_str).unwrap_or("");
                let max_cols = cols.min(line.chars().count());
                for (col, ch) in line.chars().take(max_cols).enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    if !draw_glyph_to_image(
                        image,
                        state.plain_texture.font.as_ref().expect("checked"),
                        ch,
                        col,
                        row,
                        cell_w,
                        cell_h,
                        font_px,
                        palette.fg,
                    ) {
                        missing_font = true;
                    }
                }
            }
        }
        if missing_font {
            return false;
        }
        if let Some(image) = state.plain_texture.image.as_ref() {
            if let Some(texture) = state.plain_texture.texture.as_mut() {
                texture.set(
                    egui::ImageData::Color(image.clone().into()),
                    egui::TextureOptions::NEAREST,
                );
            } else {
                state.plain_texture.texture = Some(ctx.load_texture(
                    "native_pty_plain_texture",
                    egui::ImageData::Color(image.clone().into()),
                    egui::TextureOptions::NEAREST,
                ));
            }
        }
    }
    let Some(texture) = state.plain_texture.texture.as_ref() else {
        return false;
    };
    painter.image(
        texture.id(),
        screen.rect,
        Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
        Color32::WHITE,
    );
    true
}

fn ensure_plain_texture_surface(
    renderer: &mut PlainTextureRenderer,
    ctx: &Context,
    cols: usize,
    rows: usize,
    cell_w_px: usize,
    cell_h_px: usize,
    font_px: f32,
) -> bool {
    let needs_resize = renderer.image.is_none()
        || renderer.cols != cols
        || renderer.rows != rows
        || renderer.cell_w_px != cell_w_px
        || renderer.cell_h_px != cell_h_px
        || (renderer.font_px - font_px).abs() > f32::EPSILON;
    if !needs_resize {
        return false;
    }
    renderer.cols = cols;
    renderer.rows = rows;
    renderer.cell_w_px = cell_w_px;
    renderer.cell_h_px = cell_h_px;
    renderer.font_px = font_px;
    let size = [
        cols.saturating_mul(cell_w_px),
        rows.saturating_mul(cell_h_px),
    ];
    renderer.image = Some(egui::ColorImage::new(size, Color32::BLACK));
    renderer.texture = Some(ctx.load_texture(
        "native_pty_plain_texture",
        egui::ImageData::Color(renderer.image.as_ref().expect("set").clone().into()),
        egui::TextureOptions::NEAREST,
    ));
    true
}

fn clear_texture_row(image: &mut egui::ColorImage, row: usize, cell_h: usize, bg: Color32) {
    let width = image.size[0];
    let start_y = row.saturating_mul(cell_h);
    let end_y = (start_y + cell_h).min(image.size[1]);
    for y in start_y..end_y {
        let start = y.saturating_mul(width);
        let end = start.saturating_add(width).min(image.pixels.len());
        for px in &mut image.pixels[start..end] {
            *px = bg;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_glyph_to_image(
    image: &mut egui::ColorImage,
    font: &fontdue::Font,
    ch: char,
    col: usize,
    row: usize,
    cell_w: usize,
    cell_h: usize,
    font_px: f32,
    fg: Color32,
) -> bool {
    let (metrics, bitmap) = font.rasterize(ch, font_px);
    if metrics.width == 0 || metrics.height == 0 {
        return true;
    }
    let glyph_w = metrics.width as i32;
    let glyph_h = metrics.height as i32;
    let cell_x = (col.saturating_mul(cell_w)) as i32;
    let cell_y = (row.saturating_mul(cell_h)) as i32;
    let x0 = cell_x + ((cell_w as i32 - glyph_w).max(0) / 2);
    let y0 = cell_y + ((cell_h as i32 - glyph_h).max(0) / 2);
    let width = image.size[0] as i32;
    let height = image.size[1] as i32;
    for gy in 0..glyph_h {
        for gx in 0..glyph_w {
            let sx = x0 + gx;
            let sy = y0 + gy;
            if sx < 0 || sy < 0 || sx >= width || sy >= height {
                continue;
            }
            let src_alpha = bitmap[(gy as usize) * metrics.width + gx as usize];
            if src_alpha == 0 {
                continue;
            }
            let idx = (sy as usize) * image.size[0] + sx as usize;
            let bg = image.pixels[idx];
            image.pixels[idx] = blend_color(bg, fg, src_alpha);
        }
    }
    true
}

fn blend_color(bg: Color32, fg: Color32, alpha: u8) -> Color32 {
    let a = alpha as f32 / 255.0;
    let inv = 1.0 - a;
    Color32::from_rgba_unmultiplied(
        (bg.r() as f32 * inv + fg.r() as f32 * a).round() as u8,
        (bg.g() as f32 * inv + fg.g() as f32 * a).round() as u8,
        (bg.b() as f32 * inv + fg.b() as f32 * a).round() as u8,
        255,
    )
}

fn screen_cols(screen: &RetroScreen) -> usize {
    // Derive a conservative width from the render surface when line data is empty.
    let cell = screen.row_rect(0, 0, 1);
    (screen.rect.width() / cell.width().max(1.0))
        .floor()
        .max(1.0) as usize
}

fn load_plain_texture_font() -> Option<fontdue::Font> {
    let bytes = try_load_retro_font_bytes()?;
    fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
}

fn try_load_retro_font_bytes() -> Option<Vec<u8>> {
    let mut candidates = vec![
        std::path::PathBuf::from("assets/fonts/FixedsysExcelsior301-Regular.ttf"),
        std::path::PathBuf::from("assets/fonts/Sysfixed.ttf"),
        std::path::PathBuf::from("assets/fonts/sysfixed.ttf"),
        std::path::PathBuf::from("Sysfixed.ttf"),
        std::path::PathBuf::from("sysfixed.ttf"),
    ];
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("Library/Fonts/Sysfixed.ttf"));
        candidates.push(home.join("Library/Fonts/sysfixed.ttf"));
    }
    candidates.push(std::path::PathBuf::from("/Library/Fonts/Sysfixed.ttf"));
    candidates.push(std::path::PathBuf::from("/Library/Fonts/sysfixed.ttf"));
    candidates.push(std::path::PathBuf::from("/System/Library/Fonts/Monaco.ttf"));
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}

fn ensure_plain_row_galleys(
    state: &mut NativePtyState,
    painter: &egui::Painter,
    font: &egui::FontId,
    fg: Color32,
    lines: &[String],
) {
    if state.plain_row_galleys.len() != lines.len() {
        state.plain_row_galleys.resize(lines.len(), None);
    }
    let style_changed = (state.plain_cache_font_size - font.size).abs() > f32::EPSILON
        || state.plain_cache_fg != fg;
    if style_changed {
        for galley in &mut state.plain_row_galleys {
            *galley = None;
        }
        state.plain_cache_font_size = font.size;
        state.plain_cache_fg = fg;
    }
    for (row_idx, line) in lines.iter().enumerate() {
        let text_changed = state
            .prev_plain_lines
            .get(row_idx)
            .map(String::as_str)
            .unwrap_or("")
            != line.as_str();
        if line.is_empty() {
            state.plain_row_galleys[row_idx] = None;
            continue;
        }
        if style_changed || text_changed || state.plain_row_galleys[row_idx].is_none() {
            state.plain_row_galleys[row_idx] =
                Some(painter.layout_no_wrap(line.clone(), font.clone(), fg));
        }
    }
}

fn diff_plain_snapshot(
    prev_lines: &[String],
    next_lines: &[String],
    prev_cursor: Option<(u16, u16)>,
    next_cursor: (u16, u16),
    cols: usize,
    rows: usize,
) -> DirtyStats {
    let mut changed_rows = vec![false; rows];
    let mut changed_cells = 0usize;
    for row in 0..rows {
        let prev = prev_lines.get(row).map(String::as_str).unwrap_or("");
        let next = next_lines.get(row).map(String::as_str).unwrap_or("");
        let row_changed_cells = diff_plain_row_cells(prev, next, cols);
        if row_changed_cells > 0 {
            changed_rows[row] = true;
            changed_cells = changed_cells.saturating_add(row_changed_cells);
        }
    }
    if prev_cursor != Some(next_cursor) {
        if let Some((r, _)) = prev_cursor {
            let r = r as usize;
            if r < rows {
                changed_rows[r] = true;
                changed_cells = changed_cells.saturating_add(1);
            }
        }
        let r = next_cursor.0 as usize;
        if r < rows {
            changed_rows[r] = true;
            changed_cells = changed_cells.saturating_add(1);
        }
    }
    DirtyStats {
        changed_rows: changed_rows.into_iter().filter(|changed| *changed).count(),
        changed_cells: changed_cells.min(rows.saturating_mul(cols)),
        total_rows: rows,
        total_cells: rows.saturating_mul(cols),
    }
}

fn diff_plain_row_cells(prev: &str, next: &str, cols: usize) -> usize {
    let mut changed = 0usize;
    let mut prev_chars = prev.chars();
    let mut next_chars = next.chars();
    for _ in 0..cols {
        if prev_chars.next().unwrap_or(' ') != next_chars.next().unwrap_or(' ') {
            changed = changed.saturating_add(1);
        }
    }
    changed
}

fn draw_perf_overlay(
    screen: &RetroScreen,
    painter: &egui::Painter,
    palette: &RetroPalette,
    perf: &PtyPerfStats,
    dirty: DirtyStats,
    input_activity: bool,
    output_activity: bool,
    plain_fast: bool,
) {
    let cell = screen.row_rect(0, 0, 1);
    let top_left = Pos2::new(cell.left() + 4.0, cell.top() + 4.0);
    let overlay_rect = Rect::from_min_size(
        top_left,
        egui::vec2(cell.width() * 58.0, (cell.height() * 4.0).max(44.0)),
    );
    painter.rect_filled(overlay_rect, 2.0, Color32::from_black_alpha(210));
    let lines = [
        format!(
            "PTY PERF [{}]  (Ctrl+Shift+P)",
            if plain_fast { "plain" } else { "styled" }
        ),
        format!(
            "avg frame {:>5.1}ms  draw {:>5.1}ms  snap {:>5.1}ms  input {:>4.1}ms",
            perf.frame_avg_ms, perf.draw_avg_ms, perf.snapshot_avg_ms, perf.input_avg_ms
        ),
        format!(
            "dirty {:>5.1}%  cells {}/{}  rows {}/{}",
            dirty.changed_pct(),
            dirty.changed_cells,
            dirty.total_cells,
            dirty.changed_rows,
            dirty.total_rows
        ),
        format!(
            "activity in:{} out:{}  samples:{}",
            if input_activity { "Y" } else { "N" },
            if output_activity { "Y" } else { "N" },
            perf.frames
        ),
    ];
    let mut y = top_left.y + 2.0;
    for line in lines {
        painter.text(
            Pos2::new(top_left.x + 6.0, y),
            Align2::LEFT_TOP,
            line,
            screen.font().clone(),
            palette.fg,
        );
        y += cell.height();
    }
}

fn smooth_ascii_borders_in_plain_lines(lines: &mut [String]) {
    let max_cols = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if max_cols == 0 || lines.is_empty() {
        return;
    }
    let mut grid: Vec<Vec<char>> = lines
        .iter()
        .map(|line| {
            let mut row: Vec<char> = line.chars().collect();
            row.resize(max_cols, ' ');
            row
        })
        .collect();
    let source = grid.clone();
    for row in 0..source.len() {
        for col in 0..max_cols {
            let ch = source[row][col];
            if !matches!(ch, '+' | '-' | '|') {
                continue;
            }
            let left = if col > 0 { source[row][col - 1] } else { ' ' };
            let right = if col + 1 < max_cols {
                source[row][col + 1]
            } else {
                ' '
            };
            let up = if row > 0 { source[row - 1][col] } else { ' ' };
            let down = if row + 1 < source.len() {
                source[row + 1][col]
            } else {
                ' '
            };
            let conn = LineConnections {
                up: line_connections(up).down,
                down: line_connections(down).up,
                left: line_connections(left).right,
                right: line_connections(right).left,
            };
            grid[row][col] = map_connections_to_unicode(conn).unwrap_or(ch);
        }
    }
    for (row_idx, line) in lines.iter_mut().enumerate() {
        let len = line.chars().count();
        *line = grid[row_idx].iter().take(len).collect();
    }
}

fn plain_lines_have_ascii_borders(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.as_bytes()
            .iter()
            .any(|b| matches!(*b, b'+' | b'-' | b'|'))
    })
}

fn spawn_with_fallback(
    cmd: &[String],
    cols: u16,
    rows: u16,
    options: &PtyLaunchOptions,
) -> anyhow::Result<PtySession> {
    let program = &cmd[0];
    let args: Vec<&str> = cmd[1..].iter().map(String::as_str).collect();
    match PtySession::spawn(program, &args, cols, rows, options) {
        Ok(session) => Ok(session),
        Err(primary_err) => {
            let Some(shell_cmd) = build_shell_fallback_command(cmd) else {
                return Err(primary_err);
            };
            let shell_program = &shell_cmd[0];
            let shell_args: Vec<&str> = shell_cmd[1..].iter().map(String::as_str).collect();
            PtySession::spawn(shell_program, &shell_args, cols, rows, options).map_err(
                |shell_err| {
                    anyhow::anyhow!(
                        "launch failed: {primary_err}; shell fallback failed: {shell_err}"
                    )
                },
            )
        }
    }
}

fn rewrite_legacy_command(cmd: &[String]) -> Vec<String> {
    if cmd.is_empty() {
        return Vec::new();
    }
    let mut out = cmd.to_vec();
    if out[0] == "rtv" && !command_exists("rtv") && command_exists("tuir") {
        out[0] = "tuir".to_string();
    }
    out
}

fn command_exists(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if name.contains('/') {
        return Path::new(name).is_file();
    }
    std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

fn build_shell_fallback_command(cmd: &[String]) -> Option<Vec<String>> {
    if cmd.is_empty() || cmd[0].contains('/') {
        return None;
    }
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string());
    let line = cmd
        .iter()
        .map(|part| shell_quote(part))
        .collect::<Vec<_>>()
        .join(" ");
    Some(vec![shell, "-ic".to_string(), line])
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:=".contains(ch))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
