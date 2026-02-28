use super::menu::TerminalScreen;
use super::retro_ui::{current_palette, RetroScreen};
use crate::pty::{PtyLaunchOptions, PtySession, PtyStyledCell};
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use eframe::egui::{self, Align2, Color32, Context, Key, Pos2, Rect, Stroke};
use ratatui::style::Color;
use std::path::Path;
use std::time::Duration;

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

    let pty_cols = cols.max(1) as u16;
    // Keep one terminal row as safety padding above the global footer/status bar.
    let pty_rows = rows.saturating_sub(1).max(1) as u16;
    state.session.resize(pty_cols, pty_rows);
    handle_pty_input(ctx, &mut state.session);
    // Prefer output-driven repaints, but keep a regular repaint cadence so
    // animated apps (e.g. cmatrix) keep moving even when activity detection
    // is temporarily quiet.
    if state.session.take_output_activity() {
        ctx.request_repaint();
    }
    ctx.request_repaint_after(Duration::from_millis(33));

    if !state.session.is_alive() {
        return PtyScreenEvent::ProcessExited;
    }
    let smooth_borders = matches!(
        crate::config::get_settings().cli_acs_mode,
        crate::config::CliAcsMode::Unicode
    );

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, response) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);

            let plain_fast = state.session.prefers_plain_render();
            let plain_snapshot = if plain_fast {
                Some(state.session.snapshot_plain(pty_cols, pty_rows))
            } else {
                None
            };
            let plain_has_borders = plain_snapshot
                .as_ref()
                .map(|snap| snap.lines.iter().any(|line| line_has_border_glyphs(line)))
                .unwrap_or(false);
            let content_rect = Rect::from_min_max(
                screen.row_rect(0, 0, 1).min,
                Pos2::new(
                    screen.rect.right(),
                    screen.row_rect(0, pty_rows as usize, 1).min.y,
                ),
            );
            handle_pty_mouse(
                ui.ctx(),
                &response,
                content_rect,
                pty_cols,
                pty_rows,
                &mut state.session,
            );
            let content_painter = painter.with_clip_rect(content_rect);

            if plain_fast && !plain_has_borders {
                let snap = plain_snapshot.as_ref().expect("plain snapshot present");
                for (row_idx, line) in snap.lines.iter().enumerate() {
                    if !line.is_empty() {
                        content_painter.text(
                            screen.row_rect(0, row_idx, 1).left_top(),
                            Align2::LEFT_TOP,
                            line,
                            screen.font().clone(),
                            palette.fg,
                        );
                    }
                }
                let row = snap.cursor_row as usize;
                let col = snap.cursor_col as usize;
                let cursor_rect = screen.row_rect(col, row, 1);
                let ch = snap
                    .lines
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
                let snapshot = state.session.snapshot_styled(pty_cols, pty_rows);
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
        });

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

fn handle_pty_input(ctx: &Context, session: &mut PtySession) {
    let events = ctx.input(|i| i.events.clone());
    for event in events {
        match event {
            egui::Event::Paste(text) => {
                if !text.is_empty() {
                    session.send_paste(&text);
                }
            }
            egui::Event::Text(text) => {
                if !text.is_empty() {
                    session.write(text.as_bytes());
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
                }
            }
            _ => {}
        }
    }
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

fn line_has_border_glyphs(line: &str) -> bool {
    line.chars().any(|ch| {
        matches!(
            ch,
            '-' | '|'
                | '+'
                | '─'
                | '│'
                | '┌'
                | '┐'
                | '└'
                | '┘'
                | '├'
                | '┤'
                | '┬'
                | '┴'
                | '┼'
                | '═'
                | '║'
                | '╔'
                | '╗'
                | '╚'
                | '╝'
                | '╠'
                | '╣'
                | '╦'
                | '╩'
                | '╬'
        )
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
