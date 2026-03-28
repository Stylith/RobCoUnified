use super::{TerminalSlot, TerminalSlotContext, TerminalSlotRenderer};
use crate::native::shell_slots::SlotAction;
use crate::native::NucleonNativeApp;

pub struct ClassicTerminalStatusBarRenderer;

impl TerminalSlotRenderer for ClassicTerminalStatusBarRenderer {
    fn slot(&self) -> TerminalSlot {
        TerminalSlot::StatusBar
    }

    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &TerminalSlotContext) -> Vec<SlotAction> {
        app.render_classic_terminal_status_slot(slot_ctx.ctx, slot_ctx.layout);
        Vec::new()
    }
}
