# GTA 6 graphics profile: RTX 3050 8 GB

AetherX includes an editable starting profile for an RTX 3050 with 8 GB VRAM.

## Recommended starting point

| Setting | Starting value | Why |
|---|---|---|
| Output | 1920x1080 | Native monitor resolution |
| Upscaler | DLSS Quality | Start with image quality; compare Balanced if GPU-limited |
| Textures | Medium | Conservative VRAM budget |
| Shadows | Medium | Reduce a costly setting without immediately flattening the image |
| Reflections | Low | Reflections can be expensive |
| Volumetrics | Low | Fog, clouds and volumetric lighting can be GPU-heavy |
| Ray tracing | Off | Avoid a large unknown performance cost on this GPU |
| Motion blur | Off | Cleaner motion; preference setting |
| Film grain | Off | Cleaner image |
| Sharpening | 0.25 | Mild sharpening; adjust to taste |
| FPS cap | 60 | Initial frame-pacing target, not a performance guarantee |

These are cautious starting recommendations, not measured GTA 6 settings. The PC version's actual graphics menu, VRAM use, CPU/GPU load and frame-time behavior must be tested before final tuning. If the game does not offer DLSS or a listed setting, ignore that entry.

## How to tune

1. Start at 1080p with DLSS Quality and ray tracing off.
2. Benchmark the same scene for at least 60 seconds; record average FPS, 1% lows, frame times, VRAM use and GPU utilization.
3. If GPU utilization is consistently near full and frame times are high, test DLSS Balanced before lowering textures.
4. If VRAM is near its limit or streaming stutters occur, lower texture quality and verify again.
5. Change one setting at a time. Keep the best image quality that meets the desired frame-time target.
6. Do not enable frame generation as a substitute for fixing poor base frame times.

## Current AetherX integration

The desktop app lets the user edit and save this recommendation as:

%LOCALAPPDATA%\\AetherX\\profiles\\gta6-rtx3050.ini

This file is a profile/export only. AetherX does not currently inject settings into a game, patch GTA 6, force DLSS, alter NVIDIA driver settings, or promise an FPS increase. Direct application requires a supported PC release and a verified game-specific configuration interface. No GTA 6 PC benchmark has been run as part of this change.
