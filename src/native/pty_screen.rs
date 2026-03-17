use super::menu::TerminalScreen;
use super::retro_ui::{
    current_palette, RetroPalette, RetroScreen, FIXED_PTY_CELL_H, FIXED_PTY_CELL_W,
};
use crate::pty::{PtyLaunchOptions, PtySession, PtyStyledCell};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use eframe::egui::{self, Align2, Color32, Context, FontId, Key, Pos2, Rect, Stroke};
use ratatui::style::Color;
use std::time::{Duration, Instant};

const MAX_NATIVE_PTY_COLS: usize = 240;
const MAX_NATIVE_PTY_ROWS: usize = 80;
const PERF_ALPHA: f32 = 0.18;
pub const TERMINAL_MODE_PTY_CELL_W: f32 = 11.5;
pub const TERMINAL_MODE_PTY_CELL_H: f32 = 22.0;

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
    pub completion_message: Option<String>,
    pub session: PtySession,
    pub desktop_cols_floor: Option<u16>,
    pub desktop_rows_floor: Option<u16>,
    pub desktop_live_resize: bool,
    pub fixed_cell_w: Option<f32>,
    pub fixed_cell_h: Option<f32>,
    pub fixed_font_scale: Option<f32>,
    pub fixed_font_width_divisor: Option<f32>,
    prev_plain_lines: Vec<String>,
    prev_plain_cursor: Option<(u16, u16)>,
    plain_texture: PlainTextureRenderer,
    perf: PtyPerfStats,
    pub show_perf_overlay: bool,
    idle_frames: u32,
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
        completion_message: None,
        session,
        desktop_cols_floor: None,
        desktop_rows_floor: None,
        desktop_live_resize: true,
        fixed_cell_w: None,
        fixed_cell_h: None,
        fixed_font_scale: None,
        fixed_font_width_divisor: None,
        prev_plain_lines: Vec::new(),
        prev_plain_cursor: None,
        plain_texture: PlainTextureRenderer::default(),
        perf: PtyPerfStats::default(),
        show_perf_overlay: false,
        idle_frames: 0,
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
    let mut event = PtyScreenEvent::None;
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            event = draw_embedded_pty_in_ui(ui, ctx, state, cols, rows);
        });
    event
}

pub fn draw_embedded_pty_in_ui(
    ui: &mut egui::Ui,
    ctx: &Context,
    state: &mut NativePtyState,
    cols: usize,
    rows: usize,
) -> PtyScreenEvent {
    let desired = ui.available_size();
    draw_embedded_pty_in_ui_sized(ui, ctx, state, cols, rows, desired, true)
}

/// Like `draw_embedded_pty_in_ui` but only processes keyboard input
/// when `focused` is true.  Desktop PTY windows pass the active-window
/// check here so input only flows to the focused window.
pub fn draw_embedded_pty_in_ui_focused(
    ui: &mut egui::Ui,
    ctx: &Context,
    state: &mut NativePtyState,
    cols: usize,
    rows: usize,
    focused: bool,
) -> PtyScreenEvent {
    let desired = ui.available_size();
    draw_embedded_pty_in_ui_sized(ui, ctx, state, cols, rows, desired, focused)
}

