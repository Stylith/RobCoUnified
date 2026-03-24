use super::super::retro_ui::current_palette;
use eframe::egui::{self, Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Vec2};

struct CursorSprite {
    mask: &'static [&'static str],
    hotspot: [f32; 2],
}

const ARROW_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "o..........",
        "*o.........",
        "xxo........",
        "x*xo.......",
        "xxx*o......",
        "xxxx*o.....",
        "xxxxx*o....",
        "xxxxxx*o...",
        "xxxxxxx*o..",
        "xxxxxxxxoo.",
        "xxxxoo.....",
        "x*xo*o.....",
        ".o..x*o....",
        "....oxo....",
        "....oo.....",
        ".....o.....",
    ],
    hotspot: [0.0, 0.0],
};

const IBEAM_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "o***oo.", ".oxxo..", "..xo...", "..xo...", "..xo...", "..xo...", "..xo...", "..xo...",
        "..xo...", "..xo...", ".oxxo..", "o***oo.",
    ],
    hotspot: [3.0, 0.0],
};

const POINTING_HAND_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "..*o......",
        "..xxo.....",
        "..xxo.....",
        "..xxo.....",
        "o.xxoo....",
        "*xxxxxo...",
        "xxxxxxxo..",
        "xxxxxxxo..",
        ".xxxxxxo..",
        "..xxxxo...",
        "..xxoo....",
        "..oo......",
    ],
    hotspot: [2.0, 1.0],
};

const RESIZE_HORIZONTAL_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "000010000",
        "000111000",
        "011111110",
        "111111111",
        "011111110",
        "000111000",
        "000010000",
    ],
    hotspot: [4.0, 3.0],
};

const RESIZE_VERTICAL_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "0001000", "0011100", "0111110", "0001000", "0001000", "0001000", "0111110", "0011100",
        "0001000",
    ],
    hotspot: [3.0, 4.0],
};

const RESIZE_NWSE_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "1100000", "1110000", "0111000", "0011100", "0001110", "0000111", "0000011",
    ],
    hotspot: [3.0, 3.0],
};

const RESIZE_NESW_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "0000011", "0000111", "0001110", "0011100", "0111000", "1110000", "1100000",
    ],
    hotspot: [3.0, 3.0],
};

const MOVE_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "....o....",
        "...oxo...",
        "..ooxo...",
        "ooxxxoo..",
        "oxxxxxxo.",
        "ooxxxoo..",
        "..ooxo...",
        "...oxo...",
        "....o....",
    ],
    hotspot: [4.0, 4.0],
};

const FORBIDDEN_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        ".o***o.", "o*xxx*o", "*xo.ox*", "*x...x*", "*xo.ox*", "o*xxx*o", ".o***o.",
    ],
    hotspot: [3.0, 3.0],
};

const WAIT_CURSOR: CursorSprite = CursorSprite {
    mask: &[
        "..o*o..", ".oxxxo.", "o*xxx*o", "oxxoxxo", "o*xxx*o", ".oxxxo.", "..o*o..",
    ],
    hotspot: [3.0, 3.0],
};

pub(super) fn draw_software_cursor(ctx: &Context, cursor_scale: f32) {
    if !matches!(
        egui_winit::current_app_cursor_mode(),
        egui_winit::AppCursorMode::Software
    ) {
        return;
    }

    let Some(pointer_pos) = ctx.input(|input| input.pointer.latest_pos()) else {
        return;
    };
    let Some(cursor_icon) = egui_winit::current_app_cursor_icon() else {
        return;
    };
    if matches!(cursor_icon, egui::CursorIcon::None) {
        return;
    }

    let pixels_per_point = ctx.pixels_per_point().max(1.0);
    let pixel = 1.0 / pixels_per_point;
    let unit = pixel * 2.0 * cursor_scale.clamp(0.5, 2.5);
    let sprite = sprite_for_cursor(cursor_icon);
    let palette = current_palette();
    let origin = snap_pos(
        Pos2::new(
            pointer_pos.x - sprite.hotspot[0] * unit,
            pointer_pos.y - sprite.hotspot[1] * unit,
        ),
        pixel,
    );

    let painter = ctx.layer_painter(LayerId::new(
        Order::Tooltip,
        Id::new("robco_native_software_cursor"),
    ));
    let colors = cursor_colors(palette);

    paint_shadow(
        &painter,
        sprite,
        origin + Vec2::splat(unit),
        unit,
        colors.shadow,
    );
    paint_sprite(&painter, sprite, origin, unit, colors);
}

