use super::super::desktop_app::DesktopWindow;
use super::super::file_manager_desktop;
use super::desktop_window_mgmt::DesktopHeaderAction;
use super::RobcoNativeApp;
use eframe::egui::{self, Context, Id};
use std::path::PathBuf;

impl RobcoNativeApp {
    pub(super) fn draw_file_manager(&mut self, ctx: &Context) {
        if !self.file_manager.open || self.desktop_window_is_minimized(DesktopWindow::FileManager) {
            return;
        }
        let wid = self.current_window_id(DesktopWindow::FileManager);
        let save_picker_mode = self.editor.save_as_input.is_some();
        let mut open = self.file_manager.open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::FileManager);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::FileManager);
        let mut header_action = DesktopHeaderAction::None;
        let generation = self.desktop_window_generation(wid);
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::FileManager);
        let min_size = Self::desktop_file_manager_window_min_size();
        let save_picker_size = egui::vec2(860.0, 560.0);
        let title = if wid.instance > 0 {
            format!("File Manager [{}]", wid.instance + 1)
        } else {
            "File Manager".to_string()
        };
        let mut window = egui::Window::new(&title)
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(Self::desktop_window_frame())
            .resizable(true)
            .min_size(min_size)
            .default_size([default_size.x, default_size.y]);
        if save_picker_mode {
            window = window.resizable(false);
            if let Some((pos, _)) = restore {
                window = window.current_pos(pos).fixed_size(save_picker_size);
            } else {
                let pos = self.active_desktop_default_window_pos(ctx, save_picker_size);
                window = window.current_pos(pos).fixed_size(save_picker_size);
            }
        } else if maximized {
            let rect = self.active_desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .resizable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, size)) = restore {
            let size = self.active_desktop_clamp_window_size(ctx, size, min_size);
            let pos = self.active_desktop_clamp_window_pos(ctx, pos, size);
            window = window.current_pos(pos).default_size(size);
        }
        self.file_manager.ensure_selection_valid();
        let rows = self.file_manager.rows();
        let action_selection_paths: Vec<PathBuf> = self
            .file_manager_selected_entries()
            .into_iter()
            .map(|entry| entry.path)
            .collect();
        let has_editable_selection = !action_selection_paths.is_empty();
        let has_single_file_selection =
            action_selection_paths.len() == 1 && action_selection_paths[0].is_file();
        let has_clipboard = self.file_manager_runtime.has_clipboard();
        let desktop_model = file_manager_desktop::build_desktop_view_model(
            &self.file_manager,
            &self.live_desktop_file_manager_settings,
            &rows,
            self.file_manager_selection_count(),
            has_editable_selection,
            has_single_file_selection,
            has_clipboard,
            self.editor.save_as_input.clone(),
            self.picking_icon_for_shortcut,
            self.picking_wallpaper,
        );
        let footer_model = file_manager_desktop::build_footer_model(&desktop_model);

        self.preload_file_manager_svg_previews(ctx, &desktop_model.rows);

        let search_id = Id::new(("native_file_manager_search", wid.instance, generation));
        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            self.draw_file_manager_top_panel(
                ctx,
                ui,
                generation,
                maximized,
                save_picker_mode,
                &desktop_model,
                &search_id,
                &mut header_action,
            );
            self.draw_file_manager_footer_panel(ui, generation, save_picker_mode, &footer_model);
            self.draw_file_manager_tree_panel(ui, generation, save_picker_mode, &desktop_model);
            let (open_with_entries, known_app_count) =
                super::file_manager_desktop_presenter::build_open_with_context_entries(
                    &self.file_manager,
                    &self.live_desktop_file_manager_settings,
                );
            self.draw_file_manager_content_panel(
                ctx,
                ui,
                generation,
                save_picker_mode,
                &desktop_model,
                &action_selection_paths,
                has_editable_selection,
                has_single_file_selection,
                has_clipboard,
                &open_with_entries,
                known_app_count,
            );
        });
        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.maybe_activate_desktop_window_from_click(
            ctx,
            DesktopWindow::FileManager,
            shown_contains_pointer,
        );
        if !maximized && !save_picker_mode {
            if let Some(rect) = shown_rect {
                self.note_desktop_window_rect(DesktopWindow::FileManager, rect);
            }
        }
        match header_action {
            DesktopHeaderAction::None => {}
            DesktopHeaderAction::Close => open = false,
            DesktopHeaderAction::Minimize => {
                self.set_desktop_window_minimized(DesktopWindow::FileManager, true)
            }
            DesktopHeaderAction::ToggleMaximize => {
                self.toggle_desktop_window_maximized(DesktopWindow::FileManager, shown_rect)
            }
        }
        if !self.file_manager.open {
            open = false;
        }
        if !open {
            if self.editor.should_close_after_save() {
                self.editor.prompt_close_confirmation();
            }
            self.editor.save_as_input = None;
            self.picking_icon_for_shortcut = None;
            self.picking_wallpaper = false;
        }
        self.update_desktop_window_state(DesktopWindow::FileManager, open);
    }
}
