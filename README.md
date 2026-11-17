# Nerthus

Native cross-platform build of a 2D tile sandbox engine — worldgen, pixel lighting, AABB
physics, mining/building. Rust + SDL2, one codebase, no OS-specific code.

Ships as a **self-contained binary** (SDL2 is compiled in via the `bundled` feature), so the
released artifacts need no runtime dependencies.

## Downloads

Prebuilt binaries are produced by GitHub Actions on **native runners** for each platform —
nothing is emulated or cross-compiled, so each artifact is built and self-tested on real
hardware:

| Platform | Artifact |
| --- | --- |
| macOS (Universal: Intel + Apple Silicon) | `nerthus-macos-universal` and `nerthus-macos-universal.app.zip` |
| Windows x86_64 | `nerthus-windows-x86_64` |
| Linux x86_64 | `nerthus-linux-x86_64` |
| Linux ARM64 | `nerthus-linux-arm64` |

Every artifact ships with:
- the binary (SDL2 statically linked — no runtime deps)
- `*-frame.png` — a reference frame rendered by that exact binary
- `*-README.md`
- `*.sha256` checksum

Grab them from the **Actions** tab (latest `build` run) or from **Releases** if a `v*` tag
was pushed.

### macOS

Double-click `Nerthus.app` after unzipping. Because the build is ad-hoc signed and not
notarized, Gatekeeper may still object; either right-click → Open, or:

```sh
xattr -dr com.apple.quarantine Nerthus.app
```

Prefer the raw binary? `nerthus-macos-universal` runs directly and is a real universal
Mach-O (`lipo`-merged from both slices).

### Windows

Run `nerthus-windows-x86_64.exe`. If SmartScreen warns, choose *More info → Run anyway* — the
binary is unsigned.

## Run from source

```sh
cargo run --release                          # random seed
cargo run --release -- --seed=424242         # reproducible world
```

Only needs Rust stable. SDL2 is vendored and built automatically, so no system SDL install.

## Verify without a display

```sh
cargo run --release -- --selftest            # asserts worldgen, physics, lighting, timing
cargo run --release -- --dump-frame out.png  # render one frame to PNG
```

`--selftest` is headless and CI-friendly — the build workflow runs it before publishing
binaries, so a broken engine can never ship. `--dump-frame` writes a PNG straight from the
software framebuffer, so it works over SSH with no X server.

## Controls

| Input | Action |
| --- | --- |
| `A` / `D` or ← / → | move |
| `W` / `Space` / ↑ | jump |
| Left mouse (hold) | mine |
| `Shift` + left mouse | place plank |
| `Esc` | quit |

## Architecture

| File | Role |
| --- | --- |
| `src/util.rs` | tile enum, hash noise, FBM, seeded PRNG |
| `src/world.rs` | terrain, caves, ore veins, trees, spawn resolution |
| `src/light.rs` | bucketed BFS light propagation with distance attenuation |
| `src/player.rs` | axis-separated AABB collision, coyote time, friction |
| `src/render.rs` | procedural atlas, software framebuffer, bilinear light composite |
| `src/main.rs` | SDL window/loop, camera, mining, PNG writer, selftest |

## Performance

The renderer is a pure software rasterizer. Two things keep it fast:

1. **Internal resolution + hardware upscale.** The world renders at half the window size
   (640×400 for a 1280×800 window) and SDL stretches the texture. That is 4× fewer pixels for
   no visible loss, since the art is 16px tiles.
2. **Span rasterization.** Tile blits and the light composite write horizontal runs into the
   framebuffer instead of per-pixel calls, and the light composite precomputes its horizontal
   sample table once per frame rather than per pixel.

Measured on this ARM64 machine, release build:

| Metric | Value |
| --- | --- |
| Full game frame @ 640×400 internal | **~2.0–2.4 ms** (≈420–500 fps equivalent) |
| Lighting solve, 121×81 tile region | ~265–530 µs |
| Worldgen, 420×300 world | deterministic, seed-sensitive, no dead columns |
| Binary size, statically linked SDL2 | 3.0 MB |

For reference the same scene cost ~9 ms/frame before these changes.

## Notes on the port

`hash2` uses unsigned (`u32`) wrapping arithmetic to match the browser build exactly. Using
signed `i32` arithmetic silently produces a completely different world — it showed up as 0%
ore and 71% stone instead of ~7% and ~45%.

![frame](screenshots/nerthus.png)
