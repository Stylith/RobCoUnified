use crate::config::{current_theme_color, theme_color_for_settings, Settings, WallpaperSizeMode};
use crate::theme::{
    ColorStyle, ColorToken, DesktopStyle, ElementStyle, FillStyle, FullColorTheme,
    MonochromePreset, PaletteRef, ShadowStyle, TerminalDecoration, TerminalTheme,
    ThemeOptionDef, ThemeOptionValue, TextAlignment, ThemeColor,
};
use eframe::egui::{
    self, Align2, Color32, Context, FontId, Painter, Pos2, Rect, Response, Sense, Stroke,
    TextureHandle, TextureId, Ui, Vec2,
};
use ratatui::style::Color;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub const FIXED_PTY_CELL_W: f32 = 11.0;
pub const FIXED_PTY_CELL_H: f32 = 22.0;

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
    /// Window title bar (unfocused).
    pub window_chrome: Color32,
    /// Window title bar (focused).
    pub window_chrome_focused: Color32,
    /// Taskbar, menu bar, and status bar background.
    pub bar_bg: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellSurfaceKind {
    Desktop,
    Terminal,
}

#[derive(Debug, Clone, Copy)]
struct PaletteCache {
    color: Color,
    palette: RetroPalette,
}

#[derive(Debug, Clone)]
struct ActiveSurfaceColorStyle {
    style: ColorStyle,
    overrides: Option<HashMap<ColorToken, [u8; 4]>>,
}

#[derive(Debug, Clone, Default)]
struct ActiveColorStyleStore {
    desktop: Option<ActiveSurfaceColorStyle>,
    terminal: Option<ActiveSurfaceColorStyle>,
}

#[derive(Debug, Clone, Copy)]
struct ActiveTerminalWallpaper {
    texture_id: TextureId,
    size: [usize; 2],
    mode: WallpaperSizeMode,
    monochrome: bool,
}

#[derive(Debug, Clone)]
struct ActiveTerminalVisualStore {
    decoration: TerminalDecoration,
    wallpaper: Option<ActiveTerminalWallpaper>,
}

impl Default for ActiveTerminalVisualStore {
    fn default() -> Self {
        Self {
            decoration: TerminalDecoration::default(),
            wallpaper: None,
        }
    }
}

static PALETTE_CACHE: Mutex<Option<PaletteCache>> = Mutex::new(None);
static ACTIVE_COLOR_STYLES: Mutex<ActiveColorStyleStore> = Mutex::new(ActiveColorStyleStore {
    desktop: None,
    terminal: None,
});
static ACTIVE_TERMINAL_VISUALS: LazyLock<Mutex<ActiveTerminalVisualStore>> =
    LazyLock::new(|| Mutex::new(ActiveTerminalVisualStore::default()));
static ACTIVE_DESKTOP_STYLE: LazyLock<Mutex<DesktopStyle>> =
    LazyLock::new(|| Mutex::new(DesktopStyle::flat()));

thread_local! {
    static ACTIVE_TERMINAL_OPTIONS: RefCell<HashMap<String, ThemeOptionValue>> =
        RefCell::new(HashMap::new());
    static ACTIVE_TERMINAL_SCHEMA: RefCell<Vec<ThemeOptionDef>> = RefCell::new(Vec::new());
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

fn palette_for_theme_color(color: Color) -> RetroPalette {
    let fg = color32_from_theme(color);
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
        window_chrome: fg,
        window_chrome_focused: fg,
        bar_bg: fg,
    }
}

fn color32_from_token(theme: &FullColorTheme, token: ColorToken, fallback: Color32) -> Color32 {
    theme
        .tokens
        .get(&token)
        .map(|&[r, g, b, a]| Color32::from_rgba_unmultiplied(r, g, b, a))
        .unwrap_or(fallback)
}

