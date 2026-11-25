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
}

impl AetherXApp {
    fn new() -> Self {
        let games = library::discover_games(&library::default_roots());
        let count = games.len();
        Self {
            games,
            selected: None,
            status: format!("Ready — found {count} games"),
        }
    }

    fn refresh(&mut self) {
        self.games = library::discover_games(&library::default_roots());
        self.selected = None;
        self.status = format!("Library refreshed — found {} games", self.games.len());
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
        match launch::run_with_frame_generation(exe, &[], true) {
            Ok(()) => {
                let name = exe.file_stem().and_then(|s| s.to_str()).unwrap_or("Game");
                self.status = format!(
                    "Started {name} — AetherX 2× Frame Generation is always enabled by default (backend integration still in progress)"
                );
            }
            Err(error) => self.status = format!("Could not start game: {error:#}"),
        }
    }
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
            ui.label("2× Frame Generation: ALWAYS ON");
            ui.small("AetherX requests 2× Frame Generation for every game launched from the app. The universal rendering backend is still under development; generated frames are not active yet.");
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
