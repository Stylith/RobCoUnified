use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::NucleonNativeApp;

pub struct ClassicSpotlightRenderer;

impl SlotRenderer for ClassicSpotlightRenderer {
    fn slot(&self) -> ShellSlot {
        ShellSlot::Spotlight
    }

    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
        app.render_classic_spotlight_slot(slot_ctx.ctx);
        vec![]
    }
}
