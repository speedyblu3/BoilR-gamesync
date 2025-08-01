use std::{collections::HashMap, error::Error, time::Duration};

use eframe::{egui, App, Frame};
use egui::{
   ImageButton, Rounding, Stroke, Vec2
};
use tokio::{
    runtime::Runtime,
    sync::watch::{self, Receiver},
};
use rfd::FileDialog;

// === ADDED: Scan Games Folder UI imports ===
// use crate::ui::images::page_scan_folder::{ScanFolderState, render_scan_games_folder};

use crate::{
    config::get_renames_file,
    platforms::{get_platforms, GamesPlatform, Platforms, ShortcutToImport},
    settings::{save_settings, Settings},
    sync::{self, SyncProgress},
};

use super::{
    images::ImageSelectState,
    ui_colors::{
        BACKGROUND_COLOR, BG_STROKE_COLOR, EXTRA_BACKGROUND_COLOR, LIGHT_ORANGE, ORANGE, PURLPLE,
        TEXT_COLOR,
    },
    ui_images::get_logo_icon,
    ui_import_games::FetchStatus,
    BackupState, DisconnectState,
};

const SECTION_SPACING: f32 = 25.0;

type GamesToSync = Vec<(
    String,
    Receiver<FetchStatus<eyre::Result<Vec<ShortcutToImport>>>>,
)>;

pub(crate) fn all_ready(games: &GamesToSync) -> bool {
    games.iter().all(|(_name, rx)| rx.borrow().is_some())
}

pub(crate) fn get_all_games(games: &GamesToSync) -> Vec<(String, Vec<ShortcutToImport>)> {
    games
        .iter()
        .filter_map(|(name, rx)| {
            if let FetchStatus::Fetched(Ok(data)) = &*rx.borrow() {
                Some((name.to_owned(), data.to_owned()))
            } else {
                None
            }
        })
        .collect()
}

pub struct MyEguiApp {
    selected_menu: Menues,
    pub(crate) settings: Settings,
    pub(crate) rt: Runtime,
    pub(crate) games_to_sync: GamesToSync,
    pub(crate) status_reciever: Receiver<SyncProgress>,
    pub(crate) image_selected_state: ImageSelectState,
    pub(crate) backup_state: BackupState,
    pub(crate) disconnect_state: DisconnectState,
    pub(crate) rename_map: HashMap<u32, String>,
    pub(crate) current_edit: Option<u32>,
    pub(crate) platforms: Platforms,
}

impl MyEguiApp {
    pub fn new() -> eyre::Result<Self> {
        let mut runtime = Runtime::new()?;
        let settings = Settings::new()?;
        let platforms = get_platforms();
        let games_to_sync = create_games_to_sync(&mut runtime, &platforms);
        Ok(Self {
            selected_menu: Menues::Import,
            settings,
            rt: runtime,
            games_to_sync,
            status_reciever: watch::channel(SyncProgress::NotStarted).1,
            image_selected_state: ImageSelectState::default(),
            backup_state: BackupState::default(),
            disconnect_state: DisconnectState::default(),
            rename_map: get_rename_map(),
            current_edit: Option::None,
            platforms,
        })
    }

    fn render_import_button(&mut self, ui: &mut egui::Ui) {
        let (status_string, syncing) = match &*self.status_reciever.borrow() {
            SyncProgress::NotStarted => ("".to_string(), false),
            SyncProgress::Starting => ("Starting Import".to_string(), true),
            SyncProgress::FoundGames { games_found } => {
                (format!("Found {games_found} games to  import"), true)
            }
            SyncProgress::FindingImages => ("Searching for images".to_string(), true),
            SyncProgress::DownloadingImages { to_download } => {
                (format!("Downloading {to_download} images "), true)
            }
            SyncProgress::Done => ("Done importing games".to_string(), false),
        };
        if syncing {
            ui.ctx().request_repaint();
        }
        if !status_string.is_empty() {
            if syncing {
                ui.horizontal(|c| {
                    c.spinner();
                    c.label(&status_string);
                });
            } else {
                ui.label(&status_string);
            }
        }
        let all_ready = all_ready(&self.games_to_sync);
        let import_image = egui::include_image!("../../resources/import_games_button.png");
        let size = Vec2::new(200.,100.);
        let image_button = ImageButton::new(import_image);
        if all_ready && !syncing {
            if ui
                .add_sized(size,image_button)
                .on_hover_text("Import your games into steam")
                .clicked()
            {
                if let Err(err) = save_settings(&self.settings, &self.platforms) {
                    eprintln!("Failed to save settings {err:?}");
                }
                self.run_sync_async();
            }
        } else {
            ui.add_sized(size,image_button)
                .on_hover_text("Waiting for sync to finish");
        }
    }

