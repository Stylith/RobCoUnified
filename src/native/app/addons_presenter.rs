use super::super::addons::{
    install_repository_theme, load_theme_repository_index_from_path,
    remove_installed_repository_item, ThemeRepositoryIndex,
};
use super::super::background::BackgroundResult;
use super::super::desktop_app::{DesktopWindow, WindowInstanceId};
use super::super::retro_ui::{current_palette, RetroPalette};
use super::super::{
    install_repository_addon, installed_addon_inventory, installed_color_themes,
    installed_cursor_packs, installed_desktop_styles, installed_font_packs,
    installed_icon_packs, installed_sound_packs, installed_terminal_themes,
    installed_theme_packs,
};
use super::desktop_window_mgmt::{DesktopHeaderAction, DesktopWindowRectTracking};
use super::{
    AddonsAddonSubcategory, AddonsRepoCache, AddonsSidebarCategory, AddonsThemeSubcategory,
    NucleonNativeApp, RepoAddonEntry, RepoThemeEntry,
};
use crate::config;
use crate::platform::{AddonId, AddonKind, AddonRepositoryIndex};
use eframe::egui::{self, Context, RichText};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;

struct AddonsListEntry {
    id: String,
    name: String,
    description: String,
    version: String,
    kind: AddonKind,
    installed: bool,
    update_available: bool,
}

impl AddonsListEntry {
    fn from_addon(entry: &RepoAddonEntry) -> Self {
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            version: entry.version.clone(),
            kind: entry.kind,
            installed: entry.installed,
            update_available: entry.update_available,
        }
    }

    fn from_theme(entry: &RepoThemeEntry) -> Self {
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            version: entry.version.clone(),
            kind: entry.kind,
            installed: entry.installed,
            update_available: entry.update_available,
        }
    }
}

fn addons_kind_label(kind: AddonKind) -> &'static str {
    match kind {
        AddonKind::App => "App",
        AddonKind::Theme => "Theme",
        AddonKind::DesktopTheme => "Desktop Theme",
        AddonKind::TerminalTheme => "Terminal Theme",
        AddonKind::ColorTheme => "Color Theme",
        AddonKind::IconPack => "Icon Pack",
        AddonKind::SoundPack => "Sound Pack",
        AddonKind::CursorPack => "Cursor Pack",
        AddonKind::FontPack => "Font Pack",
        AddonKind::ContentPack => "Content Pack",
        AddonKind::Game => "Game",
        AddonKind::Service => "Service",
    }
}

impl NucleonNativeApp {
    pub(super) fn open_addons_from_settings(&mut self) {
        self.addons_open = true;
        self.prime_desktop_window_defaults(DesktopWindow::Addons);
        self.desktop_active_window = Some(WindowInstanceId::primary(DesktopWindow::Addons));
    }

