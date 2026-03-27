mod classic_overlay;
mod classic_screen;
mod classic_status_bar;

use super::app::RobcoNativeApp;
use super::shell_slots::SlotAction;
use crate::theme::TerminalLayoutProfile;
use eframe::egui::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalSlot {
    StatusBar,
    Screen,
    Overlay,
}

pub struct TerminalSlotContext<'a> {
    pub ctx: &'a Context,
    pub layout: &'a TerminalLayoutProfile,
}

pub trait TerminalSlotRenderer {
    fn slot(&self) -> TerminalSlot;
    fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &TerminalSlotContext) -> Vec<SlotAction>;
}

pub struct TerminalSlotRegistry {
    renderers: Vec<Box<dyn TerminalSlotRenderer>>,
}

impl TerminalSlotRegistry {
    pub fn classic() -> Self {
        TerminalSlotRegistry {
            renderers: vec![
                Box::new(classic_status_bar::ClassicTerminalStatusBarRenderer),
                Box::new(classic_screen::ClassicTerminalScreenRenderer),
                Box::new(classic_overlay::ClassicTerminalOverlayRenderer),
            ],
        }
    }

    pub fn render_slot(
        &self,
        slot: TerminalSlot,
        app: &mut RobcoNativeApp,
        ctx: &Context,
        layout: &TerminalLayoutProfile,
    ) -> Vec<SlotAction> {
        let slot_ctx = TerminalSlotContext { ctx, layout };
        let mut actions = Vec::new();
        for renderer in &self.renderers {
            if renderer.slot() == slot {
                actions.extend(renderer.render(app, &slot_ctx));
            }
        }
        actions
    }
}