fn palette_from_full_color_theme(theme: &FullColorTheme) -> RetroPalette {
    let fg = color32_from_token(
        theme,
        ColorToken::FgPrimary,
        Color32::from_rgb(220, 220, 220),
    );
    let bg = color32_from_token(theme, ColorToken::BgPrimary, Color32::from_rgb(18, 18, 18));
    let selected_bg = color32_from_token(theme, ColorToken::Selection, fg);
    RetroPalette {
        fg,
        dim: color32_from_token(theme, ColorToken::FgDim, scale(fg, 0.52)),
        bg,
        panel: color32_from_token(theme, ColorToken::PanelBg, scale(fg, 0.06)),
        selected_bg,
        selected_fg: color32_from_token(theme, ColorToken::SelectionFg, bg),
        hovered_bg: color32_from_token(theme, ColorToken::AccentHover, scale(fg, 0.18)),
        active_bg: color32_from_token(theme, ColorToken::AccentActive, scale(fg, 0.26)),
        selection_bg: color32_from_token(theme, ColorToken::Selection, scale(fg, 0.26)),
        window_chrome: color32_from_token(theme, ColorToken::WindowChrome, selected_bg),
        window_chrome_focused: color32_from_token(
            theme,
            ColorToken::WindowChromeFocused,
            selected_bg,
        ),
        bar_bg: color32_from_token(theme, ColorToken::StatusBar, selected_bg),
    }
}

pub fn palette_for_color_style(style: &ColorStyle) -> RetroPalette {
    palette_for_color_style_with_overrides(style, None)
}

pub fn palette_for_color_style_with_overrides(
    style: &ColorStyle,
    overrides: Option<&HashMap<ColorToken, [u8; 4]>>,
) -> RetroPalette {
    match style {
        ColorStyle::Monochrome { preset, custom_rgb } => {
            let color = monochrome_preset_to_color(*preset, *custom_rgb);
            palette_for_theme_color(color)
        }
        ColorStyle::FullColor { theme_id } => {
            let mut theme = FullColorTheme::builtin_by_id(theme_id)
                .unwrap_or_else(FullColorTheme::nucleon_dark);
            if let Some(overrides) = overrides {
                for (token, color) in overrides {
                    theme.tokens.insert(*token, *color);
                }
            }
            palette_from_full_color_theme(&theme)
        }
    }
}

fn monochrome_preset_to_color(preset: MonochromePreset, custom_rgb: Option<[u8; 3]>) -> Color {
    match preset {
        MonochromePreset::Green => Color::Rgb(111, 255, 84),
        MonochromePreset::White => Color::Rgb(240, 240, 240),
        MonochromePreset::Amber => Color::Rgb(255, 191, 74),
        MonochromePreset::Blue => Color::Rgb(105, 180, 255),
        MonochromePreset::LightBlue => Color::Rgb(110, 235, 255),
        MonochromePreset::Custom => {
            let [r, g, b] = custom_rgb.unwrap_or([111, 255, 84]);
            Color::Rgb(r, g, b)
        }
    }
}

pub fn current_palette() -> RetroPalette {
    if let Ok(guard) = ACTIVE_COLOR_STYLES.lock() {
        if let Some(style) = guard.desktop.as_ref() {
            return palette_for_color_style_with_overrides(&style.style, style.overrides.as_ref());
        }
    }
    let color = current_theme_color();
    if let Ok(mut guard) = PALETTE_CACHE.lock() {
        if let Some(cache) = *guard {
            if cache.color == color {
                return cache.palette;
            }
        }
        let palette = palette_for_theme_color(color);
        *guard = Some(PaletteCache { color, palette });
        return palette;
    }
    palette_for_theme_color(color)
}

pub fn current_palette_for_surface(surface: ShellSurfaceKind) -> RetroPalette {
    if let Ok(guard) = ACTIVE_COLOR_STYLES.lock() {
        let style = match surface {
            ShellSurfaceKind::Desktop => guard.desktop.as_ref(),
            ShellSurfaceKind::Terminal => guard.terminal.as_ref(),
        };
        if let Some(style) = style {
            return palette_for_color_style_with_overrides(&style.style, style.overrides.as_ref());
        }
    }
    current_palette()
}

