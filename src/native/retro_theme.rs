//! Framework-agnostic retro color palette.
//!
//! `RetroColor` and `RetroColors` carry no egui or iced types — they can be
//! consumed by either renderer. Conversion helpers for iced live at the bottom
//! of this file behind `#[cfg(feature = ...)]` so the lib stays clean.

use crate::config::{current_theme_color, ThemeColor};
use std::sync::Mutex;

// ── RetroColor ────────────────────────────────────────────────────────────────

/// An sRGB colour with full alpha, independent of any UI framework.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetroColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RetroColor {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 240, g: 240, b: 240, a: 255 };
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Scale brightness by `factor` (0.0–2.0), clamping each channel.
    pub fn scaled(self, factor: f32) -> Self {
        let ch = |c: u8| ((c as f32 * factor).clamp(0.0, 255.0)) as u8;
        Self { r: ch(self.r), g: ch(self.g), b: ch(self.b), a: self.a }
    }

    /// Average brightness of the RGB channels (0–255).
    pub fn brightness(self) -> u8 {
        ((self.r as u16 + self.g as u16 + self.b as u16) / 3) as u8
    }

    /// Convert to an iced `Color`.
    pub fn to_iced(self) -> iced::Color {
        iced::Color::from_rgba8(self.r, self.g, self.b, self.a as f32 / 255.0)
    }

    /// Convert to an iced `Background`.
    pub fn to_iced_bg(self) -> iced::Background {
        iced::Background::Color(self.to_iced())
    }
}

// ── RetroColors ───────────────────────────────────────────────────────────────

/// The full computed retro palette. All colours are derived from a single
/// theme foreground colour (green, amber, white, etc.).
#[derive(Debug, Clone, Copy)]
pub struct RetroColors {
    /// Primary foreground (text, borders).
    pub fg: RetroColor,
    /// Dimmed foreground for secondary text.
    pub dim: RetroColor,
    /// Pure black background.
    pub bg: RetroColor,
    /// Very dark tinted panel background.
    pub panel: RetroColor,
    /// Selected item background (= fg).
    pub selected_bg: RetroColor,
    /// Selected item text (black or white depending on fg brightness).
    pub selected_fg: RetroColor,
    /// Hovered item background.
    pub hovered_bg: RetroColor,
    /// Pressed/active item background.
    pub active_bg: RetroColor,
    /// Text selection highlight.
    pub selection_bg: RetroColor,
}

impl RetroColors {
    fn from_theme(color: ThemeColor) -> Self {
        let fg = retro_color_from_theme(color);
        let selected_fg = if fg.brightness() > 96 {
            RetroColor::BLACK
        } else {
            RetroColor::WHITE
        };
        Self {
            fg,
            dim: fg.scaled(0.52),
            bg: RetroColor::BLACK,
            panel: fg.scaled(0.06),
            selected_bg: fg,
            selected_fg,
            hovered_bg: fg.scaled(0.18),
            active_bg: fg.scaled(0.26),
            selection_bg: fg.scaled(0.26),
        }
    }
}

fn retro_color_from_theme(color: ThemeColor) -> RetroColor {
    match color {
        ThemeColor::Black      => RetroColor::rgb(0, 0, 0),
        ThemeColor::DarkGray   => RetroColor::rgb(85, 85, 85),
        ThemeColor::Gray       => RetroColor::rgb(170, 170, 170),
        ThemeColor::White      => RetroColor::rgb(240, 240, 240),
        ThemeColor::Red | ThemeColor::LightRed         => RetroColor::rgb(255, 90, 90),
        ThemeColor::Green | ThemeColor::LightGreen     => RetroColor::rgb(111, 255, 84),
        ThemeColor::Yellow | ThemeColor::LightYellow   => RetroColor::rgb(255, 191, 74),
        ThemeColor::Blue | ThemeColor::LightBlue       => RetroColor::rgb(105, 180, 255),
        ThemeColor::Magenta | ThemeColor::LightMagenta => RetroColor::rgb(214, 112, 255),
        ThemeColor::Cyan | ThemeColor::LightCyan       => RetroColor::rgb(110, 235, 255),
        ThemeColor::Rgb(r, g, b) => RetroColor::rgb(r, g, b),
    }
}

// ── Global palette cache ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct PaletteCache {
    color: ThemeColor,
    palette: RetroColors,
}

static RETRO_PALETTE_CACHE: Mutex<Option<PaletteCache>> = Mutex::new(None);

/// Return the current palette, recomputing only when the theme color changes.
pub fn current_retro_colors() -> RetroColors {
    let color = current_theme_color();
    let mut guard = RETRO_PALETTE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(cache) = *guard {
        if cache.color == color {
            return cache.palette;
        }
    }
    let palette = RetroColors::from_theme(color);
    *guard = Some(PaletteCache { color, palette });
    palette
}

/// Invalidate the palette cache (call when settings change the theme color).
pub fn invalidate_retro_colors_cache() {
    if let Ok(mut guard) = RETRO_PALETTE_CACHE.lock() {
        *guard = None;
    }
}
