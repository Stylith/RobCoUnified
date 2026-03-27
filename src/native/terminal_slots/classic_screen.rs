use super::{TerminalSlot, TerminalSlotContext, TerminalSlotRenderer};
use crate::native::shell_slots::SlotAction;
use crate::native::RobcoNativeApp;

pub struct ClassicTerminalScreenRenderer;

impl TerminalSlotRenderer for ClassicTerminalScreenRenderer {
    fn slot(&self) -> TerminalSlot {
        TerminalSlot::Screen
    }

    fn render(
        &self,
        app: &mut RobcoNativeApp,
        slot_ctx: &TerminalSlotContext,
    ) -> Vec<SlotAction> {
        app.render_classic_terminal_screen_slot(slot_ctx.ctx);
        Vec::new()
    }
}
