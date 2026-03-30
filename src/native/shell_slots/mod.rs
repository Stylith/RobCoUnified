mod classic_desktop;
mod classic_dock;
mod classic_launcher;
mod classic_panel;
mod classic_spotlight;

use super::app::NucleonNativeApp;
use crate::theme::LayoutProfile;
use eframe::egui::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShellSlot {
    Panel,
    Dock,
    Launcher,
    Spotlight,
    Desktop,
}

pub struct SlotContext<'a> {
    pub ctx: &'a Context,
    pub layout: &'a LayoutProfile,
}

pub enum SlotAction {}

pub trait SlotRenderer {
    fn slot(&self) -> ShellSlot;
    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction>;
}

pub struct SlotRegistry {
    renderers: Vec<Box<dyn SlotRenderer>>,
}

impl SlotRegistry {
    pub fn classic() -> Self {
        SlotRegistry {
            renderers: vec![
                Box::new(classic_panel::ClassicPanelRenderer),
                Box::new(classic_dock::ClassicDockRenderer),
                Box::new(classic_launcher::ClassicLauncherRenderer),
                Box::new(classic_spotlight::ClassicSpotlightRenderer),
                Box::new(classic_desktop::ClassicDesktopRenderer),
            ],
        }
    }

    pub fn render_slot(
        &self,
        slot: ShellSlot,
        app: &mut NucleonNativeApp,
        ctx: &Context,
        layout: &LayoutProfile,
    ) -> Vec<SlotAction> {
        let slot_ctx = SlotContext { ctx, layout };
        let mut actions = Vec::new();
        for renderer in &self.renderers {
            if renderer.slot() == slot {
                actions.extend(renderer.render(app, &slot_ctx));
            }
        }
        actions
    }
}
