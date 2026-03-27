use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::RobcoNativeApp;

pub struct ClassicPanelRenderer;

impl SlotRenderer for ClassicPanelRenderer {
    fn slot(&self) -> ShellSlot {
        ShellSlot::Panel
    }

    fn render(&self, app: &mut RobcoNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
        app.render_classic_panel_slot(slot_ctx.ctx, slot_ctx.layout);
        vec![]
    }
}
