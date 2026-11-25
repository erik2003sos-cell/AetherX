#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::VecDeque, fs, path::PathBuf, time::Instant};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, TextureOptions, Vec2};

const TEST_SECONDS: f32 = 60.0;

struct BenchmarkApp {
    started: Instant,
    last_frame: Instant,
    frame_times: VecDeque<f32>,
    texture: TextureHandle,
    scene_load: usize,
    audio_status: String,
    launch_note: String,
    finished: bool,
    frames: u64,
}

impl BenchmarkApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let size = [256, 256];
        let mut pixels = Vec::with_capacity(size[0] * size[1]);
        for y in 0..size[1] {
            for x in 0..size[0] {
                let mortar = y % 32 < 3 || x % 48 < 3;
                let window = (x % 48 > 8 && x % 48 < 38) && (y % 32 > 7 && y % 32 < 26);
                let color = if mortar { Color32::from_rgb(43, 47, 55) }
                else if window && ((x / 48 + y / 32) % 3 == 0) { Color32::from_rgb(238, 183, 104) }
                else if window { Color32::from_rgb(39, 65, 83) }
                else { Color32::from_rgb(111, 119, 128) };
                pixels.push(color);
            }
        }
        let image = egui::ColorImage::new(size, pixels);
        let texture = cc.egui_ctx.load_texture("aetherx-city-facade", image, TextureOptions::LINEAR);
        let launch_note = if std::env::args().any(|arg| arg == "--launched-by-aetherx") {
            "Started by AetherX launcher".to_owned()
        } else { "Started directly".to_owned() };
        Self {
            started: Instant::now(), last_frame: Instant::now(),
            frame_times: VecDeque::with_capacity(1200), texture, scene_load: 3,
            audio_status: "Not tested yet".to_owned(), launch_note, finished: false, frames: 0,
        }
    }

    fn average_fps(&self) -> f32 {
        let elapsed = self.started.elapsed().as_secs_f32().min(TEST_SECONDS);
        if elapsed <= 0.0 { 0.0 } else { self.frames as f32 / elapsed }
    }

    fn one_percent_low(&self) -> f32 {
        if self.frame_times.is_empty() { return 0.0; }
        let mut times: Vec<f32> = self.frame_times.iter().copied().collect();
        times.sort_by(|a, b| b.total_cmp(a));
        let count = ((times.len() as f32 * 0.01).ceil() as usize).max(1);
        let slow_average = times.iter().take(count).sum::<f32>() / count as f32;
        if slow_average > 0.0 { 1.0 / slow_average } else { 0.0 }
    }

    fn save_report(&self) -> anyhow::Result<PathBuf> {
        let dir = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir).join("AetherX").join("benchmarks");
        fs::create_dir_all(&dir)?;
        let path = dir.join("latest-benchmark.txt");
        let elapsed = self.started.elapsed().as_secs_f32().min(TEST_SECONDS);
        let avg_ms = if self.frame_times.is_empty() { 0.0 } else {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32 * 1000.0
        };
        let report = format!(
            "AetherX GTA-style compatibility benchmark\nDuration: {:.1}s / {:.0}s\nFrames sampled: {}\nAverage app frame rate: {:.1} FPS\n1% low app frame rate: {:.1} FPS\nAverage sampled frame interval: {:.2} ms\nTexture: procedural 256x256 facade texture loaded and rendered through egui/OpenGL\nAudio playback test: {}\nLauncher: {}\nScene load level: {}/5\n\nLimitations: this is a synthetic 2D/2.5D egui/OpenGL workload, not GTA 6, Unreal Engine 5, DirectX 12, or a GPU-only benchmark. FPS is measured as app frame intervals; it is not a guaranteed in-game FPS prediction. GPU utilization and VRAM usage are not measured.\n",
            elapsed, TEST_SECONDS, self.frames, self.average_fps(), self.one_percent_low(), avg_ms,
            self.audio_status, self.launch_note, self.scene_load
        );
        fs::write(&path, report)?;
        Ok(path)
    }

    fn test_audio(&mut self) {
        #[cfg(windows)]
        { self.audio_status = match play_test_tone() {
            Ok(()) => "PASS — Windows audio device played a 440 Hz test tone".to_owned(),
            Err(error) => format!("FAIL — {error}"),
        }; }
        #[cfg(not(windows))]
        { self.audio_status = "UNAVAILABLE — audio playback test is currently Windows-only".to_owned(); }
    }

    fn draw_city(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, Color32::from_rgb(111, 157, 180));
        let horizon = rect.top() + rect.height() * 0.38;
        painter.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), horizon), rect.right_bottom()), 0.0, Color32::from_rgb(57, 78, 75));
        painter.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), horizon + rect.height() * 0.36), rect.right_bottom()), 0.0, Color32::from_rgb(46, 49, 54));

        let cols = 5 + self.scene_load * 3;
        let rows = 2 + self.scene_load;
        for row in 0..rows {
            for col in 0..cols {
                let depth = row as f32 / rows as f32;
                let w = rect.width() / (cols as f32 + 1.0) * (0.55 + depth * 0.55);
                let h = rect.height() * (0.13 + ((col * 7 + row * 11) % 9) as f32 * 0.018) * (0.6 + depth * 0.6);
                let x = rect.left() + (col as f32 + 0.35) * rect.width() / cols as f32;
                let y = horizon - h * 0.72 + depth * rect.height() * 0.16;
                let building = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                painter.rect_filled(building.translate(Vec2::new(7.0, 8.0)), 1.0, Color32::from_rgba_unmultiplied(15, 19, 23, 90));
                painter.image(self.texture.id(), building, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                painter.rect_filled(Rect::from_min_size(Pos2::new(building.left(), building.top()), Vec2::new(building.width(), 5.0)), 0.0, Color32::from_rgb(45, 51, 59));
            }
        }

        let road_y = horizon + rect.height() * 0.43;
        painter.rect_filled(Rect::from_min_max(Pos2::new(rect.left(), road_y), Pos2::new(rect.right(), road_y + rect.height() * 0.25)), 0.0, Color32::from_rgb(38, 40, 44));
        for i in 0..12 {
            let x = rect.left() + (i as f32 / 12.0) * rect.width();
            painter.rect_filled(Rect::from_min_size(Pos2::new(x, road_y + rect.height() * 0.12), Vec2::new(rect.width() * 0.035, 4.0)), 1.0, Color32::from_rgb(220, 203, 150));
        }
        let t = self.started.elapsed().as_secs_f32();
        for i in 0..(self.scene_load * 8) {
            let lane = i % 3;
            let speed = 34.0 + (i % 7) as f32 * 13.0;
            let travel = (t * speed + i as f32 * 79.0) % (rect.width() + 100.0);
            let x = rect.left() + travel - 50.0;
            let y = road_y + 13.0 + lane as f32 * rect.height() * 0.055;
            let car = Rect::from_min_size(Pos2::new(x, y), Vec2::new(34.0 + (i % 3) as f32 * 7.0, 13.0));
            let color = [Color32::from_rgb(185, 63, 48), Color32::from_rgb(49, 108, 151), Color32::from_rgb(204, 195, 169)][i % 3];
            painter.rect_filled(car, 3.0, color);
            painter.rect_filled(Rect::from_min_size(Pos2::new(car.left() + 7.0, car.top() + 2.0), Vec2::new(car.width() * 0.48, 5.0)), 1.0, Color32::from_rgb(150, 190, 199));
        }
        for i in 0..(self.scene_load * 160) {
            let x = rect.left() + ((i * 37) % 997) as f32 / 997.0 * rect.width();
            let y = horizon + ((i * 53) % 431) as f32 / 431.0 * rect.height() * 0.54;
            painter.circle_filled(Pos2::new(x, y), 2.0 + (i % 5) as f32, Color32::from_rgba_unmultiplied(205, 211, 190, 95));
        }
    }
}

