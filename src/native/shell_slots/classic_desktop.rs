use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::NucleonNativeApp;

pub struct ClassicDesktopRenderer;

impl SlotRenderer for ClassicDesktopRenderer {
    fn slot(&self) -> ShellSlot {
        ShellSlot::Desktop
    }

    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
        app.render_classic_desktop_slot(slot_ctx.ctx);
        vec![]
    }
}
