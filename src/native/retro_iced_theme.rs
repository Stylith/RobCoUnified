use super::retro_theme::current_retro_colors;
use iced::widget::{button, container, scrollable, text_editor, text_input};
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

fn button_style(
    background: iced::Background,
    text_color: iced::Color,
    border_color: iced::Color,
    border_width: f32,
) -> button::Style {
    button::Style {
        background: Some(background),
        text_color,
        border: Border {
            color: border_color,
            width: border_width,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn retro_button(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
        button::Status::Disabled => button_style(
            palette.bg.to_iced_bg(),
            palette.dim.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
        button::Status::Active => button_style(
            palette.bg.to_iced_bg(),
            palette.fg.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
    }
}

pub fn retro_button_selected(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Disabled => button_style(
            palette.active_bg.to_iced_bg(),
            palette.dim.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.active_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
        button::Status::Active => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
    }
}

pub fn retro_button_selected_strong(
    _theme: &Theme,
    status: button::Status,
) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.active_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.fg.to_iced(),
            2.0,
        ),
        button::Status::Disabled => button_style(
            palette.active_bg.to_iced_bg(),
            palette.dim.to_iced(),
            palette.dim.to_iced(),
            2.0,
        ),
        button::Status::Active => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.fg.to_iced(),
            2.0,
        ),
    }
}

pub fn retro_button_panel(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
        button::Status::Disabled => button_style(
            palette.panel.to_iced_bg(),
            palette.dim.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
        button::Status::Active => button_style(
            palette.panel.to_iced_bg(),
            palette.fg.to_iced(),
            palette.dim.to_iced(),
            1.0,
        ),
    }
}

pub fn retro_button_panel_active(
    _theme: &Theme,
    status: button::Status,
) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Disabled => button_style(
            palette.active_bg.to_iced_bg(),
            palette.dim.to_iced(),
            palette.fg.to_iced(),
            2.0,
        ),
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.fg.to_iced(),
            2.0,
        ),
        button::Status::Active => button_style(
            palette.active_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            palette.fg.to_iced(),
            2.0,
        ),
    }
}

pub fn retro_button_panel_minimized(
    _theme: &Theme,
    _status: button::Status,
) -> button::Style {
    let palette = current_retro_colors();
    button_style(
        palette.panel.to_iced_bg(),
        palette.dim.to_iced(),
        palette.dim.to_iced(),
        1.0,
    )
}

pub fn retro_button_flat(_theme: &Theme, status: button::Status) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            iced::Color::TRANSPARENT,
            0.0,
        ),
        button::Status::Disabled => button_style(
            palette.bg.to_iced_bg(),
            palette.dim.to_iced(),
            iced::Color::TRANSPARENT,
            0.0,
        ),
        button::Status::Active => button_style(
            palette.bg.to_iced_bg(),
            palette.fg.to_iced(),
            iced::Color::TRANSPARENT,
            0.0,
        ),
    }
}

pub fn retro_button_flat_selected(
    _theme: &Theme,
    status: button::Status,
) -> button::Style {
    let palette = current_retro_colors();
    match status {
        button::Status::Disabled => button_style(
            palette.active_bg.to_iced_bg(),
            palette.dim.to_iced(),
            iced::Color::TRANSPARENT,
            0.0,
        ),
        button::Status::Hovered | button::Status::Pressed => button_style(
            palette.active_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            iced::Color::TRANSPARENT,
            0.0,
        ),
        button::Status::Active => button_style(
            palette.selected_bg.to_iced_bg(),
            palette.selected_fg.to_iced(),
            iced::Color::TRANSPARENT,
            0.0,
        ),
    }
}

pub fn transparent_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        border: Border::default(),
        ..Default::default()
    }
}

pub fn icon_tile(_theme: &Theme) -> container::Style {
    let palette = current_retro_colors();
    container::Style {
        background: Some(palette.bg.to_iced_bg()),
        border: Border {
            color: palette.fg.to_iced(),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub fn icon_tile_selected(_theme: &Theme) -> container::Style {
    let palette = current_retro_colors();
    container::Style {
        background: Some(palette.selected_bg.to_iced_bg()),
        border: Border {
            color: palette.fg.to_iced(),
            width: 1.0,
            radius: 0.0.into(),
        },
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

pub fn retro_text_editor(_theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    let palette = current_retro_colors();
    let border_color = match status {
        text_editor::Status::Focused => palette.fg.to_iced(),
        _ => palette.dim.to_iced(),
    };

    text_editor::Style {
        background: palette.bg.to_iced_bg(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 0.0.into(),
        },
        icon: palette.fg.to_iced(),
        placeholder: palette.dim.to_iced(),
        value: palette.fg.to_iced(),
        selection: palette.selection_bg.to_iced(),
    }
}

pub fn retro_scrollable(_theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let palette = current_retro_colors();
    let scroller = match status {
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        }
        | scrollable::Status::Hovered {
            is_horizontal_scrollbar_hovered: true,
            ..
        }
        | scrollable::Status::Dragged {
            is_vertical_scrollbar_dragged: true,
            ..
        }
        | scrollable::Status::Dragged {
            is_horizontal_scrollbar_dragged: true,
            ..
        } => palette.fg.to_iced(),
        _ => palette.dim.to_iced(),
    };

    let rail = scrollable::Rail {
        background: Some(palette.bg.to_iced_bg()),
        border: Border::default(),
        scroller: scrollable::Scroller {
            color: scroller,
            border: Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
        },
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
    }
}