    pub(super) fn draw_addons(&mut self, ctx: &Context) {
        if !self.addons_open || self.desktop_window_is_minimized(DesktopWindow::Addons) {
            return;
        }
        if self.addons_repo_cache.is_none() && !self.addons_repo_fetch_in_progress {
            self.fetch_addons_repo_indexes();
        }
        let wid = self.current_window_id(DesktopWindow::Addons);
        let mut open = self.addons_open;
        let maximized = self.desktop_window_is_maximized(DesktopWindow::Addons);
        let restore = self.take_desktop_window_restore_dims(DesktopWindow::Addons);
        let mut header_action = DesktopHeaderAction::None;
        let egui_id = self.desktop_window_egui_id(wid);
        let default_size = Self::desktop_default_window_size(DesktopWindow::Addons);
        let default_pos = self.active_desktop_default_window_pos(ctx, default_size);
        let mut window = egui::Window::new("Addons")
            .id(egui_id)
            .open(&mut open)
            .title_bar(false)
            .frame(self.desktop_window_frame())
            .resizable(false)
            .default_pos(default_pos)
            .fixed_size(default_size);
        if maximized {
            let rect = self.active_desktop_workspace_rect(ctx);
            window = window
                .movable(false)
                .fixed_pos(rect.min)
                .fixed_size(rect.size());
        } else if let Some((pos, _size)) = restore {
            let pos = self.active_desktop_clamp_window_pos(ctx, pos, default_size);
            window = window.current_pos(pos);
        }

        let shown = window.show(ctx, |ui| {
            Self::apply_settings_control_style(ui);
            header_action = Self::draw_desktop_window_header(
                ui,
                "Addons",
                maximized,
                self.desktop_active_window == Some(wid),
                &self.desktop_active_desktop_style,
            );
            let palette = current_palette();

            ui.add_space(4.0);
            ui.label(RichText::new("Addons").strong().size(28.0));
            ui.add_space(10.0);
            Self::retro_separator_with_thickness(
                ui,
                self.desktop_active_desktop_style.separator_thickness,
            );
            ui.add_space(10.0);

            let body_height = ui.available_height().max(120.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), body_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.push_id("addons_layout", |ui| {
                        egui::SidePanel::left("addons_sidebar")
                            .exact_width(200.0)
                            .frame(egui::Frame::none())
                            .show_inside(ui, |ui| {
                                self.draw_addons_sidebar(ui, &palette);
                            });
                        egui::CentralPanel::default()
                            .frame(egui::Frame::none())
                            .show_inside(ui, |ui| {
                                self.draw_addons_content(ui, &palette);
                            });
                    });
                },
            );
        });

        let shown_rect = shown.as_ref().map(|inner| inner.response.rect);
        let shown_contains_pointer = shown
            .as_ref()
            .is_some_and(|inner| inner.response.contains_pointer());
        self.finish_desktop_window_host(
            ctx,
            DesktopWindow::Addons,
            &mut open,
            maximized,
            shown_rect,
            shown_contains_pointer,
            DesktopWindowRectTracking::PositionOnly,
            header_action,
        );
    }

    fn draw_addons_sidebar(&mut self, ui: &mut egui::Ui, palette: &RetroPalette) {
        let categories = [
            ("Installed", AddonsSidebarCategory::Installed),
            ("Addons", AddonsSidebarCategory::Addons),
            ("Themes", AddonsSidebarCategory::Themes),
            ("Tools", AddonsSidebarCategory::Tools),
            ("Extras", AddonsSidebarCategory::Extras),
        ];

        ui.label(RichText::new("Library").color(palette.dim).strong());
        ui.add_space(8.0);

        for (label, category) in categories {
            let selected = self.addons_sidebar_category == category;
            if draw_addons_sidebar_button(ui, label, selected, palette).clicked() {
                self.addons_sidebar_category = category;
            }
            if category == AddonsSidebarCategory::Addons && selected {
                ui.indent("addon_sub", |ui| {
                    let subcategories = [
                        ("Apps", AddonsAddonSubcategory::Apps),
                        ("Games", AddonsAddonSubcategory::Games),
                    ];
                    for (label, subcategory) in subcategories {
                        let selected = self.addons_addon_subcategory == subcategory;
                        if draw_addons_sidebar_button(ui, label, selected, palette).clicked() {
                            self.addons_addon_subcategory = subcategory;
                        }
                    }
                });
            }
            if category == AddonsSidebarCategory::Themes && selected {
                ui.indent("theme_sub", |ui| {
                    let subcategories = [
                        ("Packs", AddonsThemeSubcategory::Packs),
                        ("Desktop", AddonsThemeSubcategory::Desktop),
                        ("Terminal", AddonsThemeSubcategory::Terminal),
                        ("Colors", AddonsThemeSubcategory::Colors),
                        ("Icons", AddonsThemeSubcategory::Icons),
                        ("Sounds", AddonsThemeSubcategory::Sounds),
                        ("Cursors", AddonsThemeSubcategory::Cursors),
                        ("Fonts", AddonsThemeSubcategory::Fonts),
                    ];
                    for (label, subcategory) in subcategories {
                        let selected = self.addons_theme_subcategory == subcategory;
                        if draw_addons_sidebar_button(ui, label, selected, palette).clicked() {
                            self.addons_theme_subcategory = subcategory;
                        }
                    }
                });
            }
        }
    }

    fn draw_addons_content(&mut self, ui: &mut egui::Ui, palette: &RetroPalette) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Repository")
                    .strong()
                    .color(palette.fg)
                    .heading(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.fetch_addons_repo_indexes();
                }
            });
        });
        ui.add_space(8.0);
        Self::retro_separator_with_thickness(
            ui,
            self.desktop_active_desktop_style.separator_thickness,
        );
        ui.add_space(8.0);

        if self.addons_repo_fetch_in_progress {
            ui.label(RichText::new("Fetching repository indexes...").color(palette.dim));
            return;
        }

        let Some(cache) = self.addons_repo_cache.as_ref() else {
            ui.label(RichText::new("No repository data loaded.").color(palette.dim));
            return;
        };

        let entries = self.filtered_addons_entries(cache);
        if entries.is_empty() {
            ui.label(RichText::new("No items in this category.").color(palette.dim));
            return;
        }

        let body_max_height = ui.available_height().max(120.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(body_max_height)
            .show(ui, |ui| {
                for entry in entries {
                    egui::Frame::none()
                        .stroke(egui::Stroke::new(2.0, palette.fg))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let action_label = self.addons_entry_action_label(&entry);
                                let action_width = 120.0;
                                let text_width =
                                    (ui.available_width() - action_width - 20.0).max(160.0);

                                ui.add_sized(
                                    [text_width, 0.0],
                                    egui::Label::new(
                                        RichText::new(&entry.name).strong().color(palette.fg),
                                    )
                                    .truncate(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add_sized(
                                                [action_width, 0.0],
                                                egui::Button::new(action_label),
                                            )
                                            .clicked()
                                        {
                                            self.handle_addons_entry_action(
                                                &entry.id,
                                                entry.kind,
                                                entry.installed,
                                                entry.update_available,
                                            );
                                        }
                                    },
                                );
                            });
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(format!(
                                    "{}  •  {}",
                                    addons_kind_label(entry.kind),
                                    entry.version
                                ))
                                .small()
                                .color(palette.dim),
                            );
                            if !entry.description.is_empty() {
                                ui.label(
                                    RichText::new(&entry.description).small().color(palette.dim),
                                );
                            }
                        });
                    ui.add_space(8.0);
                }
            });
    }

    fn filtered_addons_entries(&self, cache: &AddonsRepoCache) -> Vec<AddonsListEntry> {
        let mut entries = Vec::new();
        match self.addons_sidebar_category {
            AddonsSidebarCategory::Installed => {
                entries.extend(
                    cache
                        .addons_index
                        .iter()
                        .filter(|entry| entry.installed)
                        .map(AddonsListEntry::from_addon),
                );
                entries.extend(
                    cache
                        .themes_index
                        .iter()
                        .filter(|entry| entry.installed)
                        .map(AddonsListEntry::from_theme),
                );
            }
            AddonsSidebarCategory::Addons => {
                entries.extend(
                    cache
                        .addons_index
                        .iter()
                        .filter(|entry| match self.addons_addon_subcategory {
                            AddonsAddonSubcategory::Apps => {
                                matches!(
                                    entry.kind,
                                    AddonKind::App | AddonKind::ContentPack | AddonKind::Service
                                )
                            }
                            AddonsAddonSubcategory::Games => entry.kind == AddonKind::Game,
                        })
                        .map(AddonsListEntry::from_addon),
                );
            }
            AddonsSidebarCategory::Themes => {
                entries.extend(
                    cache
                        .themes_index
                        .iter()
                        .filter(|entry| match self.addons_theme_subcategory {
                            AddonsThemeSubcategory::Packs => entry.kind == AddonKind::Theme,
                            AddonsThemeSubcategory::Desktop => {
                                entry.kind == AddonKind::DesktopTheme
                            }
                            AddonsThemeSubcategory::Terminal => {
                                entry.kind == AddonKind::TerminalTheme
                            }
                            AddonsThemeSubcategory::Colors => entry.kind == AddonKind::ColorTheme,
                            AddonsThemeSubcategory::Icons => entry.kind == AddonKind::IconPack,
                            AddonsThemeSubcategory::Sounds => entry.kind == AddonKind::SoundPack,
                            AddonsThemeSubcategory::Cursors => entry.kind == AddonKind::CursorPack,
                            AddonsThemeSubcategory::Fonts => entry.kind == AddonKind::FontPack,
                        })
                        .map(AddonsListEntry::from_theme),
                );
            }
            AddonsSidebarCategory::Tools | AddonsSidebarCategory::Extras => {}
        }
        entries
    }

    fn addons_entry_action_label(&self, entry: &AddonsListEntry) -> &'static str {
        if !entry.installed {
            "Install"
        } else if entry.update_available {
            "Update"
        } else {
            "Uninstall"
        }
    }

    pub(super) fn fetch_addons_repo_indexes(&mut self) {
        if self.addons_repo_fetch_in_progress {
            return;
        }
        self.addons_repo_fetch_in_progress = true;
        let tx = self.background.sender();
        thread::spawn(move || {
            let addons = fetch_addon_repo_entries();
            let themes = fetch_theme_repo_entries();
            let _ = tx.send(BackgroundResult::AddonsRepoIndexesFetched { addons, themes });
        });
    }

    fn handle_addons_entry_action(
        &mut self,
        item_id: &str,
        kind: AddonKind,
        installed: bool,
        update_available: bool,
    ) {
        let tx = self.background.sender();
        let item_id = item_id.to_string();
        thread::spawn(move || {
            let uninstall = installed && !update_available;
            let result = if uninstall {
                remove_installed_repository_item(kind, &item_id)
            } else if matches!(
                kind,
                AddonKind::Theme
                    | AddonKind::DesktopTheme
                    | AddonKind::TerminalTheme
                    | AddonKind::ColorTheme
                    | AddonKind::IconPack
                    | AddonKind::SoundPack
                    | AddonKind::CursorPack
                    | AddonKind::FontPack
            ) {
                install_repository_theme(&item_id)
            } else {
                install_repository_addon(AddonId::from(item_id.clone()))
            };
            let (status, success) = match result {
                Ok(status) => (status, true),
                Err(status) => (status, false),
            };
            let _ = tx.send(BackgroundResult::AddonsRepoActionFinished {
                item_id,
                kind,
                status,
                success,
                installed: if uninstall { false } else { success },
            });
        });
    }
}