pub fn set_active_color_style(
    surface: ShellSurfaceKind,
    style: ColorStyle,
    overrides: Option<HashMap<ColorToken, [u8; 4]>>,
) {
    if let Ok(mut guard) = ACTIVE_COLOR_STYLES.lock() {
        let active = Some(ActiveSurfaceColorStyle { style, overrides });
        match surface {
            ShellSurfaceKind::Desktop => guard.desktop = active,
            ShellSurfaceKind::Terminal => guard.terminal = active,
        }
    }
    if let Ok(mut guard) = PALETTE_CACHE.lock() {
        *guard = None;
    }
}

pub fn set_active_desktop_style(style: DesktopStyle) {
    if let Ok(mut guard) = ACTIVE_DESKTOP_STYLE.lock() {
        *guard = style;
    }
}

pub fn current_desktop_style() -> DesktopStyle {
    ACTIVE_DESKTOP_STYLE
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| DesktopStyle::flat())
}

pub fn set_active_terminal_theme(
    theme: &TerminalTheme,
    options: &HashMap<String, ThemeOptionValue>,
) {
    ACTIVE_TERMINAL_OPTIONS.with(|active| *active.borrow_mut() = options.clone());
    ACTIVE_TERMINAL_SCHEMA.with(|schema| *schema.borrow_mut() = theme.options_schema.clone());
}

pub fn terminal_option_bool(key: &str) -> bool {
    ACTIVE_TERMINAL_OPTIONS.with(|active| {
        ACTIVE_TERMINAL_SCHEMA.with(|schema| {
            TerminalTheme::get_bool(&active.borrow(), key, &schema.borrow())
        })
    })
}

pub fn terminal_option_string(key: &str) -> String {
    ACTIVE_TERMINAL_OPTIONS.with(|active| {
        ACTIVE_TERMINAL_SCHEMA.with(|schema| {
            TerminalTheme::get_string(&active.borrow(), key, &schema.borrow())
        })
    })
}

pub fn terminal_option_int(key: &str) -> i32 {
    ACTIVE_TERMINAL_OPTIONS.with(|active| {
        ACTIVE_TERMINAL_SCHEMA.with(|schema| {
            TerminalTheme::get_int(&active.borrow(), key, &schema.borrow())
        })
    })
}

pub fn terminal_menu_row_text(label: &str, selected: bool, indent: usize) -> String {
    let marker = terminal_option_string("selection_marker");
    let padding = " ".repeat(marker.chars().count());
    let prefix = if selected { marker.as_str() } else { padding.as_str() };
    format!("{prefix}{}{}", " ".repeat(indent), label)
}

pub fn resolve_theme_color(color: &ThemeColor, palette: &RetroPalette) -> Color32 {
    match color {
        ThemeColor::Rgba([r, g, b, a]) => Color32::from_rgba_premultiplied(*r, *g, *b, *a),
        ThemeColor::Palette(reference) => match reference {
            PaletteRef::Fg => palette.fg,
            PaletteRef::Dim => palette.dim,
            PaletteRef::Bg => palette.bg,
            PaletteRef::Panel => palette.panel,
            PaletteRef::SelectedBg => palette.selected_bg,
            PaletteRef::SelectedFg => palette.selected_fg,
            PaletteRef::HoveredBg => palette.hovered_bg,
            PaletteRef::ActiveBg => palette.active_bg,
            PaletteRef::SelectionBg => palette.selection_bg,
            PaletteRef::WindowChrome => palette.window_chrome,
            PaletteRef::WindowChromeFocused => palette.window_chrome_focused,
            PaletteRef::BarBg => palette.bar_bg,
        },
    }
}

fn shadow_from_style(shadow: &ShadowStyle, palette: &RetroPalette) -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: egui::vec2(shadow.offset_x, shadow.offset_y),
        blur: shadow.blur,
        spread: 0.0,
        color: resolve_theme_color(&shadow.color, palette),
    }
}