    fn render_scan_folder(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scan Games Folder");
        
        if ui.button("Choose Games Folder").clicked() {
            if let Some(folder) = FileDialog::new().pick_folder() {
                println!("Selected folder: {}", folder.display());
                // self.scan_folder_state.folder = Some(folder); // Removed
                // self.scan_folder_state.found_games.clear(); // Removed
                // self.scan_folder_state.error = None; // Removed
                // self.scan_folder_state.import_status = None; // Removed
            }
        }

        // if let Some(folder) = &self.scan_folder_state.folder { // Removed
        //     ui.label(format!("Selected folder: {}", folder.display())); // Removed
        //     
        //     if ui.button("Scan for Games").clicked() { // Removed
        //         println!("Scanning folder: {}", folder.display()); // Removed
        //         let games = scan_folder(folder); // Removed
        //         println!("Found {} games", games.len()); // Removed
        //         if games.is_empty() { // Removed
        //             self.scan_folder_state.error = Some("No games found in this folder.".to_string()); // Removed
        //         } else { // Removed
        //             self.scan_folder_state.found_games = games; // Removed
        //             self.scan_folder_state.error = None; // Removed
        //         } // Removed
        //         self.scan_folder_state.import_status = None; // Removed
        //     } // Removed
        // } // Removed

        // if let Some(error) = &self.scan_folder_state.error { // Removed
        //     ui.colored_label(egui::Color32::RED, error); // Removed
        // } // Removed

        // if !self.scan_folder_state.found_games.is_empty() { // Removed
        //     ui.separator(); // Removed
        //     ui.label("Games found:"); // Removed
        //     egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| { // Removed
        //         for game in &self.scan_folder_state.found_games { // Removed
        //             ui.horizontal(|ui| { // Removed
        //                 ui.vertical(|ui| { // Removed
        //                     ui.label(format!("Name: {}", game.name)); // Removed
        //                     ui.label(format!("App ID: {}", game.app_id)); // Removed
        //                     ui.label(format!("Size: {:.1} MB", game.size_bytes as f64 / (1024.0 * 1024.0))); // Removed
        //                 }); // Removed
        //                 ui.vertical(|ui| { // Removed
        //                     ui.label(format!("Path: {}", game.path.display())); // Removed
        //                 }); // Removed
        //             }); // Removed
        //             ui.separator(); // Removed
        //         } // Removed
        //     }); // Removed
        //     ui.label(format!("Total games found: {}", self.scan_folder_state.found_games.len())); // Removed
        //     ui.add_space(10.0); // Removed
        //     
        //     if ui.button("Import All to Steam").clicked() { // Removed
        //         self.import_scanned_games(); // Removed
        //     } // Removed
        // } // Removed
        
        // if let Some(status) = &self.scan_folder_state.import_status { // Removed
        //     ui.label(status); // Removed
        // } // Removed
    }
}

fn get_rename_map() -> HashMap<u32, String> {
    try_get_rename_map().unwrap_or_default()
}

fn try_get_rename_map() -> Result<HashMap<u32, String>, Box<dyn Error>> {
    let rename_map = get_renames_file();
    let file_content = std::fs::read_to_string(rename_map)?;
    let deserialized = serde_json::from_str(&file_content)?;
    Ok(deserialized)
}

// === CHANGED: Menues enum, remove ScanFolder ===
#[derive(PartialEq, Clone, Default)]
enum Menues {
    #[default]
    Import,
    Settings,
    Images,
    Backup,
    Disconnect,
}

fn create_games_to_sync(rt: &mut Runtime, platforms: &[Box<dyn GamesPlatform>]) -> GamesToSync {
    let mut to_sync = vec![];
    for platform in platforms {
        if platform.enabled() {
            let (tx, rx) = watch::channel(FetchStatus::NeedsFetched);
            to_sync.push((platform.name().to_string(), rx));
            let platform = platform.clone();
            rt.spawn_blocking(move || {
                let _ = tx.send(FetchStatus::Fetching);
                let games_to_sync = sync::get_platform_shortcuts(platform);
                let _ = tx.send(FetchStatus::Fetched(games_to_sync));
            });
        }
    }
    to_sync
}