fn draw_addons_sidebar_button(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    palette: &RetroPalette,
) -> egui::Response {
    let text_color = if selected {
        palette.selected_fg
    } else {
        palette.fg
    };
    ui.add_sized(
        [ui.available_width(), 28.0],
        egui::Button::new(RichText::new(label).color(text_color))
            .fill(if selected {
                palette.selected_bg
            } else {
                palette.bg
            })
            .stroke(egui::Stroke::new(2.0, palette.fg)),
    )
}

fn download_repository_index(url: &str, cached_path: &Path) -> Result<(), String> {
    if let Some(parent) = cached_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create repository cache directory: {error}"))?;
    }
    let status = Command::new("curl")
        .arg("-L")
        .arg("--fail")
        .arg("--silent")
        .arg("--max-time")
        .arg("15")
        .arg("-o")
        .arg(cached_path)
        .arg(url)
        .status()
        .map_err(|error| format!("Failed to launch curl: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "curl failed while downloading repository index (exit {}).",
            status
        ))
    }
}

fn fetch_addon_repo_entries() -> Result<Vec<RepoAddonEntry>, String> {
    let cached_path = config::cached_addon_repository_index_file();
    let download_result =
        download_repository_index(&config::addon_repository_index_url(), &cached_path);
    if download_result.is_err() && !cached_path.exists() {
        return Err(download_result.unwrap_err());
    }
    let raw = fs::read_to_string(&cached_path)
        .map_err(|error| format!("Failed to read addon repository index: {error}"))?;
    let index = serde_json::from_str::<AddonRepositoryIndex>(&raw)
        .map_err(|error| format!("Failed to parse addon repository index: {error}"))?;
    Ok(repo_addon_entries_from_index(&index))
}

