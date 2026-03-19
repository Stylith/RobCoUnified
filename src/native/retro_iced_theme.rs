use super::retro_theme::current_retro_colors;
use iced::widget::{container, text_input};
use iced::{theme::Palette, Border, Theme};

pub fn retro_theme() -> Theme {
    let palette = current_retro_colors();

    Theme::custom(
        "RobCoOS".to_string(),
        Palette {
            background: palette.bg.to_iced(),
            text: palette.fg.to_iced(),
            primary: palette.fg.to_iced(),
            success: palette.fg.to_iced(),
            danger: iced::Color::from_rgb8(255, 90, 90),
        },
    )
}

pub fn window_background(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        ..Default::default()
    }
}

pub fn panel_background(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(current_retro_colors().panel.to_iced_bg()),
        ..Default::default()
    }
}

pub fn bordered_panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn overlay_panel(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        border: Border {
            color: palette.background.strong.color,
            width: 2.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn separator(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(iced::Background::Color(palette.background.strong.color)),
        ..Default::default()
    }
}

pub fn terminal_text_input(theme: &Theme, _status: text_input::Status) -> text_input::Style {
    let palette = current_retro_colors();
    let border = theme.extended_palette().background.strong.color;

    text_input::Style {
        background: palette.bg.to_iced_bg(),
        border: Border {
            color: border,
            width: 2.0,
            radius: 0.0.into(),
        },
        icon: theme.palette().text,
        placeholder: theme.extended_palette().secondary.base.color,
        value: theme.palette().text,
        selection: palette.selection_bg.to_iced(),
    }
}
