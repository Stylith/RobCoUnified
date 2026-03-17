use super::desktop_connections_service::{
    connections_macos_disabled, connections_macos_disabled_hint,
};
use super::menu::draw_terminal_menu_screen;
use crate::config::ConnectionKind;
use crate::connections::{DiscoveredConnection, NetworkMenuGroup};
use eframe::egui::Context;
pub use robcos_native_connections_app::{
    apply_kind_menu_activation, apply_network_groups_activation, apply_picker_activation,
    apply_saved_menu_activation, apply_search_query, build_connections_root_menu,
    build_kind_menu_model, build_network_groups_menu, build_picker_menu, build_saved_menu_model,
    resolve_connections_root_activation, resolve_terminal_connections_request, ConnectionsEvent,
    ConnectionsView, PickerMode, TerminalConnectionsRequest, TerminalConnectionsState,
};

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
    content_col: usize,
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
            content_col,
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
            content_col,
            &mut state.view,
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
            content_col,
            &mut state.view,
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
            content_col,
            &mut state.view,
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
            content_col,
            &mut state.view,
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
    content_col: usize,
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
        content_col,
    );
    resolve_terminal_connections_request(state, event, connections_macos_disabled_hint())
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
    content_col: usize,
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
        content_col,
        shell_status,
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
    content_col: usize,
    view: &mut ConnectionsView,
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
        content_col,
        shell_status,
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
    content_col: usize,
    view: &mut ConnectionsView,
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
        content_col,
        shell_status,
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
    content_col: usize,
    view: &mut ConnectionsView,
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
        content_col,
        shell_status,
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
    content_col: usize,
    view: &mut ConnectionsView,
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
        content_col,
        shell_status,
    );
    let mut temp_state = TerminalConnectionsState {
        view: view.clone(),
        ..Default::default()
    };
    let event = apply_picker_activation(&mut temp_state, kind, group, items, mode, activated);
    *view = temp_state.view;
    event
}