impl eframe::App for BenchmarkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().clamp(0.0001, 1.0);
        self.last_frame = now;
        if !self.finished && self.started.elapsed().as_secs_f32() < TEST_SECONDS {
            self.frame_times.push_back(dt);
            if self.frame_times.len() > 1200 { self.frame_times.pop_front(); }
            self.frames += 1;
        }
        if self.started.elapsed().as_secs_f32() >= TEST_SECONDS { self.finished = true; }
        ctx.request_repaint();

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("AETHERX — CITY BENCHMARK");
                ui.separator();
                ui.label("Synthetic GTA-style graphics / launch / audio test");
            });
            ui.label(&self.launch_note);
        });
        egui::SidePanel::left("controls").resizable(false).default_width(260.0).show(ctx, |ui| {
            ui.heading("Live measurements");
            ui.label(format!("Average FPS: {:.1}", self.average_fps()));
            ui.label(format!("1% low: {:.1} FPS", self.one_percent_low()));
            let avg_ms = if self.frame_times.is_empty() { 0.0 } else { self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32 * 1000.0 };
            ui.label(format!("Average frame interval: {:.2} ms", avg_ms));
            ui.label(format!("Frames sampled: {}", self.frames));
            let elapsed = self.started.elapsed().as_secs_f32().min(TEST_SECONDS);
            ui.add(egui::ProgressBar::new(elapsed / TEST_SECONDS).text(format!("Benchmark: {:.0}/{:.0}s", elapsed, TEST_SECONDS)));
            if self.finished { ui.colored_label(Color32::LIGHT_GREEN, "60-second run complete"); }
            ui.separator();
            ui.heading("Workload");
            ui.add(egui::Slider::new(&mut self.scene_load, 1..=5).text("Scene load"));
            ui.label(format!("{} building columns, {} animated cars", 5 + self.scene_load * 3, self.scene_load * 8));
            ui.separator();
            ui.heading("Texture test");
            ui.label("256×256 procedural facade texture");
            ui.label("Loaded once and repeatedly rendered");
            ui.separator();
            ui.heading("Audio test");
            if ui.button("▶ Play test tone").clicked() { self.test_audio(); }
            ui.label(&self.audio_status);
            ui.separator();
            if ui.button("Save benchmark report").clicked() {
                match self.save_report() {
                    Ok(path) => ui.label(format!("Saved: {}", path.display())),
                    Err(error) => ui.colored_label(Color32::LIGHT_RED, format!("Could not save report: {error}")),
                };
            }
            if ui.button("Restart 60-second test").clicked() {
                self.started = Instant::now(); self.last_frame = self.started;
                self.frame_times.clear(); self.frames = 0; self.finished = false;
            }
            ui.add_space(10.0);
            ui.small("Not GTA 6 / UE5. This measures AetherX's own egui/OpenGL scene, not DirectX 12 game performance.");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Urban scene — animated traffic and textured buildings");
            let available = ui.available_size().max(Vec2::new(320.0, 240.0));
            let (response, _) = ui.allocate_painter(available, Sense::hover());
            self.draw_city(ui, response.rect);
        });
    }
}

