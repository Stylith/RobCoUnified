use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::NucleonNativeApp;

pub struct ClassicDockRenderer;

impl SlotRenderer for ClassicDockRenderer {
    fn slot(&self) -> ShellSlot {
        ShellSlot::Dock
    }

    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
        app.render_classic_dock_slot(slot_ctx.ctx, slot_ctx.layout);
        vec![]
    }
}