impl App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        ctx.set_pixels_per_point(1.0);
        let frame = egui::Frame::default()
            .stroke(Stroke::new(0., BACKGROUND_COLOR))
            .fill(BACKGROUND_COLOR);
        egui::SidePanel::new(egui::panel::Side::Left, "Side Panel")
            .default_width(40.0)
            .frame(frame)
            .show(ctx, |ui| {
                let image = egui::include_image!("../../resources/logo32.png");
                ui.image(image);
                ui.add_space(SECTION_SPACING);

                let menu_before = self.selected_menu.clone();

                let mut changed = ui
                    .selectable_value(&mut self.selected_menu, Menues::Import, "Import Games")
                    .changed();

                if self.settings.steamgrid_db.auth_key.is_some() {
                    changed = changed
                        || ui
                            .selectable_value(&mut self.selected_menu, Menues::Images, "Images")
                            .changed();
                }
                changed = changed
                    || ui
                        .selectable_value(&mut self.selected_menu, Menues::Settings, "Settings")
                        .changed();

                changed = changed
                    || ui
                        .selectable_value(&mut self.selected_menu, Menues::Backup, "Backup")
                        .changed();

                changed = changed
                    || ui
                        .selectable_value(&mut self.selected_menu, Menues::Disconnect, "Disconnect")
                        .changed();
                if self.selected_menu == Menues::Import {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        self.render_import_button(ui);
                    });
                }
                if changed {
                    self.backup_state.available_backups = None;
                }
                if changed
                    && menu_before == Menues::Settings
                    && self.selected_menu == Menues::Import
                {
                    //We reset games here, since user might change settings
                    self.games_to_sync = create_games_to_sync(&mut self.rt, &self.platforms);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_menu {
                Menues::Import => {
                    self.render_import_games(ui);
                }
                Menues::Settings => {
                    self.render_settings(ui);
                }
                Menues::Images => {
                    self.render_ui_images(ui);
                }
                Menues::Backup => {
                    self.render_backup(ui);
                }
                Menues::Disconnect => {
                    self.render_disconnect(ui);
                }
            };
        });

        if self.selected_menu == Menues::Settings {
            egui::TopBottomPanel::new(egui::panel::TopBottomSide::Bottom, "Bottom Panel")
                .frame(frame)
                .show(ctx, |ui| {
                    let image = egui::include_image!("../../resources/save.png");
                    let mut size = image.texture_size().unwrap_or([32.0, 32.0].into());
                    if size.x <= 0.0 || size.y <= 0.0 {
                        size = egui::vec2(32.0, 32.0);
                    }
                    let save_button = ImageButton::new(image);
                    if ui
                        .add_sized(size * 0.5, save_button)
                        .on_hover_text("Save settings")
                        .clicked()
                    {
                        if let Err(err) = save_settings(&self.settings, &self.platforms) {
                            eprintln!("Failed to save settings: {err:?}");
                        }
                    }
                });
        }
    }
}

fn create_style(style: &mut egui::Style) {
    style.spacing.item_spacing = egui::vec2(15.0, 15.0);
    style.visuals.button_frame = false;
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = BACKGROUND_COLOR;
    style.visuals.override_text_color = Some(TEXT_COLOR);
    style.visuals.widgets.noninteractive.rounding = Rounding {
        ne: 0.0,
        nw: 0.0,
        se: 0.0,
        sw: 0.0,
    };
    style.visuals.faint_bg_color = PURLPLE;
    style.visuals.extreme_bg_color = EXTRA_BACKGROUND_COLOR;
    style.visuals.widgets.active.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.active.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.active.fg_stroke = Stroke::new(2.0, LIGHT_ORANGE);
    style.visuals.widgets.open.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.open.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.open.fg_stroke = Stroke::new(2.0, LIGHT_ORANGE);
    style.visuals.widgets.noninteractive.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(2.0, ORANGE);
    style.visuals.widgets.inactive.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, ORANGE);
    style.visuals.widgets.hovered.bg_fill = BACKGROUND_COLOR;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(2.0, BG_STROKE_COLOR);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, LIGHT_ORANGE);
    style.visuals.selection.bg_fill = PURLPLE;
}
fn setup(ctx: &egui::Context) {
    let mut style: egui::Style = (*ctx.style()).clone();
    create_style(&mut style);
    ctx.set_style(style);
    egui_extras::install_image_loaders(ctx);
}
pub fn run_sync() -> eyre::Result<()> {
    let mut app = MyEguiApp::new()?;
    while !all_ready(&app.games_to_sync) {
        println!("Finding games, trying again in 500ms");
        std::thread::sleep(Duration::from_secs_f32(0.5));
    }
    app.run_sync_blocking()
}

pub fn run_ui(args: Vec<String>) -> eyre::Result<()> {
    let app = MyEguiApp::new()?;
    let no_v_sync = args.contains(&"--no-vsync".to_string());
    let fullscreen = is_fullscreen(&args);
    let logo = get_logo_icon();
    let viewport = egui::ViewportBuilder { fullscreen: Some(fullscreen), icon: Some(logo.into()), ..Default::default() };
    let native_options = eframe::NativeOptions {
        viewport,
        vsync: !no_v_sync,
        ..Default::default()
    };
    let run_result = eframe::run_native(
        "BoilR",
        native_options,
        Box::new(|cc| {
            setup(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    );
    run_result.map_err(|e| eyre::eyre!("Could not initialize: {:?}", e))
}

fn is_fullscreen(args: &[String]) -> bool {
    let is_steam_mode = match std::env::var("SteamAppId") {
        Ok(value) => !value.is_empty(),
        Err(_) => false,
    };
    is_steam_mode || args.contains(&"--fullscreen".to_string())
}