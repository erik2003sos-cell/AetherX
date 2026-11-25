#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../launch.rs"]
mod launch;
#[path = "../library.rs"]
mod library;

use std::path::PathBuf;

struct AetherXApp {
    games: Vec<PathBuf>,
    selected: Option<usize>,
    status: String,
    frame_generation_2x: bool,
}

impl AetherXApp {
    fn new() -> Self {
        let games = library::discover_games(&library::default_roots());
        let count = games.len();
        Self {
            games,
            selected: None,
            status: format!("Ready — found {count} games"),
            frame_generation_2x: load_frame_generation_setting(),
        }
    }

    fn refresh(&mut self) {
        self.games = library::discover_games(&library::default_roots());
        self.selected = None;
        self.status = format!("Library refreshed — found {} games", self.games.len());
    }

    fn set_frame_generation(&mut self, enabled: bool) {
        self.frame_generation_2x = enabled;
        match save_frame_generation_setting(enabled) {
            Ok(()) => {
                self.status = if enabled {
                    "2× Frame Generation requested. The rendering backend is still under development; this does not generate frames yet.".to_owned()
                } else {
                    "2× Frame Generation disabled.".to_owned()
                };
            }
            Err(error) => {
                self.status = format!("Setting changed for this session, but could not be saved: {error}");
            }
        }
    }

    fn play_selected(&mut self) {
        let Some(index) = self.selected else {
            self.status = "Select a game first.".to_owned();
            return;
        };
        let Some(exe) = self.games.get(index) else {
            self.status = "The selected game is no longer in the library.".to_owned();
            return;
        };
        match launch::run_with_frame_generation(exe, &[], self.frame_generation_2x) {
            Ok(()) => {
                let name = exe.file_stem().and_then(|s| s.to_str()).unwrap_or("Game");
                self.status = if self.frame_generation_2x {
                    format!("Started {name} — 2× Frame Generation requested (backend not active yet)")
                } else {
                    format!("Started {name}")
                };
            }
            Err(error) => self.status = format!("Could not start game: {error:#}"),
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("AetherX").join("settings.ini"))
}

fn load_frame_generation_setting() -> bool {
    let Some(path) = settings_path() else {
        return true;
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == "frame_generation_2x").then(|| value.trim() != "false")
            })
        })
        .unwrap_or(true)
}

fn save_frame_generation_setting(enabled: bool) -> std::io::Result<()> {
    let Some(path) = settings_path() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "LOCALAPPDATA is not set",
        ));
    };
    let parent = path.parent().expect("settings path has a parent");
    std::fs::create_dir_all(parent)?;
    std::fs::write(
        path,
        format!("frame_generation_2x={}\n", if enabled { "true" } else { "false" }),
    )
}

impl eframe::App for AetherXApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use eframe::egui;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AETHERX");
            ui.label("Your PC game library");
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button("↻  Refresh library").clicked() {
                    self.refresh();
                }
                let can_play = self.selected.is_some();
                if ui.add_enabled(can_play, egui::Button::new("▶  PLAY")).clicked() {
                    self.play_selected();
                }
            });

            ui.separator();
            ui.label(format!("{} games found", self.games.len()));
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(320.0)
                .show(ui, |ui| {
                    for (index, path) in self.games.iter().enumerate() {
                        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");
                        let selected = self.selected == Some(index);
                        if ui.selectable_label(selected, name).clicked() {
                            self.selected = Some(index);
                        }
                        if selected {
                            ui.small(path.display().to_string());
                        }
                    }
                });

            ui.separator();
            ui.heading("Performance");
            let mut enabled = self.frame_generation_2x;
            if ui.checkbox(&mut enabled, "Enable 2× Frame Generation for launched games").changed() {
                self.set_frame_generation(enabled);
            }
            ui.small("Universal D3D12 frame-generation backend is under development. The setting is saved and passed to launched games, but generated frames are not active yet.");
            ui.add_space(8.0);
            ui.label(&self.status);
            ui.add_space(4.0);
            ui.small("AetherX launches native Windows games. Unsupported systems must fall back safely; no CPU/GPU can be promised support until tested.");
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([760.0, 620.0])
            .with_min_inner_size([520.0, 420.0])
            .with_title("AetherX"),
        ..Default::default()
    };
    eframe::run_native(
        "AetherX",
        options,
        Box::new(|_cc| Ok(Box::new(AetherXApp::new()))),
    )
}
