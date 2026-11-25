# AetherX City Benchmark

The benchmark is a separate Windows desktop executable that AetherX can launch from its main window. It exercises a repeatable, synthetic urban scene and checks a few host capabilities independently.

## Build and run

Build both desktop executables from the repository root:

```powershell
cargo build --release -p victory --bin aetherx --bin aetherx_benchmark
.\target\release\aetherx.exe
```

Use **Run GTA-style benchmark** in AetherX. The benchmark executable must be beside `aetherx.exe` (the Cargo command above puts both in the same release folder).

## What it tests

- **Launch:** AetherX starts the benchmark as a separate Windows process and reports the child PID. The benchmark window identifies whether it was started by AetherX.
- **Textures:** a generated 256×256 facade texture is uploaded to egui's renderer and repeatedly drawn across building surfaces.
- **Scene rendering:** a deterministic 2.5D city scene with buildings, shadows, road markings, animated traffic and additional geometry. Scene load can be adjusted from 1 to 5.
- **Frame pacing:** app frame intervals, average app FPS and a sampled 1% low are collected for a 60-second run.
- **Audio:** the Windows audio device plays a generated 440 Hz WAV tone. This confirms basic playback, not spatial audio, surround, mixing, or low-latency game audio.
- **Report:** save a text report to `%LOCALAPPDATA%\AetherX\benchmarks\latest-benchmark.txt`.

## Important limitations

This is a synthetic egui/OpenGL workload, not a GTA 6 clone or a benchmark of GTA 6's engine. It does not use Unreal Engine 5, DirectX 12, real-world game assets, ray tracing, DLSS, or GTA 6's texture streaming system. Its FPS measures the benchmark window's frame intervals, not game FPS. It does not currently measure GPU utilization, CPU utilization, VRAM use, or audio latency. Do not use its FPS number to predict GTA 6 performance.

The test is intended to verify that AetherX can launch a separate app, upload and render a texture, sustain a repeatable animated scene, collect frame timing, and play a basic Windows audio sample.
