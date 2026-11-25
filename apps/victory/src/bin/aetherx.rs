#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[path = "../launch.rs"]
mod launch;
#[path = "../library.rs"]
mod library;

use std::{fs, path::PathBuf};

#[derive(Clone, Copy, PartialEq)]
enum Upscaler { DlssQuality, DlssBalanced, DlssPerformance, Native }
impl Upscaler {
    fn label(self) -> &'static str {
        match self { Self::DlssQuality => "DLSS Quality (start here)", Self::DlssBalanced => "DLSS Balanced", Self::DlssPerformance => "DLSS Performance", Self::Native => "Native resolution" }
    }
    fn as_str(self) -> &'static str {
        match self { Self::DlssQuality => "dlss_quality", Self::DlssBalanced => "dlss_balanced", Self::DlssPerformance => "dlss_performance", Self::Native => "native" }
    }
}

struct GraphicsProfile {
    resolution: &'static str,
    upscaler: Upscaler,
    textures: &'static str,
    shadows: &'static str,
    reflections: &'static str,
    volumetrics: &'static str,
    ray_tracing: bool,
    motion_blur: bool,
    film_grain: bool,
    sharpening: f32,
    fps_cap: u32,
}
impl Default for GraphicsProfile {
    fn default() -> Self {
        Self { resolution: "1920x1080", upscaler: Upscaler::DlssQuality, textures: "Medium", shadows: "Medium", reflections: "Low", volumetrics: "Low", ray_tracing: false, motion_blur: false, film_grain: false, sharpening: 0.25, fps_cap: 60 }
    }
}
impl GraphicsProfile {
    fn save(&self) -> anyhow::Result<PathBuf> {
        let dir = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("AetherX").join("profiles");
        fs::create_dir_all(&dir)?;
        let path = dir.join("gta6-rtx3050.ini");
        let body = format!(
            "# AetherX GTA 6 starting profile — RTX 3050 8 GB, 1080p\n# Recommendations only: AetherX does not inject settings into the game.\nresolution={}\nupscaler={}\ntextures={}\nshadows={}\nreflections={}\nvolumetrics={}\nray_tracing={}\nmotion_blur={}\nfilm_grain={}\nsharpening={:.2}\nfps_cap={}\n",
            self.resolution, self.upscaler.as_str(), self.textures, self.shadows, self.reflections, self.volumetrics, self.ray_tracing, self.motion_blur, self.film_grain, self.sharpening, self.fps_cap
        );
        fs::write(&path, body)?;
        Ok(path)
    }
}

struct AetherXApp {
    games: Vec<PathBuf>,
    selected: Option<usize>,
    status: String,
    graphics: GraphicsProfile,
}
impl AetherXApp {
    fn new() -> Self {
        let games = library::discover_games(&library::default_roots());
        let count = games.len();
        Self { games, selected: None, status: format!("Ready — found {count} games"), graphics: GraphicsProfile::default() }
    }
    fn refresh(&mut self) {
        self.games = library::discover_games(&library::default_roots());
        self.selected = None;
        self.status = format!("Library refreshed — found {} games", self.games.len());
    }
    fn play_selected(&mut self) {
        let Some(index) = self.selected else { self.status = "Select a game first.".to_owned(); return; };
        let Some(exe) = self.games.get(index) else { self.status = "The selected game is no longer in the library.".to_owned(); return; };
        match launch::run(exe, &[]) {
            Ok(()) => {
                let name = exe.file_stem().and_then(|s| s.to_str()).unwrap_or("Game");
                self.status = format!("Started {name} — native rendering; no frame-generation overlay");
            }
            Err(error) => self.status = format!("Could not start game: {error:#}"),
        }
    }
    fn save_graphics(&mut self) {
        self.status = match self.graphics.save() {
            Ok(path) => format!("GTA 6 profile saved: {}", path.display()),
            Err(error) => format!("Could not save graphics profile: {error:#}"),
        };
    }
}
impl eframe::App for AetherXApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use eframe::egui;
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AETHERX");
            ui.label("PC game library and graphics profiles");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("↻ Refresh library").clicked() { self.refresh(); }
                if ui.add_enabled(self.selected.is_some(), egui::Button::new("▶ PLAY")).clicked() { self.play_selected(); }
            });
            ui.separator();
            ui.label(format!("{} games found", self.games.len()));
            egui::ScrollArea::vertical().max_height(145.0).show(ui, |ui| {
                for (index, path) in self.games.iter().enumerate() {
                    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");
                    let selected = self.selected == Some(index);
                    if ui.selectable_label(selected, name).clicked() { self.selected = Some(index); }
                    if selected { ui.small(path.display().to_string()); }
                }
            });
            ui.separator();
            ui.heading("GTA 6 — RTX 3050 8 GB profile");
            ui.small("Conservative 1080p starting point. Tune after the actual PC release and benchmark; no GTA 6 PC settings interface is available to validate yet.");
            egui::Grid::new("gta6_graphics").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
                ui.label("Output resolution");
                egui::ComboBox::from_id_salt("resolution").selected_text(self.graphics.resolution).show_ui(ui, |ui| {
                    for value in ["1920x1080", "1600x900", "1280x720"] { ui.selectable_value(&mut self.graphics.resolution, value, value); }
                }); ui.end_row();
                ui.label("Upscaling");
                egui::ComboBox::from_id_salt("upscaler").selected_text(self.graphics.upscaler.label()).show_ui(ui, |ui| {
                    for value in [Upscaler::DlssQuality, Upscaler::DlssBalanced, Upscaler::DlssPerformance, Upscaler::Native] { ui.selectable_value(&mut self.graphics.upscaler, value, value.label()); }
                }); ui.end_row();
                for (label, value) in [("Textures", &mut self.graphics.textures), ("Shadows", &mut self.graphics.shadows), ("Reflections", &mut self.graphics.reflections), ("Volumetrics", &mut self.graphics.volumetrics)] {
                    ui.label(label);
                    egui::ComboBox::from_id_salt(label).selected_text(*value).show_ui(ui, |ui| {
                        for level in ["Low", "Medium", "High"] { ui.selectable_value(value, level, level); }
                    }); ui.end_row();
                }
                ui.label("Ray tracing"); ui.checkbox(&mut self.graphics.ray_tracing, "Off (recommended starting point)"); ui.end_row();
                ui.label("Motion blur"); ui.checkbox(&mut self.graphics.motion_blur, "Enable"); ui.end_row();
                ui.label("Film grain"); ui.checkbox(&mut self.graphics.film_grain, "Enable"); ui.end_row();
                ui.label("Sharpening"); ui.add(egui::Slider::new(&mut self.graphics.sharpening, 0.0..=1.0)); ui.end_row();
                ui.label("FPS cap"); ui.add(egui::Slider::new(&mut self.graphics.fps_cap, 30..=120).suffix(" FPS")); ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.button("Save GTA 6 profile").clicked() { self.save_graphics(); }
                if ui.button("Reset recommended profile").clicked() { self.graphics = GraphicsProfile::default(); self.status = "Recommended RTX 3050 profile restored.".to_owned(); }
            });
            ui.separator();
            ui.label(&self.status);
            ui.small("Profile is saved locally as an INI recommendation. AetherX does not yet apply settings to GTA 6, force DLSS, alter drivers, or guarantee FPS. Frame generation remains removed.");
        });
    }
}
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([800.0, 790.0]).with_min_inner_size([620.0, 620.0]).with_title("AetherX"),
        ..Default::default()
    };
    eframe::run_native("AetherX", options, Box::new(|_cc| Ok(Box::new(AetherXApp::new()))))
}