pub fn frame_from_element_style(style: &ElementStyle, palette: &RetroPalette) -> egui::Frame {
    let mut frame = egui::Frame::none();
    if let FillStyle::Solid { color } = &style.fill {
        frame = frame.fill(resolve_theme_color(color, palette));
    }
    if let Some(border) = &style.border {
        frame = frame.stroke(egui::Stroke::new(
            border.width,
            resolve_theme_color(&border.color, palette),
        ));
    }
    frame = frame.rounding(egui::Rounding::same(style.rounding));
    if let Some(shadow) = &style.shadow {
        frame = frame.shadow(shadow_from_style(shadow, palette));
    }
    frame
}

pub fn paint_gradient_fill(
    painter: &egui::Painter,
    rect: Rect,
    style: &ElementStyle,
    palette: &RetroPalette,
) {
    let FillStyle::LinearGradient { stops, angle } = &style.fill else {
        return;
    };
    if stops.len() < 2 {
        return;
    }
    let is_vertical = *angle == 0.0 || *angle == 180.0;
    let is_horizontal = *angle == 90.0 || *angle == 270.0;
    if !is_vertical && !is_horizontal {
        return;
    }
    let reversed = *angle == 180.0 || *angle == 270.0;
    let mut mesh = egui::Mesh::default();
    for stop in stops {
        let t = if reversed {
            1.0 - stop.position
        } else {
            stop.position
        };
        let color = resolve_theme_color(&stop.color, palette);
        if is_vertical {
            let y = rect.top() + t * rect.height();
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(rect.left(), y),
                uv: egui::epaint::WHITE_UV,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(rect.right(), y),
                uv: egui::epaint::WHITE_UV,
                color,
            });
        } else {
            let x = rect.left() + t * rect.width();
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x, rect.top()),
                uv: egui::epaint::WHITE_UV,
                color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: egui::pos2(x, rect.bottom()),
                uv: egui::epaint::WHITE_UV,
                color,
            });
        }
    }
    for i in 0..(stops.len() - 1) {
        let base = (i * 2) as u32;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 3, base, base + 3, base + 2]);
    }
    painter.add(egui::Shape::mesh(mesh));
}

pub fn element_text_color(style: &ElementStyle, palette: &RetroPalette) -> Color32 {
    style
        .text_color
        .as_ref()
        .map(|color| resolve_theme_color(color, palette))
        .unwrap_or(palette.fg)
}

pub fn desktop_style_rounding(style: &DesktopStyle) -> egui::Rounding {
    egui::Rounding::same(style.window_frame.rounding)
}

pub fn desktop_style_shadow(style: &DesktopStyle) -> egui::epaint::Shadow {
    style
        .window_frame
        .shadow
        .as_ref()
        .map(|shadow| shadow_from_style(shadow, &current_palette()))
        .unwrap_or(egui::epaint::Shadow::NONE)
}

pub fn set_active_terminal_decoration(decoration: TerminalDecoration) {
    if let Ok(mut guard) = ACTIVE_TERMINAL_VISUALS.lock() {
        guard.decoration = decoration;
    }
}

pub fn active_terminal_decoration() -> TerminalDecoration {
    ACTIVE_TERMINAL_VISUALS
        .lock()
        .map(|guard| guard.decoration.clone())
        .unwrap_or_default()
}

pub fn set_active_terminal_wallpaper(
    texture: Option<&TextureHandle>,
    mode: WallpaperSizeMode,
    monochrome: bool,
) {
    if let Ok(mut guard) = ACTIVE_TERMINAL_VISUALS.lock() {
        guard.wallpaper = texture.map(|texture| ActiveTerminalWallpaper {
            texture_id: texture.id(),
            size: texture.size(),
            mode,
            monochrome,
        });
    }
}

fn active_terminal_wallpaper() -> Option<ActiveTerminalWallpaper> {
    ACTIVE_TERMINAL_VISUALS
        .lock()
        .ok()
        .and_then(|guard| guard.wallpaper)
}

