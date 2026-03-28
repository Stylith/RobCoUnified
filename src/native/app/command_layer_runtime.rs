use super::super::command_layer::{
    build_command_layer_sections, draw_command_layer, CommandLayerSection, CommandLayerTarget,
};
use super::super::desktop_app::{DesktopMenuAction, DesktopMenuBuildContext};
use super::super::editor_app::EditorTextCommand;
use super::NucleonNativeApp;
use eframe::egui::{self, Context, Id};

impl NucleonNativeApp {
    pub(super) fn open_command_layer(&mut self, target: CommandLayerTarget) {
        let sections = self.command_layer_sections(target);
        self.command_layer.open_for_target(target, &sections);
    }

    pub(super) fn command_layer_sections(
        &self,
        target: CommandLayerTarget,
    ) -> Vec<CommandLayerSection> {
        let context = DesktopMenuBuildContext {
            editor: &self.editor,
            editor_recent_files: &self.settings.draft.editor_recent_files,
            file_manager: &self.file_manager,
            file_manager_runtime: &self.file_manager_runtime,
            file_manager_settings: &self.live_desktop_file_manager_settings,
        };
        build_command_layer_sections(target, &context)
    }

    pub(super) fn draw_command_layer_at(
        &mut self,
        ctx: &Context,
        target: CommandLayerTarget,
        bar_pos: egui::Pos2,
        block_rect: egui::Rect,
    ) {
        if !self.command_layer_open_for(target) {
            return;
        }
        let sections = self.command_layer_sections(target);
        if let Some(action) =
            draw_command_layer(ctx, &mut self.command_layer, &sections, block_rect, bar_pos)
        {
            self.apply_command_layer_action(ctx, action);
        }
    }

    pub(super) fn command_layer_open_for(&self, target: CommandLayerTarget) -> bool {
        self.command_layer.open && self.command_layer.target == target
    }

    pub(super) fn terminal_command_layer_bar_pos(&self, ctx: &Context) -> egui::Pos2 {
        let viewport = ctx.screen_rect();
        egui::pos2(viewport.left() + 12.0, viewport.top() + 42.0)
    }

    pub(super) fn apply_command_layer_action(&mut self, ctx: &Context, action: DesktopMenuAction) {
        match action {
            DesktopMenuAction::EditorTextCommand(command) => {
                self.apply_editor_command_layer_text_action(ctx, command);
            }
            action => self.apply_desktop_menu_action(ctx, &action),
        }
    }

    fn apply_editor_command_layer_text_action(
        &mut self,
        ctx: &Context,
        command: EditorTextCommand,
    ) {
        let text_edit_id = if self.desktop_mode_open {
            self.active_editor_text_edit_id()
        } else {
            Id::new("terminal_editor_text_edit")
        };
        self.run_editor_text_command(ctx, text_edit_id, command);
    }
}
