# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```sh
cargo run            # run in debug mode (shows origin-zone debug rect)
cargo run --release  # run without debug overlays
cargo build          # compile only
cargo check          # fast type-check without linking
cargo clippy         # lint
```

There are no tests at this time.

## Architecture

Single-binary Bevy 0.19 app (`src/main.rs`). Entry point wires `DefaultPlugins` plus `GraphPlugin`.

**GraphPlugin** registers three systems:
- `setup` (Startup) — spawns a `Camera2d`
- `draw_infinite_grid` (Update) — renders an infinite coordinate grid using `Gizmos`; grid cells snap to camera position so lines appear fixed while the camera moves
- `handle_zoom_input` (Update) — pinch/scroll zoom that keeps the world point under the cursor fixed; snaps zoom pivot to origin when cursor is within `ORIGIN_ZONE_PADDING` of world (0,0)
- `handle_pan_input` (Update) — left-click drag pans the camera

**Grid rendering** (`draw_infinite_grid`):
- `halve_step(camera_scale)` produces a power-of-2 cell scale so the grid doubles cleanly on zoom
- `subdivision_count(cell_size)` picks 4 or 5 subdivisions based on the decade mantissa of the cell size
- Two `gizmos.grid_2d` calls render major cells and subdivided subcells; two `gizmos.line_2d` calls render the bold X/Y axes

**Leftover files** — `src/convert.rs` and `src/linalg.rs` contain nannou-era utilities (coordinate conversion, RK4 integrator) that are not currently used by `main.rs`. They reference `nannou` types and will fail to compile if included in `mod` declarations; treat them as reference material for a future particle/wave simulation layer.

## Key constants (src/main.rs)

| Constant | Purpose |
|---|---|
| `DEFAULT_CELL_SIZE` | Base pixel size of a grid cell at scale 1.0 |
| `CAMERA_SCALE_DAMPING` | Tuned for macOS trackpad scroll sensitivity |
| `ORIGIN_ZONE_PADDING` | Radius around origin where zoom pivots to (0,0) instead of cursor |