pub fn palette_for_settings(settings: &Settings) -> RetroPalette {
    palette_for_theme_color(theme_color_for_settings(settings))
}

pub struct RetroScreen {
    pub rect: Rect,
    cols: usize,
    cell: Vec2,
    font: FontId,
    pixels_per_point: f32,
}

impl RetroScreen {
    pub fn new(ui: &mut Ui, cols: usize, rows: usize) -> (Self, Response) {
        let desired = ui.available_size();
        Self::new_sized(ui, cols, rows, desired)
    }

    pub fn new_sized(ui: &mut Ui, cols: usize, rows: usize, desired: Vec2) -> (Self, Response) {
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
        let cell_w = (rect.width() / cols.max(1) as f32).floor().max(8.0);
        let cell_h = (rect.height() / rows.max(1) as f32).floor().max(12.0);
        // Terminal sizing is driven by the caller-provided grid (cols/rows).
        // Keeping glyph sizing grid-relative avoids double-scaling artifacts.
        let target_font = (cell_h * 0.80).max(8.0);
        let height_limit = (cell_h - 3.0).max(8.0);
        let width_limit = ((cell_w - 0.5).max(7.0) / 0.47).max(8.0);
        let font_size = (target_font.min(height_limit.min(width_limit))).max(8.0);
        let font_size = (font_size * pixels_per_point).round() / pixels_per_point;
        (
            Self {
                rect,
                cols,
                cell: egui::vec2(cell_w, cell_h),
                font: FontId::monospace(font_size),
                pixels_per_point,
            },
            response,
        )
    }