fn sprite_for_cursor(cursor_icon: egui::CursorIcon) -> &'static CursorSprite {
    match cursor_icon {
        egui::CursorIcon::Text | egui::CursorIcon::VerticalText => &IBEAM_CURSOR,
        egui::CursorIcon::PointingHand => &POINTING_HAND_CURSOR,
        egui::CursorIcon::ResizeHorizontal
        | egui::CursorIcon::ResizeEast
        | egui::CursorIcon::ResizeWest
        | egui::CursorIcon::ResizeColumn => &RESIZE_HORIZONTAL_CURSOR,
        egui::CursorIcon::ResizeVertical
        | egui::CursorIcon::ResizeNorth
        | egui::CursorIcon::ResizeSouth
        | egui::CursorIcon::ResizeRow => &RESIZE_VERTICAL_CURSOR,
        egui::CursorIcon::ResizeNwSe
        | egui::CursorIcon::ResizeNorthWest
        | egui::CursorIcon::ResizeSouthEast => &RESIZE_NWSE_CURSOR,
        egui::CursorIcon::ResizeNeSw
        | egui::CursorIcon::ResizeNorthEast
        | egui::CursorIcon::ResizeSouthWest => &RESIZE_NESW_CURSOR,
        egui::CursorIcon::Move
        | egui::CursorIcon::AllScroll
        | egui::CursorIcon::Crosshair
        | egui::CursorIcon::Cell
        | egui::CursorIcon::Grab
        | egui::CursorIcon::Grabbing => &MOVE_CURSOR,
        egui::CursorIcon::NotAllowed | egui::CursorIcon::NoDrop => &FORBIDDEN_CURSOR,
        egui::CursorIcon::Wait | egui::CursorIcon::Progress => &WAIT_CURSOR,
        _ => &ARROW_CURSOR,
    }
}

#[derive(Clone, Copy)]
struct CursorColors {
    shadow: Color32,
    outline: Color32,
    fill: Color32,
    highlight: Color32,
}

fn paint_shadow(painter: &Painter, sprite: &CursorSprite, origin: Pos2, unit: f32, color: Color32) {
    for (row, line) in sprite.mask.iter().enumerate() {
        for (col, cell) in line.bytes().enumerate() {
            if matches!(cell, b'0' | b'.') {
                continue;
            }
            let min = Pos2::new(origin.x + col as f32 * unit, origin.y + row as f32 * unit);
            painter.rect_filled(Rect::from_min_size(min, Vec2::splat(unit)), 0.0, color);
        }
    }
}

fn paint_sprite(
    painter: &Painter,
    sprite: &CursorSprite,
    origin: Pos2,
    unit: f32,
    colors: CursorColors,
) {
    for (row, line) in sprite.mask.iter().enumerate() {
        for (col, cell) in line.bytes().enumerate() {
            let color = match cell {
                b'0' | b'.' => continue,
                b'o' => colors.outline,
                b'x' | b'1' => colors.fill,
                b'*' => colors.highlight,
                _ => colors.fill,
            };
            let min = Pos2::new(origin.x + col as f32 * unit, origin.y + row as f32 * unit);
            painter.rect_filled(Rect::from_min_size(min, Vec2::splat(unit)), 0.0, color);
        }
    }
}

fn cursor_colors(palette: super::super::retro_ui::RetroPalette) -> CursorColors {
    CursorColors {
        shadow: scale_color(palette.dim, 0.85, 230),
        outline: scale_color(palette.dim, 1.05, 255),
        fill: palette.fg,
        highlight: blend_towards_white(palette.fg, 0.38),
    }
}

fn scale_color(color: Color32, factor: f32, alpha: u8) -> Color32 {
    let [r, g, b, _] = color.to_array();
    Color32::from_rgba_unmultiplied(
        ((r as f32) * factor).clamp(0.0, 255.0) as u8,
        ((g as f32) * factor).clamp(0.0, 255.0) as u8,
        ((b as f32) * factor).clamp(0.0, 255.0) as u8,
        alpha,
    )
}

fn blend_towards_white(color: Color32, amount: f32) -> Color32 {
    let [r, g, b, a] = color.to_array();
    let blend = |channel: u8| -> u8 {
        (channel as f32 + (255.0 - channel as f32) * amount.clamp(0.0, 1.0)).round() as u8
    };
    Color32::from_rgba_unmultiplied(blend(r), blend(g), blend(b), a)
}

fn snap_pos(pos: Pos2, pixel: f32) -> Pos2 {
    Pos2::new(snap_value(pos.x, pixel), snap_value(pos.y, pixel))
}

fn snap_value(value: f32, pixel: f32) -> f32 {
    (value / pixel).round() * pixel
}
