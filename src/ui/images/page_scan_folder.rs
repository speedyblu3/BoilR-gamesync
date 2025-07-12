use egui::{Ui, ScrollArea};
use std::path::PathBuf;
use crate::game_scan::{scan_folder, ScannedGame};
use crate::steam_shortcuts_util::shortcut::ShortcutOwned;
use rfd::FileDialog;
use crate::settings::Settings;
use crate::sync;
use std::collections::HashMap;
use futures::executor::block_on;
use std::path::Path;

pub struct ScanFolderState {
    pub folder: Option<PathBuf>,
    pub found_games: Vec<ScannedGame>,
    pub added_shortcuts: usize,
    pub error: Option<String>,
    pub import_status: Option<String>, // New: status/progress for import
}

impl Default for ScanFolderState {
    fn default() -> Self {
        Self {
            folder: None,
            found_games: vec![],
            added_shortcuts: 0,
            error: None,
            import_status: None,
        }
    }
}

pub fn render_scan_games_folder(
    ui: &mut Ui,
    state: &mut ScanFolderState,
    _add_shortcut: &mut dyn FnMut(ShortcutOwned),
    _trigger_image_download: &mut dyn FnMut(&ShortcutOwned),
) {
    ui.heading("Scan Games Folder");
    if ui.button("Choose Games Folder").clicked() {
        if let Some(folder) = FileDialog::new().pick_folder() {
            state.folder = Some(folder);
            state.found_games.clear();
            state.added_shortcuts = 0;
            state.error = None;
            state.import_status = None;
        }
    }

    if let Some(folder) = &state.folder {
        if ui.button("Scan for Games").clicked() {
            let games = scan_folder(folder);
            if games.is_empty() {
                state.error = Some("No games found in this folder.".to_string());
            } else {
                state.found_games = games;
                state.error = None;
            }
            state.import_status = None;
        }
        ui.label(format!("Selected folder: {}", folder.display()));
    }

    if let Some(error) = &state.error {
        ui.colored_label(egui::Color32::RED, error);
    }

    if !state.found_games.is_empty() {
        ui.separator();
        ui.label("Games found:");
        ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            for game in &state.found_games {
                ui.horizontal(|ui| {
                    ui.label(&game.name);
                    ui.label(game.path.display().to_string());
                });
            }
        });
        ui.label(format!("Total games found: {}", state.found_games.len()));
        ui.add_space(10.0);
        if ui.button("Import All to Steam").clicked() {
            // Import all found games using the sync/artwork pipeline
            let shortcuts: Vec<ShortcutOwned> = state.found_games.iter().map(|g| {
                ShortcutOwned {
                    app_name: g.name.clone(),
                    exe: format!("\"{}\"", g.path.display()),
                    start_dir: format!("\"{}\"", g.path.parent().unwrap_or(Path::new(".")).display()),
                    icon: String::new(),
                    shortcut_path: String::new(),
                    launch_options: String::new(),
                    is_hidden: false,
                    allow_desktop_config: true,
                    allow_overlay: true,
                    open_vr: false,
                    dev_kit: false,
                    dev_kit_game_id: String::new(),
                    dev_kit_override_app_id: 0,
                    last_play_time: 0,
                    flatpak_app_id: String::new(),
                    tags: vec![],
                    app_id: 0,
                }
            }).collect();
            let platform_shortcuts = vec![("ScannedFolder".to_string(), shortcuts)];
            let settings = match Settings::new() {
                Ok(s) => s,
                Err(e) => {
                    state.import_status = Some(format!("Failed to load settings: {e}"));
                    return;
                }
            };
            let renames: HashMap<u32, String> = HashMap::new();
            state.import_status = Some("Importing shortcuts into Steam...".to_string());
            let mut sender = None;
            match sync::sync_shortcuts(&settings, &platform_shortcuts, &mut sender, &renames) {
                Ok(usersinfo) => {
                    state.import_status = Some("Downloading artwork from SteamGridDB...".to_string());
                    block_on(sync::download_images(&settings, &usersinfo, &mut sender));
                    if let Err(e) = sync::fix_all_shortcut_icons(&settings) {
                        state.import_status = Some(format!("Could not fix shortcuts: {e}"));
                    } else {
                        state.import_status = Some("Scan/import/artwork complete.".to_string());
                    }
                }
                Err(e) => {
                    state.import_status = Some(format!("Failed to import shortcuts: {e}"));
                }
            }
        }
        if let Some(status) = &state.import_status {
            ui.label(status);
        }
    }
}