    pub fn new_fixed_cell_sized_tuned(
        ui: &mut Ui,
        cols: usize,
        rows: usize,
        desired: Vec2,
        cell_w: f32,
        cell_h: f32,
        font_scale: f32,
        width_divisor: f32,
    ) -> (Self, Response) {
        let (outer_rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
        let grid_w = (cols.max(1) as f32 * cell_w).min(outer_rect.width());
        let grid_h = (rows.max(1) as f32 * cell_h).min(outer_rect.height());
        let rect = Rect::from_min_size(
            outer_rect.min,
            egui::vec2(grid_w.max(cell_w), grid_h.max(cell_h)),
        );
        let target_font = (cell_h * font_scale).max(8.0);
        let height_limit = (cell_h - 3.0).max(8.0);
        let width_limit = ((cell_w - 1.0).max(7.0) / width_divisor.max(0.1)).max(8.0);
        let font_size = (target_font.min(height_limit.min(width_limit))).max(8.0);
        let font_size = (font_size * pixels_per_point).round() / pixels_per_point;
        (
            Self {
                rect,
                cols,
                cell: egui::vec2(cell_w, cell_h),
                font: FontId::monospace(font_size),
                pixels_per_point,
            },
            response,
        )
    }

    fn snap(&self, value: f32) -> f32 {
        (value * self.pixels_per_point).round() / self.pixels_per_point
    }

    fn snap_pos(&self, pos: Pos2) -> Pos2 {
        Pos2::new(self.snap(pos.x), self.snap(pos.y))
    }

    fn clip_text(&self, col: usize, text: &str) -> String {
        let max_chars = self.cols.saturating_sub(col);
        text.chars().take(max_chars).collect()
    }

    pub fn font(&self) -> &FontId {
        &self.font
    }

    fn row_top(&self, row: usize) -> f32 {
        self.snap(self.rect.top() + row as f32 * self.cell.y)
    }

    fn row_text_y(&self, row: usize) -> f32 {
        let top = self.row_top(row);
        let inset = ((self.cell.y - self.font.size).max(0.0) * 0.5).floor();
        self.snap(top + inset)
    }

    pub fn row_text_top(&self, row: usize) -> f32 {
        self.row_text_y(row)
    }

    pub fn snap_value(&self, value: f32) -> f32 {
        self.snap(value)
    }

    pub fn text_band_rect(&self, row: usize, left: f32, width: f32) -> Rect {
        let row_rect = self.row_rect(0, row, self.cols.max(1));
        let text_top = self.row_text_y(row).max(row_rect.top());
        let text_bottom = (text_top + self.font.size)
            .min(row_rect.bottom())
            .max(text_top + 1.0);
        Rect::from_min_max(
            self.snap_pos(Pos2::new(
                left.clamp(row_rect.left(), row_rect.right()),
                text_top,
            )),
            Pos2::new(
                self.snap((left + width).clamp(row_rect.left(), row_rect.right())),
                self.snap(text_bottom),
            ),
        )
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
        let snapped = self.snap_pos(pos);
        painter.text(snapped, align, text, self.font.clone(), color);
        if faux_bold {
            let dx = 1.0 / self.pixels_per_point;
            painter.text(
                self.snap_pos(Pos2::new(snapped.x + dx, snapped.y)),
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

    pub fn paint_terminal_background(&self, painter: &Painter, palette: &RetroPalette) {
        let Some(wallpaper) = active_terminal_wallpaper() else {
            self.paint_bg(painter, palette.bg);
            return;
        };

        let screen = self.rect;
        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
        let image_size = egui::vec2(wallpaper.size[0] as f32, wallpaper.size[1] as f32);
        let tint = if wallpaper.monochrome {
            palette.fg
        } else {
            Color32::WHITE
        };

        match wallpaper.mode {
            WallpaperSizeMode::FitToScreen | WallpaperSizeMode::Stretch => {
                painter.image(wallpaper.texture_id, screen, uv, tint);
            }
            WallpaperSizeMode::Centered => {
                painter.rect_filled(screen, 0.0, palette.bg);
                let origin = screen.center() - image_size * 0.5;
                painter.image(
                    wallpaper.texture_id,
                    Rect::from_min_size(origin, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::DefaultSize => {
                painter.rect_filled(screen, 0.0, palette.bg);
                painter.image(
                    wallpaper.texture_id,
                    Rect::from_min_size(screen.min, image_size),
                    uv,
                    tint,
                );
            }
            WallpaperSizeMode::Tile => {
                painter.rect_filled(screen, 0.0, palette.bg);
                let mut y = screen.top();
                while y < screen.bottom() {
                    let mut x = screen.left();
                    while x < screen.right() {
                        painter.image(
                            wallpaper.texture_id,
                            Rect::from_min_size(Pos2::new(x, y), image_size),
                            uv,
                            tint,
                        );
                        x += image_size.x.max(1.0);
                    }
                    y += image_size.y.max(1.0);
                }
            }
        }

        self.paint_bg(painter, Color32::from_black_alpha(180));
    }

    pub fn text(&self, painter: &Painter, col: usize, row: usize, text: &str, color: Color32) {
        let clipped = self.clip_text(col, text);
        let pos = self.snap_pos(Pos2::new(
            self.rect.left() + col as f32 * self.cell.x,
            self.row_text_y(row),
        ));
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
        let pos = self.snap_pos(Pos2::new(
            self.rect.left() + col as f32 * self.cell.x,
            self.row_text_y(row),
        ));
        self.paint_text(painter, pos, Align2::LEFT_TOP, &clipped, color, false);
        // Match underline width to the actual rendered glyph run so it doesn't
        // overshoot subtitle text across font/scale combinations.
        let galley = painter.layout_no_wrap(
            clipped.trim_end_matches(' ').to_string(),
            self.font.clone(),
            color,
        );
        let width = self.snap(galley.size().x);
        if width > 0.0 {
            let row_bottom = self.row_top(row) + self.cell.y - 1.0;
            let y = self.snap((pos.y + self.font.size + 1.0).min(row_bottom));
            painter.line_segment(
                [
                    self.snap_pos(Pos2::new(pos.x, y)),
                    self.snap_pos(Pos2::new(pos.x + width, y)),
                ],
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
        let pos = self.snap_pos(Pos2::new(self.rect.center().x, self.row_text_y(row)));
        self.paint_text(painter, pos, Align2::CENTER_TOP, &clipped, color, strong);
    }

    pub fn separator(&self, painter: &Painter, row: usize, palette: &RetroPalette) {
        let text = "=".repeat(self.cols.saturating_sub(6).max(1));
        self.centered_text(painter, row, &text, palette.dim, false);
    }

    pub fn themed_separator(
        &self,
        painter: &Painter,
        row: usize,
        palette: &RetroPalette,
        decoration: &TerminalDecoration,
    ) {
        if !terminal_option_bool("show_separators") {
            return;
        }
        let separator_char = terminal_option_string("separator_char");
        let repeat_len = separator_char.len().max(1);
        let char_count = self.cols.saturating_sub(6).max(1);
        let text = separator_char.repeat((char_count / repeat_len).max(1));
        match decoration.separator_alignment {
            TextAlignment::Center => self.centered_text(painter, row, &text, palette.dim, false),
            TextAlignment::Left => self.text(painter, 3, row, &text, palette.dim),
            TextAlignment::Right => {
                let start_col = self.cols.saturating_sub(text.chars().count() + 3);
                self.text(painter, start_col, row, &text, palette.dim);
            }
        }
    }

    pub fn themed_title(
        &self,
        painter: &Painter,
        row: usize,
        title: &str,
        palette: &RetroPalette,
        decoration: &TerminalDecoration,
    ) {
        match decoration.title_alignment {
            TextAlignment::Center => {
                self.centered_text(painter, row, title, palette.fg, decoration.title_bold)
            }
            TextAlignment::Left => {
                let clipped = self.clip_text(3, title);
                let pos = self.snap_pos(Pos2::new(
                    self.rect.left() + 3.0 * self.cell.x,
                    self.row_text_y(row),
                ));
                self.paint_text(
                    painter,
                    pos,
                    Align2::LEFT_TOP,
                    &clipped,
                    palette.fg,
                    decoration.title_bold,
                );
            }
            TextAlignment::Right => {
                let clipped = self.clip_text(3, title);
                let pos = self.snap_pos(Pos2::new(
                    self.rect.right() - 3.0 * self.cell.x,
                    self.row_text_y(row),
                ));
                self.paint_text(
                    painter,
                    pos,
                    Align2::RIGHT_TOP,
                    &clipped,
                    palette.fg,
                    decoration.title_bold,
                );
            }
        }
    }

    pub fn themed_subtitle(
        &self,
        painter: &Painter,
        col: usize,
        row: usize,
        subtitle: &str,
        palette: &RetroPalette,
        decoration: &TerminalDecoration,
    ) {
        let clipped = self.clip_text(col, subtitle);
        let (pos, align) = match decoration.subtitle_alignment {
            TextAlignment::Left => (
                self.snap_pos(Pos2::new(
                    self.rect.left() + col as f32 * self.cell.x,
                    self.row_text_y(row),
                )),
                Align2::LEFT_TOP,
            ),
            TextAlignment::Center => (
                self.snap_pos(Pos2::new(self.rect.center().x, self.row_text_y(row))),
                Align2::CENTER_TOP,
            ),
            TextAlignment::Right => (
                self.snap_pos(Pos2::new(
                    self.rect.right() - col as f32 * self.cell.x,
                    self.row_text_y(row),
                )),
                Align2::RIGHT_TOP,
            ),
        };
        self.paint_text(painter, pos, align, &clipped, palette.fg, false);
        if terminal_option_bool("subtitle_underlined") {
            let galley = painter.layout_no_wrap(clipped.clone(), self.font.clone(), palette.fg);
            let width = self.snap(galley.size().x);
            if width > 0.0 {
                let start_x = match decoration.subtitle_alignment {
                    TextAlignment::Left => pos.x,
                    TextAlignment::Center => pos.x - width * 0.5,
                    TextAlignment::Right => pos.x - width,
                };
                let row_bottom = self.row_top(row) + self.cell.y - 1.0;
                let y = self.snap((pos.y + self.font.size + 1.0).min(row_bottom));
                painter.line_segment(
                    [
                        self.snap_pos(Pos2::new(start_x, y)),
                        self.snap_pos(Pos2::new(start_x + width, y)),
                    ],
                    Stroke::new(2.0, palette.fg),
                );
            }
        }
    }

    pub fn row_rect(&self, col: usize, row: usize, width_chars: usize) -> Rect {
        let left = self.snap(self.rect.left() + col as f32 * self.cell.x);
        let top = self.rect.top() + row as f32 * self.cell.y;
        let nominal_right = left + width_chars.max(1) as f32 * self.cell.x;
        let right = if col.saturating_add(width_chars.max(1)) >= self.cols {
            self.rect.right()
        } else {
            nominal_right.min(self.rect.right())
        };
        Rect::from_min_max(
            self.snap_pos(Pos2::new(left.min(self.rect.right()), top)),
            self.snap_pos(Pos2::new(right.max(left), top + self.cell.y)),
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
        let measured_w = clipped.chars().count().max(1) as f32 * self.cell.x;
        let left = if terminal_option_string("menu_alignment") == "Center" {
            self.snap(self.rect.center().x - measured_w * 0.5)
        } else {
            self.snap(self.rect.left() + col as f32 * self.cell.x)
        };
        let top = self.row_top(row);
        let extend_highlight = selected && terminal_option_string("selection_style") == "Full Row";
        let right = if extend_highlight {
            self.rect.right()
        } else {
            (left + measured_w).min(self.rect.right())
        };
        let hit_rect = Rect::from_min_max(
            self.snap_pos(Pos2::new(left.min(self.rect.right()), top)),
            self.snap_pos(Pos2::new(right.max(left), top + self.cell.y)),
        );
        let paint_rect = self.text_band_rect(row, hit_rect.left(), hit_rect.width());
        if selected {
            painter.rect_filled(paint_rect, 0.0, palette.selected_bg);
        }
        self.paint_text(
            painter,
            self.snap_pos(Pos2::new(hit_rect.left(), self.row_text_y(row))),
            Align2::LEFT_TOP,
            &clipped,
            if selected {
                palette.selected_fg
            } else {
                palette.fg
            },
            false,
        );
        ui.interact(
            hit_rect,
            ui.id().with(("retro_row", row, col, text)),
            Sense::click(),
        )
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
        let rect = self.panel_rect(col, row, w, h);
        painter.rect_filled(rect, 0.0, palette.bg);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, palette.fg));
    }

    pub fn panel_rect(&self, col: usize, row: usize, w: usize, h: usize) -> Rect {
        let left = self.snap(self.rect.left() + col as f32 * self.cell.x);
        let top = self.snap(self.rect.top() + row as f32 * self.cell.y);
        let right = if col.saturating_add(w) >= self.cols {
            self.rect.right()
        } else {
            self.rect.left() + (col + w) as f32 * self.cell.x
        };
        let bottom =
            self.snap((self.rect.top() + (row + h) as f32 * self.cell.y).min(self.rect.bottom()));
        Rect::from_min_max(
            self.snap_pos(Pos2::new(
                left.min(self.rect.right()),
                top.min(self.rect.bottom()),
            )),
            self.snap_pos(Pos2::new(right.max(left), bottom.max(top))),
        )
    }
}

fn apply_visuals_with_palette(ctx: &Context, palette: RetroPalette) {
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

pub fn configure_visuals_for_palette(ctx: &Context, palette: RetroPalette) {
    apply_visuals_with_palette(ctx, palette);
}

pub fn configure_visuals(ctx: &Context) {
    configure_visuals_for_palette(ctx, current_palette());
}

pub fn configure_visuals_for_settings(ctx: &Context, settings: &Settings) {
    configure_visuals_for_palette(ctx, palette_for_settings(settings));
}

pub fn configure_visuals_for_color_style(ctx: &Context, style: &ColorStyle) {
    configure_visuals_for_palette(ctx, palette_for_color_style(style));
}