pub fn draw_embedded_pty_in_ui_sized(
    ui: &mut egui::Ui,
    ctx: &Context,
    state: &mut NativePtyState,
    cols: usize,
    rows: usize,
    desired: egui::Vec2,
    focused: bool,
) -> PtyScreenEvent {
    if focused && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q)) {
        return PtyScreenEvent::CloseRequested;
    }
    if focused && ctx.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::P)) {
        state.show_perf_overlay = !state.show_perf_overlay;
    }
    let fixed_cell_w = state.fixed_cell_w.unwrap_or(FIXED_PTY_CELL_W);
    let fixed_cell_h = state.fixed_cell_h.unwrap_or(FIXED_PTY_CELL_H);
    let fixed_font_scale = state.fixed_font_scale.unwrap_or(0.90);
    let fixed_font_width_divisor = state.fixed_font_width_divisor.unwrap_or(0.53);
    let frame_started = Instant::now();
    let show_top_bar = state.session.top_bar_label().is_some();
    let (display_cols, display_rows) = if let Some(cols_floor) = state.desktop_cols_floor {
        let cols_floor = cols_floor as usize;
        let rows_floor =
            state.desktop_rows_floor.unwrap_or(20) as usize + 1 + usize::from(show_top_bar);
        if state.desktop_live_resize {
            (
                ((desired.x / fixed_cell_w).floor() as usize)
                    .max(cols_floor)
                    .clamp(40, MAX_NATIVE_PTY_COLS),
                ((desired.y / fixed_cell_h).floor() as usize)
                    .max(rows_floor)
                    .clamp(2, MAX_NATIVE_PTY_ROWS + 1 + usize::from(show_top_bar)),
            )
        } else {
            (cols_floor, rows_floor)
        }
    } else {
        (cols, rows)
    };
    let pty_cols = display_cols.clamp(1, MAX_NATIVE_PTY_COLS) as u16;
    // Keep one row for global footer/status bar and optional PTY title band.
    let reserved_rows = 1 + usize::from(show_top_bar);
    let pty_rows = display_rows
        .saturating_sub(reserved_rows)
        .clamp(1, MAX_NATIVE_PTY_ROWS) as u16;
    let input_started = Instant::now();
    // Resize the PTY if dimensions changed.  PtySession::resize() has a
    // built-in guard (no-op if same) so this is safe to call every frame.
    state.session.resize(pty_cols, pty_rows);
    let input_activity = if focused {
        handle_pty_input(ctx, &mut state.session)
    } else {
        false
    };
    let input_ms = input_started.elapsed().as_secs_f32() * 1000.0;
    let output_activity = state.session.take_output_activity();
    // Always repaint at 60fps while PTY is alive.  Activity detection is
    // still used for perf overlay stats, but the repaint cadence is fixed
    // so we never miss frames or show half-updated ncurses screens.
    // When there's been no activity for a while, drop to a slower cadence.
    let idle_secs = if !input_activity && !output_activity {
        state.idle_frames = state.idle_frames.saturating_add(1);
        state.idle_frames as f32 / 60.0
    } else {
        state.idle_frames = 0;
        0.0
    };
    let tick_ms = if idle_secs > 2.0 { 200 } else { 16 };
    ctx.request_repaint_after(Duration::from_millis(tick_ms));

    if !state.session.is_alive() {
        return PtyScreenEvent::ProcessExited;
    }
    let smooth_borders = matches!(
        crate::config::with_settings(|settings| settings.cli_acs_mode),
        crate::config::CliAcsMode::Unicode
    );
    let mut snapshot_ms = 0.0f32;
    let mut dirty_stats = DirtyStats {
        total_rows: pty_rows as usize,
        total_cells: pty_rows as usize * pty_cols as usize,
        ..DirtyStats::default()
    };

    let draw_started = Instant::now();
    let palette = current_palette();
    ui.painter().rect_filled(ui.max_rect(), 0.0, palette.bg);
    let render_rows = pty_rows as usize + usize::from(show_top_bar);
    let (screen, response) = if state.desktop_cols_floor.is_some() {
        RetroScreen::new_fixed_cell_sized_tuned(
            ui,
            pty_cols as usize,
            render_rows,
            desired,
            fixed_cell_w,
            fixed_cell_h,
            fixed_font_scale,
            fixed_font_width_divisor,
        )
    } else {
        RetroScreen::new_sized(ui, pty_cols as usize, render_rows, desired)
    };
    let painter = ui.painter_at(screen.rect);
    let row_offset = usize::from(show_top_bar);
    let content_rect = if show_top_bar {
        let top = screen.row_rect(0, row_offset, 1).top();
        Rect::from_min_max(
            Pos2::new(screen.rect.left(), top),
            Pos2::new(screen.rect.right(), screen.rect.bottom()),
        )
    } else {
        screen.rect
    };
    if let Some(label) = state.session.top_bar_label() {
        let bar_rect = screen.row_rect(0, 0, pty_cols as usize);
        let bar_font = FontId::monospace((bar_rect.height() * 0.72).max(screen.font().size + 2.0));
        painter.rect_filled(bar_rect, 0.0, palette.selected_bg);
        painter.text(
            bar_rect.center(),
            Align2::CENTER_CENTER,
            format!(" {label} "),
            bar_font.clone(),
            palette.selected_fg,
        );
        painter.text(
            Pos2::new(bar_rect.center().x + 0.7, bar_rect.center().y),
            Align2::CENTER_CENTER,
            format!(" {label} "),
            bar_font,
            palette.selected_fg,
        );
    }

    // ── Fetch committed display frame ──────────────────────────────────
    // The reader thread builds this snapshot after each coalesced I/O
    // batch, so it's guaranteed to be consistent (no mid-update tears).
    let started = Instant::now();
    let frame = state.session.committed_frame();
    snapshot_ms += started.elapsed().as_secs_f32() * 1000.0;

    let plain_fast = state.session.prefers_plain_render();

    handle_pty_mouse(
        ui.ctx(),
        &response,
        content_rect,
        pty_cols,
        pty_rows,
        &mut state.session,
    );
    let content_painter = painter.with_clip_rect(content_rect);

    // Clamp iteration to the minimum of committed frame and display dims.
    let render_cols = (frame.cols as usize).min(pty_cols as usize);
    let render_rows_count = (frame.rows as usize).min(pty_rows as usize);

    if plain_fast {
        let snap = &frame.plain;
        let smoothed_lines = if smooth_borders && plain_lines_have_ascii_borders(&snap.lines) {
            let mut lines = snap.lines.clone();
            smooth_ascii_borders_in_plain_lines(&mut lines);
            Some(lines)
        } else {
            None
        };
        let lines_ref: &[String] = smoothed_lines.as_deref().unwrap_or(&snap.lines);
        let dirty_rows = collect_dirty_rows(&state.prev_plain_lines, lines_ref, render_rows_count);
        if state.show_perf_overlay {
            dirty_stats = diff_plain_snapshot(
                &state.prev_plain_lines,
                lines_ref,
                state.prev_plain_cursor,
                (snap.cursor_row, snap.cursor_col),
                render_cols,
                render_rows_count,
            );
        }
        let mut row_requires_cell_draw = vec![false; render_rows_count];
        for (row_idx, row) in frame
            .styled
            .cells
            .iter()
            .enumerate()
            .take(render_rows_count)
        {
            row_requires_cell_draw[row_idx] = row.iter().take(render_cols).any(|cell| {
                let (fg, _bg) = resolve_cell_colors(*cell);
                cell.ch != ' ' && (fg != palette.fg || cell.bold || cell.italic || cell.underline)
            });
        }
        let use_texture = std::env::var("ROBCOS_NATIVE_PTY_TEXTURE")
            .ok()
            .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "on" | "ON"))
            .unwrap_or(false);
        let allow_texture = !row_requires_cell_draw.iter().any(|needs| *needs);
        let texture_drawn = use_texture
            && allow_texture
            && render_plain_texture_if_possible(
                state,
                ctx,
                &content_painter,
                &screen,
                content_rect,
                &palette,
                lines_ref,
                dirty_rows.as_slice(),
            );
        let glyph_advance = screen.row_rect(0, row_offset, 1).width().max(1.0);
        let x_origin = content_rect.left();
        // Background colors from the styled cells of the committed frame.
        for (row_idx, row) in frame
            .styled
            .cells
            .iter()
            .enumerate()
            .take(render_rows_count)
        {
            for (col_idx, cell) in row.iter().enumerate().take(render_cols) {
                let (_fg, bg) = resolve_cell_colors(*cell);
                if bg == palette.bg {
                    continue;
                }
                let x = screen.snap_value(x_origin + col_idx as f32 * glyph_advance);
                let rect = screen.text_band_rect(
                    row_idx + row_offset,
                    x,
                    screen.snap_value(glyph_advance.ceil()),
                );
                content_painter.rect_filled(rect, 0.0, bg);
            }
        }
        if !texture_drawn {
            for (row_idx, line) in lines_ref.iter().enumerate().take(render_rows_count) {
                if row_requires_cell_draw[row_idx] {
                    continue;
                }
                let clipped: String = line
                    .trim_end_matches(' ')
                    .chars()
                    .take(render_cols)
                    .collect();
                if clipped.is_empty() {
                    continue;
                }
                let y = screen.row_text_top(row_idx + row_offset);
                content_painter.text(
                    Pos2::new(x_origin, y),
                    Align2::LEFT_TOP,
                    clipped,
                    screen.font().clone(),
                    palette.fg,
                );
            }
        }
        for (row_idx, row) in frame
            .styled
            .cells
            .iter()
            .enumerate()
            .take(render_rows_count)
        {
            let force_row_cells = row_requires_cell_draw[row_idx];
            for (col_idx, cell) in row.iter().enumerate().take(render_cols) {
                let (fg, _bg) = resolve_cell_colors(*cell);
                if cell.ch == ' ' {
                    continue;
                }
                if !force_row_cells
                    && fg == palette.fg
                    && !cell.bold
                    && !cell.italic
                    && !cell.underline
                {
                    continue;
                }
                let x = screen.snap_value(x_origin + col_idx as f32 * glyph_advance);
                let y = screen.row_text_top(row_idx + row_offset);
                content_painter.text(
                    Pos2::new(x, y),
                    Align2::LEFT_TOP,
                    cell.ch.to_string(),
                    screen.font().clone(),
                    fg,
                );
                if cell.bold {
                    content_painter.text(
                        Pos2::new(x + 0.7, y),
                        Align2::LEFT_TOP,
                        cell.ch.to_string(),
                        screen.font().clone(),
                        fg,
                    );
                }
                if cell.underline {
                    let rect = screen.text_band_rect(
                        row_idx + row_offset,
                        x,
                        screen.snap_value(glyph_advance.ceil()),
                    );
                    let uy = rect.bottom() - 2.0;
                    content_painter.line_segment(
                        [Pos2::new(rect.left(), uy), Pos2::new(rect.right(), uy)],
                        Stroke::new(1.0, fg),
                    );
                }
            }
        }
        state.prev_plain_lines = lines_ref[..render_rows_count.min(lines_ref.len())].to_vec();
        state.prev_plain_cursor = Some((snap.cursor_row, snap.cursor_col));
        if !snap.cursor_hidden {
            let row = snap.cursor_row as usize + row_offset;
            let col = snap.cursor_col as usize;
            let cursor_x = screen.snap_value(x_origin + col as f32 * glyph_advance);
            let cursor_rect =
                screen.text_band_rect(row, cursor_x, screen.snap_value(glyph_advance.ceil()));
            let ch = lines_ref
                .get(row.saturating_sub(row_offset))
                .and_then(|line| line.chars().nth(col))
                .unwrap_or(' ');
            content_painter.rect_filled(cursor_rect, 0.0, palette.fg);
            if ch != ' ' {
                content_painter.text(
                    Pos2::new(cursor_x, screen.row_text_top(row)),
                    Align2::LEFT_TOP,
                    ch.to_string(),
                    screen.font().clone(),
                    palette.bg,
                );
            }
        }
    } else {
        let snapshot = &frame.styled;

        if state.show_perf_overlay {
            dirty_stats.changed_rows = dirty_stats.total_rows;
            dirty_stats.changed_cells = dirty_stats.total_cells;
        }
        state.prev_plain_lines.clear();
        state.prev_plain_cursor = None;
        for (row_idx, row) in snapshot.cells.iter().enumerate().take(render_rows_count) {
            for (col_idx, cell) in row.iter().enumerate().take(render_cols) {
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
                    vector_border_connections(&snapshot.cells, row_idx, col_idx, cell_to_draw.ch)
                } else {
                    None
                };
                draw_cell(
                    &screen,
                    &content_painter,
                    col_idx,
                    row_idx + row_offset,
                    &cell_to_draw,
                    border_conn,
                );
            }
        }

        if !snapshot.cursor_hidden {
            let row = snapshot.cursor_row as usize + row_offset;
            let col = snapshot.cursor_col as usize;
            let cursor_rect = screen.row_rect(col, row, 1);
            let cell = snapshot
                .cells
                .get(row.saturating_sub(row_offset))
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
            }
        }
    }
    let draw_ms = draw_started.elapsed().as_secs_f32() * 1000.0;
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
                    if session.mouse_mode_enabled() {
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
                            session.send_mouse_event(
                                kind,
                                egui_mods_to_crossterm(modifiers),
                                col,
                                row,
                            );
                        }
                    } else {
                        let (key, amount) = if delta.y > 0.0 {
                            (KeyCode::Up, delta.y.abs())
                        } else if delta.y < 0.0 {
                            (KeyCode::Down, delta.y.abs())
                        } else if delta.x > 0.0 {
                            (KeyCode::Right, delta.x.abs())
                        } else if delta.x < 0.0 {
                            (KeyCode::Left, delta.x.abs())
                        } else {
                            continue;
                        };
                        let repeats = (amount / 24.0).round().clamp(1.0, 6.0) as usize;
                        for _ in 0..repeats {
                            session.send_key(key, egui_mods_to_crossterm(modifiers));
                        }
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

fn scale_theme_color(base: Color32, factor: f32) -> Color32 {
    let [r, g, b, a] = base.to_array();
    Color32::from_rgba_unmultiplied(
        ((r as f32) * factor).clamp(0.0, 255.0) as u8,
        ((g as f32) * factor).clamp(0.0, 255.0) as u8,
        ((b as f32) * factor).clamp(0.0, 255.0) as u8,
        a,
    )
}

fn luma01(color: Color32) -> f32 {
    let r = color.r() as f32 / 255.0;
    let g = color.g() as f32 / 255.0;
    let b = color.b() as f32 / 255.0;
    (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0)
}

fn theme_shade_from_source(source: Color32, is_background: bool) -> Color32 {
    let palette = current_palette();
    let base = palette.fg;
    let luma = luma01(source);

    if is_background {
        if luma <= 0.03 {
            return palette.bg;
        }
        let factor = 0.05 + luma * 0.30;
        return scale_theme_color(base, factor.clamp(0.05, 0.38));
    }

    if luma <= 0.10 {
        return palette.dim;
    }
    let factor = 0.40 + luma * 0.95;
    scale_theme_color(base, factor.clamp(0.42, 1.22))
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

pub fn handle_pty_input(ctx: &Context, session: &mut PtySession) -> bool {
    let mut had_input = false;
    let events = ctx.input(|i| i.events.clone());
    // Track whether any Event::Text arrived this frame.  When it does
    // (e.g. terminal mode where the CentralPanel is focused), we skip
    // the Key→char fallback to avoid double-sending characters.
    let had_text_event = events.iter().any(|e| matches!(e, egui::Event::Text(_)));
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
            // egui intercepts Ctrl+X/C as Cut/Copy before emitting Key events.
            // Convert them back to control bytes for the PTY.
            egui::Event::Cut => {
                session.write(&[0x18]); // Ctrl+X
                had_input = true;
            }
            egui::Event::Copy => {
                session.write(&[0x03]); // Ctrl+C
                had_input = true;
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
                // ── Direct byte writing for all keys ─────────────────
                // We write PTY bytes directly rather than going through
                // send_key(), which locks the parser to check application
                // cursor mode.  Only arrow keys actually need that check.

                // Ctrl+letter → control byte (0x01..0x1A)
                if modifiers.ctrl && !modifiers.alt {
                    if let Some(ch) = key_to_char(key, false) {
                        let lc = ch.to_ascii_lowercase();
                        if lc.is_ascii_lowercase() {
                            let ctrl_byte = (lc as u8) - b'a' + 1;
                            session.write(&[ctrl_byte]);
                            had_input = true;
                            continue;
                        }
                    }
                }

                // Escape → ESC byte
                if key == Key::Escape {
                    session.write(b"\x1b");
                    had_input = true;
                    continue;
                }

                // Simple control keys that don't depend on terminal mode
                match key {
                    Key::Enter => {
                        session.write(b"\r");
                        had_input = true;
                        continue;
                    }
                    Key::Tab => {
                        session.write(b"\t");
                        had_input = true;
                        continue;
                    }
                    Key::Backspace => {
                        session.write(b"\x7f");
                        had_input = true;
                        continue;
                    }
                    Key::Delete => {
                        session.write(b"\x1b[3~");
                        had_input = true;
                        continue;
                    }
                    Key::Home => {
                        session.write(b"\x1b[H");
                        had_input = true;
                        continue;
                    }
                    Key::End => {
                        session.write(b"\x1b[F");
                        had_input = true;
                        continue;
                    }
                    Key::PageUp => {
                        session.write(b"\x1b[5~");
                        had_input = true;
                        continue;
                    }
                    Key::PageDown => {
                        session.write(b"\x1b[6~");
                        had_input = true;
                        continue;
                    }
                    Key::Insert => {
                        session.write(b"\x1b[2~");
                        had_input = true;
                        continue;
                    }
                    _ => {}
                }

                // Arrow keys — these need application_cursor check via send_key
                if matches!(
                    key,
                    Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
                ) {
                    if let Some((code, mods)) = map_key_event(key, modifiers) {
                        session.send_key(code, mods);
                        had_input = true;
                    }
                    continue;
                }

                // Printable key fallback (desktop mode: no Event::Text without focus)
                if !modifiers.ctrl && !modifiers.alt && !modifiers.command {
                    if !had_text_event {
                        if let Some(ch) = key_to_char(key, modifiers.shift) {
                            let mut tmp = [0u8; 4];
                            let s = ch.encode_utf8(&mut tmp);
                            session.write(s.as_bytes());
                            had_input = true;
                        }
                    }
                    continue;
                }

                // Fallback for anything else (Alt+key, etc.)
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

/// Convert an egui Key to a printable char for the PTY fallback path.
/// Returns None for keys that aren't simple printable characters.
fn key_to_char(key: Key, shift: bool) -> Option<char> {
    use Key::*;
    match key {
        A => Some(if shift { 'A' } else { 'a' }),
        B => Some(if shift { 'B' } else { 'b' }),
        C => Some(if shift { 'C' } else { 'c' }),
        D => Some(if shift { 'D' } else { 'd' }),
        E => Some(if shift { 'E' } else { 'e' }),
        F => Some(if shift { 'F' } else { 'f' }),
        G => Some(if shift { 'G' } else { 'g' }),
        H => Some(if shift { 'H' } else { 'h' }),
        I => Some(if shift { 'I' } else { 'i' }),
        J => Some(if shift { 'J' } else { 'j' }),
        K => Some(if shift { 'K' } else { 'k' }),
        L => Some(if shift { 'L' } else { 'l' }),
        M => Some(if shift { 'M' } else { 'm' }),
        N => Some(if shift { 'N' } else { 'n' }),
        O => Some(if shift { 'O' } else { 'o' }),
        P => Some(if shift { 'P' } else { 'p' }),
        Q => Some(if shift { 'Q' } else { 'q' }),
        R => Some(if shift { 'R' } else { 'r' }),
        S => Some(if shift { 'S' } else { 's' }),
        T => Some(if shift { 'T' } else { 't' }),
        U => Some(if shift { 'U' } else { 'u' }),
        V => Some(if shift { 'V' } else { 'v' }),
        W => Some(if shift { 'W' } else { 'w' }),
        X => Some(if shift { 'X' } else { 'x' }),
        Y => Some(if shift { 'Y' } else { 'y' }),
        Z => Some(if shift { 'Z' } else { 'z' }),
        Num0 => Some(if shift { ')' } else { '0' }),
        Num1 => Some(if shift { '!' } else { '1' }),
        Num2 => Some(if shift { '@' } else { '2' }),
        Num3 => Some(if shift { '#' } else { '3' }),
        Num4 => Some(if shift { '$' } else { '4' }),
        Num5 => Some(if shift { '%' } else { '5' }),
        Num6 => Some(if shift { '^' } else { '6' }),
        Num7 => Some(if shift { '&' } else { '7' }),
        Num8 => Some(if shift { '*' } else { '8' }),
        Num9 => Some(if shift { '(' } else { '9' }),
        Space => Some(' '),
        Minus => Some(if shift { '_' } else { '-' }),
        Equals => Some(if shift { '+' } else { '=' }),
        OpenBracket => Some(if shift { '{' } else { '[' }),
        CloseBracket => Some(if shift { '}' } else { ']' }),
        Backslash => Some(if shift { '|' } else { '\\' }),
        Semicolon => Some(if shift { ':' } else { ';' }),
        Colon => Some(':'),
        Comma => Some(if shift { '<' } else { ',' }),
        Period => Some(if shift { '>' } else { '.' }),
        Slash => Some(if shift { '?' } else { '/' }),
        Backtick => Some(if shift { '~' } else { '`' }),
        _ => None,
    }
}

fn resolve_cell_colors(cell: PtyStyledCell) -> (Color32, Color32) {
    let mut fg = theme_shade_from_source(color32_from_tui(cell.fg), false);
    let mut bg = theme_shade_from_source(color32_from_tui(cell.bg), true);
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
    let cx = (rect.left() + rect.width() * 0.5).round();
    let cy = (rect.top() + rect.height() * 0.5).round();
    let overscan = 1.0;
    let thickness = if bold {
        (rect.height() * 0.18).clamp(1.5, 3.0)
    } else {
        (rect.height() * 0.14).clamp(1.0, 2.2)
    }
    .round()
    .max(1.0);
    let half = thickness * 0.5;
    if conn.left {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(rect.left() - overscan, cy - half),
                Pos2::new(cx + half, cy + half),
            ),
            0.0,
            color,
        );
    }
    if conn.right {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(cx - half, cy - half),
                Pos2::new(rect.right() + overscan, cy + half),
            ),
            0.0,
            color,
        );
    }
    if conn.up {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(cx - half, rect.top() - overscan),
                Pos2::new(cx + half, cy + half),
            ),
            0.0,
            color,
        );
    }
    if conn.down {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(cx - half, cy - half),
                Pos2::new(cx + half, rect.bottom() + overscan),
            ),
            0.0,
            color,
        );
    }
    painter.rect_filled(
        Rect::from_center_size(Pos2::new(cx, cy), egui::vec2(thickness, thickness)),
        0.0,
        color,
    );
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
    target_rect: Rect,
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
        let Some(font) = state.plain_texture.font.as_ref() else {
            return false;
        };
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
                        image, font, ch, col, row, cell_w, cell_h, font_px, palette.fg,
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
        target_rect,
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
    let Some(image) = renderer.image.as_ref() else {
        renderer.texture = None;
        return true;
    };
    renderer.texture = Some(ctx.load_texture(
        "native_pty_plain_texture",
        egui::ImageData::Color(image.clone().into()),
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
    let x0 = cell_x + 1;
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

const RETRO_TEXTURE_FONT_BYTES: &[u8] =
    include_bytes!("../../assets/fonts/FixedsysExcelsior301-Regular.ttf");

fn try_load_retro_font_bytes() -> Option<Vec<u8>> {
    if !RETRO_TEXTURE_FONT_BYTES.is_empty() {
        return Some(RETRO_TEXTURE_FONT_BYTES.to_vec());
    }

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
    let cmdline = cmd.join(" ");
    if crate::launcher::is_shell_preferred(cmd) {
        if let Some(shell_cmd) = crate::launcher::build_shell_fallback_command(cmd) {
            let shell_program = &shell_cmd[0];
            let shell_args: Vec<&str> = shell_cmd[1..].iter().map(String::as_str).collect();
            if let Ok(session) = PtySession::spawn(shell_program, &shell_args, cols, rows, options)
            {
                crate::diag::log(
                    "pty-native",
                    &format!("Using saved shell launch preference for command: {cmdline}"),
                );
                return Ok(session);
            }
        }
    }
    let program = &cmd[0];
    let args: Vec<&str> = cmd[1..].iter().map(String::as_str).collect();
    match PtySession::spawn(program, &args, cols, rows, options) {
        Ok(mut session) => {
            if crate::launcher::should_probe_fast_exit(cmd)
                && session.exited_within(crate::launcher::fast_exit_retry_window())
            {
                crate::diag::log(
                    "pty-native",
                    &format!("Direct PTY launch exited quickly; retrying via shell: {cmdline}"),
                );
                if let Some(shell_cmd) = crate::launcher::build_shell_fallback_command(cmd) {
                    let shell_program = &shell_cmd[0];
                    let shell_args: Vec<&str> = shell_cmd[1..].iter().map(String::as_str).collect();
                    return PtySession::spawn(shell_program, &shell_args, cols, rows, options)
                        .map(|session| {
                            crate::launcher::remember_shell_preferred(cmd);
                            crate::diag::log(
                                "pty-native",
                                &format!("Fast-exit shell retry succeeded for command: {cmdline}"),
                            );
                            session
                        })
                        .map_err(|shell_err| {
                            crate::diag::log(
                                "pty-native",
                                &format!(
                                    "Fast-exit shell retry failed for command '{cmdline}': {shell_err}"
                                ),
                            );
                            anyhow::anyhow!("launch exited quickly; shell retry failed: {shell_err}")
                        });
                }
            }
            Ok(session)
        }
        Err(primary_err) => {
            crate::diag::log(
                "pty-native",
                &format!("Direct PTY launch failed for '{cmdline}': {primary_err}"),
            );
            let Some(shell_cmd) = crate::launcher::build_shell_fallback_command(cmd) else {
                return Err(primary_err);
            };
            let shell_program = &shell_cmd[0];
            let shell_args: Vec<&str> = shell_cmd[1..].iter().map(String::as_str).collect();
            PtySession::spawn(shell_program, &shell_args, cols, rows, options)
                .map(|session| {
                    if crate::launcher::should_probe_fast_exit(cmd) {
                        crate::launcher::remember_shell_preferred(cmd);
                    }
                    crate::diag::log(
                        "pty-native",
                        &format!("Shell fallback launch succeeded for command: {cmdline}"),
                    );
                    session
                })
                .map_err(|shell_err| {
                    crate::diag::log(
                        "pty-native",
                        &format!(
                            "Shell fallback launch failed for command '{cmdline}': {shell_err}"
                        ),
                    );
                    anyhow::anyhow!(
                        "launch failed: {primary_err}; shell fallback failed: {shell_err}"
                    )
                })
        }
    }
}

fn rewrite_legacy_command(cmd: &[String]) -> Vec<String> {
    if cmd.is_empty() {
        return Vec::new();
    }
    let mut out = cmd.to_vec();
    if out[0] == "rtv"
        && !crate::launcher::command_exists("rtv")
        && crate::launcher::command_exists("tuir")
    {
        out[0] = "tuir".to_string();
    }
    crate::launcher::normalize_command_aliases(&out)
}
