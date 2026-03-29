use super::{TerminalSlot, TerminalSlotContext, TerminalSlotRenderer};
use crate::native::shell_slots::SlotAction;
use crate::native::NucleonNativeApp;

pub struct ClassicTerminalOverlayRenderer;

impl TerminalSlotRenderer for ClassicTerminalOverlayRenderer {
    fn slot(&self) -> TerminalSlot {
        TerminalSlot::Overlay
    }

    fn render(
        &self,
        app: &mut NucleonNativeApp,
        slot_ctx: &TerminalSlotContext,
    ) -> Vec<SlotAction> {
        app.render_classic_terminal_overlay_slot(slot_ctx.ctx);
        Vec::new()
    }
}
