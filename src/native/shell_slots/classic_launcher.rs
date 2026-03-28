use super::{ShellSlot, SlotAction, SlotContext, SlotRenderer};
use crate::native::app::NucleonNativeApp;
use crate::theme::LauncherStyle;

pub struct ClassicLauncherRenderer;

impl SlotRenderer for ClassicLauncherRenderer {
    fn slot(&self) -> ShellSlot {
        ShellSlot::Launcher
    }

    fn render(&self, app: &mut NucleonNativeApp, slot_ctx: &SlotContext) -> Vec<SlotAction> {
        if slot_ctx.layout.launcher_style == LauncherStyle::Hidden {
            return vec![];
        }
        app.render_classic_launcher_slot(slot_ctx.ctx);
        vec![]
    }
}
