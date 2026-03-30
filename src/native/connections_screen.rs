use super::desktop_connections_service::{
    connections_macos_disabled, connections_macos_disabled_hint,
};
use super::menu::{draw_terminal_menu_screen, paint_terminal_menu_screen};
use super::retro_ui::{ContentBounds, RetroScreen};
use crate::config::ConnectionKind;
use crate::connections::{DiscoveredConnection, NetworkMenuGroup};
use eframe::egui::{Context, Painter, Ui};
pub use nucleon_native_connections_app::{
    apply_kind_menu_activation, apply_network_groups_activation, apply_picker_activation,
    apply_saved_menu_activation, apply_search_query, build_connections_root_menu,
    build_kind_menu_model, build_network_groups_menu, build_picker_menu, build_saved_menu_model,
    resolve_connections_root_activation, resolve_terminal_connections_request, ConnectionsEvent,
    ConnectionsView, PickerMode, TerminalConnectionsRequest, TerminalConnectionsState,
};

#[allow(clippy::too_many_arguments)]
pub fn paint_connections_screen(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    state: &mut TerminalConnectionsState,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> ConnectionsEvent {
    if connections_macos_disabled() {
        return ConnectionsEvent::Status(connections_macos_disabled_hint().to_string());
    }

    match state.view.clone() {
        ConnectionsView::Root => paint_connections_root(
            ui,
            screen,
            painter,
            &mut state.root_idx,
            shell_status,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            header_lines,
        ),
        ConnectionsView::NetworkGroups => paint_network_groups(
            ui,
            screen,
            painter,
            &mut state.network_group_idx,
            shell_status,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
        ConnectionsView::Kind { kind, group } => paint_kind_menu(
            ui,
            screen,
            painter,
            kind,
            group,
            &mut state.kind_idx,
            shell_status,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
        ConnectionsView::Saved { kind, group } => paint_saved_menu(
            ui,
            screen,
            painter,
            kind,
            group,
            &mut state.saved_idx,
            shell_status,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
        ConnectionsView::Picker {
            kind,
            group,
            title,
            items,
            mode,
        } => paint_picker(
            ui,
            screen,
            painter,
            kind,
            group,
            &title,
            &items,
            mode,
            &mut state.picker_idx,
            shell_status,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_connections_screen(
    ctx: &Context,
    state: &mut TerminalConnectionsState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> ConnectionsEvent {
    if connections_macos_disabled() {
        return ConnectionsEvent::Status(connections_macos_disabled_hint().to_string());
    }

    match state.view.clone() {
        ConnectionsView::Root => draw_connections_root(
            ctx,
            &mut state.root_idx,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            header_lines,
        ),
        ConnectionsView::NetworkGroups => draw_network_groups(
            ctx,
            &mut state.network_group_idx,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
        ConnectionsView::Kind { kind, group } => draw_kind_menu(
            ctx,
            kind,
            group,
            &mut state.kind_idx,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
        ConnectionsView::Saved { kind, group } => draw_saved_menu(
            ctx,
            kind,
            group,
            &mut state.saved_idx,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
        ConnectionsView::Picker {
            kind,
            group,
            title,
            items,
            mode,
        } => draw_picker(
            ctx,
            kind,
            group,
            &title,
            &items,
            mode,
            &mut state.picker_idx,
            shell_status,
            cols,
            rows,
            header_start_row,
            separator_top_row,
            title_row,
            separator_bottom_row,
            subtitle_row,
            menu_start_row,
            status_row,
            bounds,
            &mut state.view,
            header_lines,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_terminal_connections_screen(
    ctx: &Context,
    state: &mut TerminalConnectionsState,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> TerminalConnectionsRequest {
    let event = draw_connections_screen(
        ctx,
        state,
        shell_status,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        header_lines,
    );
    resolve_terminal_connections_request(state, event, connections_macos_disabled_hint())
}

#[allow(clippy::too_many_arguments)]
fn paint_connections_root(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> ConnectionsEvent {
    let menu = build_connections_root_menu();
    let activated = paint_terminal_menu_screen(
        ui,
        screen,
        painter,
        &menu.title,
        menu.subtitle.as_deref(),
        &menu.items,
        selected_idx,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    resolve_connections_root_activation(activated)
}

#[allow(clippy::too_many_arguments)]
fn paint_network_groups(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let menu = build_network_groups_menu();
    let activated = paint_terminal_menu_screen(
        ui,
        screen,
        painter,
        &menu.title,
        menu.subtitle.as_deref(),
        &menu.items,
        selected_idx,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_network_groups_activation(&mut temp_state, activated);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn paint_kind_menu(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let model = build_kind_menu_model(kind, group);
    let activated = paint_terminal_menu_screen(
        ui,
        screen,
        painter,
        &model.menu.title,
        model.menu.subtitle.as_deref(),
        &model.menu.items,
        selected_idx,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_kind_menu_activation(&mut temp_state, kind, group, activated, &model);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn paint_saved_menu(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let model = match build_saved_menu_model(kind, group) {
        Ok(model) => model,
        Err(status) => return ConnectionsEvent::Status(status),
    };
    let activated = paint_terminal_menu_screen(
        ui,
        screen,
        painter,
        &model.menu.title,
        model.menu.subtitle.as_deref(),
        &model.menu.items,
        selected_idx,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_saved_menu_activation(&mut temp_state, kind, group, activated, &model);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn paint_picker(
    ui: &mut Ui,
    screen: &RetroScreen,
    painter: &Painter,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    title: &str,
    items: &[DiscoveredConnection],
    mode: PickerMode,
    selected_idx: &mut usize,
    shell_status: &str,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let menu = build_picker_menu(title, items);
    let activated = paint_terminal_menu_screen(
        ui,
        screen,
        painter,
        &menu.title,
        menu.subtitle.as_deref(),
        &menu.items,
        selected_idx,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_picker_activation(&mut temp_state, kind, group, items, mode, activated);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn draw_connections_root(
    ctx: &Context,
    selected_idx: &mut usize,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    header_lines: &[String],
) -> ConnectionsEvent {
    let menu = build_connections_root_menu();
    let activated = draw_terminal_menu_screen(
        ctx,
        &menu.title,
        menu.subtitle.as_deref(),
        &menu.items,
        selected_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    resolve_connections_root_activation(activated)
}

#[allow(clippy::too_many_arguments)]
fn draw_network_groups(
    ctx: &Context,
    selected_idx: &mut usize,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let menu = build_network_groups_menu();
    let activated = draw_terminal_menu_screen(
        ctx,
        &menu.title,
        menu.subtitle.as_deref(),
        &menu.items,
        selected_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_network_groups_activation(&mut temp_state, activated);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn draw_kind_menu(
    ctx: &Context,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    selected_idx: &mut usize,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let model = build_kind_menu_model(kind, group);
    let activated = draw_terminal_menu_screen(
        ctx,
        &model.menu.title,
        model.menu.subtitle.as_deref(),
        &model.menu.items,
        selected_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_kind_menu_activation(&mut temp_state, kind, group, activated, &model);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn draw_saved_menu(
    ctx: &Context,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    selected_idx: &mut usize,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let model = match build_saved_menu_model(kind, group) {
        Ok(model) => model,
        Err(status) => return ConnectionsEvent::Status(status),
    };
    let activated = draw_terminal_menu_screen(
        ctx,
        &model.menu.title,
        model.menu.subtitle.as_deref(),
        &model.menu.items,
        selected_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_saved_menu_activation(&mut temp_state, kind, group, activated, &model);
    *view = temp_state.view;
    event
}

#[allow(clippy::too_many_arguments)]
fn draw_picker(
    ctx: &Context,
    kind: ConnectionKind,
    group: Option<NetworkMenuGroup>,
    title: &str,
    items: &[DiscoveredConnection],
    mode: PickerMode,
    selected_idx: &mut usize,
    shell_status: &str,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    subtitle_row: usize,
    menu_start_row: usize,
    status_row: usize,
    bounds: &ContentBounds,
    view: &mut ConnectionsView,
    header_lines: &[String],
) -> ConnectionsEvent {
    let menu = build_picker_menu(title, items);
    let activated = draw_terminal_menu_screen(
        ctx,
        &menu.title,
        menu.subtitle.as_deref(),
        &menu.items,
        selected_idx,
        cols,
        rows,
        header_start_row,
        separator_top_row,
        title_row,
        separator_bottom_row,
        subtitle_row,
        menu_start_row,
        status_row,
        bounds,
        shell_status,
        header_lines,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_picker_activation(&mut temp_state, kind, group, items, mode, activated);
    *view = temp_state.view;
    event
}