fn fetch_theme_repo_entries() -> Result<Vec<RepoThemeEntry>, String> {
    let cached_path = config::cached_theme_repository_index_file();
    let download_result =
        download_repository_index(&config::theme_repository_index_url(), &cached_path);
    if download_result.is_err() && !cached_path.exists() {
        return Err(download_result.unwrap_err());
    }
    let index = load_theme_repository_index_from_path(&cached_path)?;
    Ok(repo_theme_entries_from_index(&index))
}

fn repo_addon_entries_from_index(index: &AddonRepositoryIndex) -> Vec<RepoAddonEntry> {
    let installed_versions = installed_addon_inventory()
        .into_iter()
        .map(|record| (record.manifest.id.to_string(), record.manifest.version))
        .collect::<HashMap<_, _>>();
    let mut entries = index
        .addons
        .iter()
        .filter(|package| !package.manifest.essential)
        .map(|package| {
            let installed_version = installed_versions.get(package.manifest.id.as_str());
            RepoAddonEntry {
                id: package.manifest.id.to_string(),
                name: package.manifest.display_name.clone(),
                description: String::new(),
                version: package.manifest.version.clone(),
                kind: package.manifest.kind,
                installed: installed_version.is_some(),
                update_available: installed_version
                    .is_some_and(|version| version != &package.manifest.version),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn repo_theme_entries_from_index(index: &ThemeRepositoryIndex) -> Vec<RepoThemeEntry> {
    let mut installed_versions = HashMap::new();
    for theme in installed_theme_packs() {
        installed_versions.insert((AddonKind::Theme, theme.id), theme.version);
    }
    for manifest in installed_desktop_styles() {
        installed_versions.insert((AddonKind::DesktopTheme, manifest.id), manifest.version);
    }
    for manifest in installed_terminal_themes() {
        installed_versions.insert((AddonKind::TerminalTheme, manifest.id), manifest.version);
    }
    for manifest in installed_color_themes() {
        installed_versions.insert((AddonKind::ColorTheme, manifest.id), manifest.version);
    }
    for manifest in installed_icon_packs() {
        installed_versions.insert((AddonKind::IconPack, manifest.id), manifest.version);
    }
    for manifest in installed_sound_packs() {
        installed_versions.insert((AddonKind::SoundPack, manifest.id), manifest.version);
    }
    for manifest in installed_cursor_packs() {
        installed_versions.insert((AddonKind::CursorPack, manifest.id), manifest.version);
    }
    for manifest in installed_font_packs() {
        installed_versions.insert((AddonKind::FontPack, manifest.id), manifest.version);
    }

    let mut entries = index
        .entries
        .iter()
        .map(|entry| {
            let installed_version = installed_versions.get(&(entry.kind, entry.id.clone()));
            RepoThemeEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                version: entry.version.clone(),
                kind: entry.kind,
                installed: installed_version.is_some(),
                update_available: installed_version
                    .is_some_and(|version| version != &entry.version),
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}