#[cfg(windows)]
fn play_test_tone() -> Result<(), String> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    let sample_rate = 44_100u32;
    let samples = sample_rate;
    let mut wav = Vec::with_capacity(44 + samples as usize * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + samples * 2).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(samples * 2).to_le_bytes());
    for i in 0..samples {
        let phase = i as f32 / sample_rate as f32 * std::f32::consts::TAU * 440.0;
        let envelope = (1.0 - i as f32 / samples as f32).min(1.0);
        let value = (phase.sin() * envelope * 7_000.0) as i16;
        wav.extend_from_slice(&value.to_le_bytes());
    }
    let path = std::env::temp_dir().join("aetherx-audio-test.wav");
    fs::write(&path, wav).map_err(|error| error.to_string())?;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    #[link(name = "winmm")]
    unsafe extern "system" {
        fn PlaySoundW(sound: *const u16, module: *mut c_void, flags: u32) -> i32;
    }
    // SAFETY: valid NUL-terminated UTF-16 path remains alive during synchronous playback.
    let played = unsafe { PlaySoundW(wide.as_ptr(), std::ptr::null_mut(), 0x0002_0000) };
    let _ = fs::remove_file(&path);
    if played == 0 { Err("Windows could not play the generated WAV file".to_owned()) } else { Ok(()) }
}

#[cfg(not(windows))]
fn play_test_tone() -> Result<(), String> { Err("Windows-only audio test".to_owned()) }

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 780.0])
            .with_min_inner_size([900.0, 600.0]).with_title("AetherX City Benchmark"),
        ..Default::default()
    };
    eframe::run_native("AetherX City Benchmark", options, Box::new(|cc| Ok(Box::new(BenchmarkApp::new(cc)))))
}
