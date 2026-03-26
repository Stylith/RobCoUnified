use super::retro_ui::{current_palette, RetroScreen};
use crate::config::HEADER_LINES;
use eframe::egui::{self, Context, Rect};
use robcos_native_nuke_codes_app::{
    fetch_nuke_codes as fetch_builtin_nuke_codes, fetch_nuke_codes_with_providers,
    resolve_nuke_codes_event, NukeCodesProvider,
};
pub use robcos_native_nuke_codes_app::{NukeCodesEvent, NukeCodesView};
use serde::Deserialize;
use std::path::Path;

const NUKE_CODES_ADDON_ID: &str = "tools.nuke-codes";
const NUKE_CODES_PROVIDER_FILE: &str = "providers.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct NukeCodesProviderConfig {
    source: String,
    url: String,
}

pub fn fetch_nuke_codes() -> NukeCodesView {
    match load_nuke_code_providers_from_bundle() {
        Ok(Some(providers)) if !providers.is_empty() => fetch_nuke_codes_with_providers(&providers),
        Ok(_) => fetch_builtin_nuke_codes(),
        Err(err) => NukeCodesView::Error(format!("Addon bundle config error: {err}")),
    }
}

fn load_nuke_code_providers_from_bundle() -> Result<Option<Vec<NukeCodesProvider>>, String> {
    let Some(provider_file) = super::addons::installed_addon_bundle_path(
        &crate::platform::AddonId::from(NUKE_CODES_ADDON_ID),
        NUKE_CODES_PROVIDER_FILE,
    )
    else {
        return Ok(None);
    };
    load_nuke_code_providers_from_file(&provider_file)
}

#[cfg(test)]
fn load_nuke_code_providers_from_dir(dir: &Path) -> Result<Option<Vec<NukeCodesProvider>>, String> {
    load_nuke_code_providers_from_file(&dir.join(NUKE_CODES_PROVIDER_FILE))
}

fn load_nuke_code_providers_from_file(path: &Path) -> Result<Option<Vec<NukeCodesProvider>>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let parsed: Vec<NukeCodesProviderConfig> = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse '{}': {error}", path.display()))?;
    let providers = parsed
        .into_iter()
        .filter(|provider| {
            let source = provider.source.trim();
            let url = provider.url.trim();
            !source.is_empty() && !url.is_empty()
        })
        .map(|provider| NukeCodesProvider {
            source: provider.source,
            url: provider.url,
        })
        .collect::<Vec<_>>();
    if providers.is_empty() {
        return Err(format!(
            "'{}' does not define any usable providers.",
            path.display()
        ));
    }
    Ok(Some(providers))
}

#[allow(clippy::too_many_arguments)]
pub fn draw_nuke_codes_screen(
    ctx: &Context,
    state: &NukeCodesView,
    cols: usize,
    rows: usize,
    header_start_row: usize,
    separator_top_row: usize,
    title_row: usize,
    separator_bottom_row: usize,
    menu_start_row: usize,
    status_row: usize,
    content_col: usize,
) -> NukeCodesEvent {
    let refresh = ctx.input(|i| i.key_pressed(egui::Key::R));
    let back = ctx.input(|i| {
        i.key_pressed(egui::Key::Q)
            || i.key_pressed(egui::Key::Escape)
            || i.key_pressed(egui::Key::Tab)
    });

    let event = resolve_nuke_codes_event(refresh, back);

    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(current_palette().bg)
                .inner_margin(0.0),
        )
        .show(ctx, |ui| {
            let palette = current_palette();
            let (screen, _) = RetroScreen::new(ui, cols, rows);
            let painter = ui.painter_at(screen.rect);
            screen.paint_bg(&painter, palette.bg);
            for (idx, line) in HEADER_LINES.iter().enumerate() {
                screen.centered_text(&painter, header_start_row + idx, line, palette.fg, true);
            }
            screen.separator(&painter, separator_top_row, &palette);
            screen.centered_text(
                &painter,
                title_row,
                "NUCLEAR LAUNCH CODES",
                palette.fg,
                true,
            );
            screen.separator(&painter, separator_bottom_row, &palette);

            match state {
                NukeCodesView::Data(codes) => {
                    let block_w = 21usize;
                    let top = screen.row_rect(content_col, menu_start_row, block_w);
                    let bottom = screen.row_rect(content_col, menu_start_row + 2, block_w);
                    let block = Rect::from_min_max(top.min, egui::pos2(bottom.max.x, bottom.max.y));
                    painter.rect_filled(block, 0.0, palette.panel);

                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        &format!("ALPHA   : {}", codes.alpha),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 1,
                        &format!("BRAVO   : {}", codes.bravo),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 2,
                        &format!("CHARLIE : {}", codes.charlie),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 4,
                        &format!("SOURCE      : {}", codes.source),
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 5,
                        &format!("FETCHED AT  : {}", codes.fetched_at),
                        palette.fg,
                    );
                }
                NukeCodesView::Error(err) => {
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        "UNABLE TO FETCH LIVE CODES",
                        palette.fg,
                    );
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row + 2,
                        &format!("ERROR: {err}"),
                        palette.dim,
                    );
                }
                NukeCodesView::Unloaded => {
                    screen.text(
                        &painter,
                        content_col + 1,
                        menu_start_row,
                        "Loading launch codes...",
                        palette.fg,
                    );
                }
            }

            screen.text(
                &painter,
                content_col,
                status_row.saturating_sub(3),
                "Press R to refresh. Q / Esc / Tab = Back",
                palette.dim,
            );
        });

    event
}

#[cfg(test)]
mod tests {
    use super::{load_nuke_code_providers_from_dir, NUKE_CODES_PROVIDER_FILE};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn loads_nuke_code_providers_from_bundle_file() {
        let dir = temp_dir("loads_nuke_code_providers_from_bundle_file");
        fs::write(
            dir.join(NUKE_CODES_PROVIDER_FILE),
            r#"[{"source":"Mirror A","url":"https://example.invalid/a"}]"#,
        )
        .unwrap();

        let providers = load_nuke_code_providers_from_dir(&dir).unwrap().unwrap();

        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].source, "Mirror A");
        assert_eq!(providers[0].url, "https://example.invalid/a");
    }

    #[test]
    fn missing_provider_file_falls_back_cleanly() {
        let dir = temp_dir("missing_provider_file_falls_back_cleanly");
        assert!(load_nuke_code_providers_from_dir(&dir).unwrap().is_none());
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("robcos-nuke-codes-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
