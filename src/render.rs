use crate::util::{hash2, Tile, TILE};
use crate::world::World;

pub struct Renderer {
    pub width: i32,
    pub height: i32,
    fb: Vec<u8>,
    atlas: Vec<[u8; 4]>,
}

const VARIANTS: usize = 4;
const ATLAS_TILES: usize = 7;

fn palette(t: Tile) -> Option<([u8; 3], [u8; 3], [u8; 3])> {
    match t {
        Tile::Dirt => Some(([116, 78, 46], [86, 56, 32], [140, 96, 58])),
        Tile::Grass => Some(([86, 140, 54], [58, 104, 36], [116, 170, 72])),
        Tile::Stone => Some(([96, 96, 104], [70, 70, 78], [124, 124, 134])),
        Tile::Wood => Some(([104, 74, 42], [76, 52, 28], [128, 94, 56])),
        Tile::Leaves => Some(([58, 124, 48], [40, 92, 34], [82, 152, 66])),
        Tile::Ore => Some(([96, 96, 104], [70, 70, 78], [196, 168, 74])),
        Tile::Plank => Some(([150, 112, 66], [118, 86, 48], [176, 136, 84])),
        _ => None,
    }
}

impl Renderer {
    pub fn new(width: i32, height: i32) -> Renderer {
        let mut r = Renderer {
            width,
            height,
            fb: vec![0; (width * height * 4) as usize],
            atlas: vec![[0, 0, 0, 0]; ATLAS_TILES * VARIANTS * TILE as usize * TILE as usize],
        };
        r.build_atlas();
        r
    }

    fn build_atlas(&mut self) {
        let tiles = [
            Tile::Dirt,
            Tile::Grass,
            Tile::Stone,
            Tile::Wood,
            Tile::Leaves,
            Tile::Ore,
            Tile::Plank,
        ];
        let tw = TILE as usize;
        let aw = ATLAS_TILES * VARIANTS * tw;
        self.atlas = vec![[0, 0, 0, 0]; aw * tw];

        for (ti, tile) in tiles.iter().enumerate() {
            let (base, shade, hi) = match palette(*tile) {
                Some(p) => p,
                None => continue,
            };
            for v in 0..VARIANTS {
                let ox = (ti * VARIANTS + v) * tw;
                for py in 0..tw {
                    for px in 0..tw {
                        let n = hash2((px + v * 31) as i32, (py + tile.index().unwrap_or(0) * 17) as i32, 1337);
                        let n2 = hash2((px * 3 + v) as i32, (py * 5 + tile.index().unwrap_or(0)) as i32, 99);
                        let c = if n > 0.86 {
                            hi
                        } else if n < 0.16 {
                            shade
                        } else {
                            base
                        };
                        let j = (n2 - 0.5) * 18.0;
                        let idx = (py * aw + ox + px) as usize;
                        self.atlas[idx] = [
                            (c[0] as f32 + j).clamp(0.0, 255.0) as u8,
                            (c[1] as f32 + j).clamp(0.0, 255.0) as u8,
                            (c[2] as f32 + j).clamp(0.0, 255.0) as u8,
                            255,
                        ];
                    }
                }
                if *tile == Tile::Ore {
                    for i in 0..7 {
                        let ax = (hash2(i as i32, v as i32, 7) * 13.0) as i32 + 1;
                        let ay = (hash2(i as i32, v as i32, 11) * 13.0) as i32 + 1;
                        for dy in 0..2 {
                            for dx in 0..2 {
                                let x = ox as i32 + ax + dx;
                                let y = ay + dy;
                                if x < aw as i32 && y < tw as i32 {
                                    self.atlas[(y as usize * aw) + x as usize] = [214, 186, 86, 255];
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[inline]
    fn put(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;
        self.fb[i] = c[0];
        self.fb[i + 1] = c[1];
        self.fb[i + 2] = c[2];
        self.fb[i + 3] = 255;
    }

    #[inline]
    fn span(&mut self, x: i32, y: i32, w: i32, c: [u8; 4]) {
        if y < 0 || y >= self.height || w <= 0 {
            return;
        }
        let x0 = x.max(0);
        let x1 = (x + w).min(self.width);
        if x1 <= x0 {
            return;
        }
        let row = (y * self.width) as usize;
        for xx in x0..x1 {
            let i = (row + xx as usize) * 4;
            self.fb[i] = c[0];
            self.fb[i + 1] = c[1];
            self.fb[i + 2] = c[2];
            self.fb[i + 3] = 255;
        }
    }

    #[inline]
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y..(y + h) {
            self.span(x, yy, w, c);
        }
    }

    pub fn clear(&mut self, c: [u8; 4]) {
        let n = (self.width * self.height) as usize;
        for i in 0..n {
            let o = i * 4;
            self.fb[o] = c[0];
            self.fb[o + 1] = c[1];
            self.fb[o + 2] = c[2];
            self.fb[o + 3] = 255;
        }
    }

    pub fn draw_sky(&mut self, world: &World, cam_x: f32, cam_y: f32, zoom: f32) {
        let day = [96u8, 132, 196];
        let dusk = [34u8, 40, 74];

        for x in 0..self.width {
            let horizon_at = |px: f32| -> f32 {
                let wx = (px / zoom + cam_x) / 16.0;
                let x0 = wx.floor();
                let fx = wx - x0;
                let h0 = world.column_height(x0 as i32) as f32 * 16.0;
                let h1 = world.column_height(x0 as i32 + 1) as f32 * 16.0;
                let surf = h0 + (h1 - h0) * fx;
                (surf - cam_y) * zoom
            };
            let horizon = (horizon_at(x as f32) + horizon_at((x + 2) as f32)) * 0.5;
            let t = (horizon / self.height as f32).clamp(0.0, 1.0);
            let mix = if t > 0.6 { 1.0 } else { t / 0.6 };

            let limit = horizon.clamp(0.0, self.height as f32) as i32;
            for y in 0..limit {
                let f = y as f32 / self.height as f32;
                let r = dusk[0] as f32 + (day[0] as f32 - dusk[0] as f32) * mix * (1.0 - f * 0.55);
                let g = dusk[1] as f32 + (day[1] as f32 - dusk[1] as f32) * mix * (1.0 - f * 0.45);
                let b = dusk[2] as f32 + (day[2] as f32 - dusk[2] as f32) * mix * (1.0 - f * 0.25);
                self.put(
                    x,
                    y,
                    [
                        r.clamp(0.0, 255.0) as u8,
                        g.clamp(0.0, 255.0) as u8,
                        b.clamp(0.0, 255.0) as u8,
                        255,
                    ],
                );
            }
            for y in limit..self.height {
                self.put(x, y, [14, 14, 22, 255]);
            }
        }

        for i in 0..160 {
            let sx = (hash2(i, 3, 11) * self.width as f32) as i32;
            let sy = (hash2(i, 7, 11) * self.height as f32) as i32;
            let wx = (sx as f32 / zoom + cam_x) / 16.0;
            let surface_px = world.column_height(wx as i32) as f32 * 16.0;
            if (sy as f32) > (surface_px - cam_y) * zoom {
                continue;
            }
            let a = (hash2(i, 13, 5) * 90.0 + 140.0) as u8;
            self.put(sx, sy, [a, a, a, 255]);
        }
    }

    pub fn draw_world(
        &mut self,
        world: &World,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
    ) {
        let x0 = ((cam_x / 16.0).floor() as i32 - 1).max(0);
        let y0 = ((cam_y / 16.0).floor() as i32 - 1).max(0);
        let x1 = (((cam_x + self.width as f32 / zoom) / 16.0).ceil() as i32 + 1).min(world.w - 1);
        let y1 = (((cam_y + self.height as f32 / zoom) / 16.0).ceil() as i32 + 1).min(world.h - 1);

        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let tile = world.get(tx, ty);
                if tile == Tile::Air {
                    continue;
                }
                let sx = ((tx as f32 * 16.0 - cam_x) * zoom) as i32;
                let sy = ((ty as f32 * 16.0 - cam_y) * zoom) as i32;
                let span = (16.0 * zoom) as i32;
                if sx + span < 0 || sy + span < 0 {
                    continue;
                }
                if sx >= self.width || sy >= self.height {
                    continue;
                }

                if tile == Tile::Torch {
                    let s = zoom as i32;
                    self.fill_rect(sx + 6 * s, sy + 6 * s, 4 * s, 10 * s, [96, 74, 42, 255]);
                    self.fill_rect(sx + 5 * s, sy + 1 * s, 6 * s, 6 * s, [255, 196, 64, 255]);
                    self.fill_rect(sx + 7 * s, sy + 2 * s, 2 * s, 3 * s, [255, 252, 214, 255]);
                    continue;
                }

                let variant = (hash2(tx, ty, 4242) * VARIANTS as f32) as usize % VARIANTS;
                self.blit_tile_scaled(tile, variant, sx, sy, zoom, [1.0, 1.0, 1.0]);
            }
        }
    }

    /// Bilinear light composite over the whole framebuffer, then additive torch glow.
    pub fn composite_light(
        &mut self,
        light: &crate::light::Lighting,
        world: &World,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
        torches: &[crate::light::TorchLight],
    ) {
        const AMB_R: f32 = 0.055;
        const AMB_G: f32 = 0.065;
        const AMB_B: f32 = 0.105;

        let inv_zoom = 1.0 / zoom;
        let w = world.w as usize;
        let h = world.h as usize;
        debug_assert_eq!(light.r.len(), w * h);

        let mut x_fracs = vec![(0usize, 0.0f32, 0usize); self.width as usize];
        for sx in 0..self.width {
            let wx = (sx as f32 * inv_zoom + cam_x) / 16.0;
            let tx = (wx.floor() as i32).clamp(0, world.w - 1) as usize;
            x_fracs[sx as usize] = (tx, wx - wx.floor(), (tx + 1).min(w - 1));
        }

        for sy in 0..self.height {
            let ly = (sy as f32 * inv_zoom + cam_y) / 16.0;
            let ty_f = ly.floor();
            let fy = ly - ty_f;
            let ty = (ty_f as i32).clamp(0, world.h - 1) as usize;
            let ty1 = (ty + 1).min(h - 1);

            let base0 = ty * w;
            let base1 = ty1 * w;
            let row = (sy * self.width) as usize;

            for sx in 0..self.width {
                let (tx, fx, tx1) = x_fracs[sx as usize];

                let lr = {
                    let a0 = light.r[base0 + tx];
                    let a1 = light.r[base0 + tx1];
                    let b0 = light.r[base1 + tx];
                    let b1 = light.r[base1 + tx1];
                    let top = a0 + (a1 - a0) * fx;
                    let bot = b0 + (b1 - b0) * fx;
                    top + (bot - top) * fy
                };
                let lg = {
                    let a0 = light.g[base0 + tx];
                    let a1 = light.g[base0 + tx1];
                    let b0 = light.g[base1 + tx];
                    let b1 = light.g[base1 + tx1];
                    let top = a0 + (a1 - a0) * fx;
                    let bot = b0 + (b1 - b0) * fx;
                    top + (bot - top) * fy
                };
                let lb = {
                    let a0 = light.b[base0 + tx];
                    let a1 = light.b[base0 + tx1];
                    let b0 = light.b[base1 + tx];
                    let b1 = light.b[base1 + tx1];
                    let top = a0 + (a1 - a0) * fx;
                    let bot = b0 + (b1 - b0) * fx;
                    top + (bot - top) * fy
                };

                let i = (row + sx as usize) * 4;
                let mr = (lr + AMB_R).min(1.0);
                let mg = (lg + AMB_G).min(1.0);
                let mb = (lb + AMB_B).min(1.0);
                self.fb[i] = (self.fb[i] as f32 * mr) as u8;
                self.fb[i + 1] = (self.fb[i + 1] as f32 * mg) as u8;
                self.fb[i + 2] = (self.fb[i + 2] as f32 * mb) as u8;
            }
        }

        for t in torches {
            let cx = ((t.x as f32 * 16.0 + 8.0 - cam_x) * zoom) as i32;
            let cy = ((t.y as f32 * 16.0 + 8.0 - cam_y) * zoom) as i32;
            let rad = (72.0 * zoom) as i32;
            let r2 = (rad * rad) as f32;
            for dy in -rad..=rad {
                let y = cy + dy;
                if y < 0 || y >= self.height {
                    continue;
                }
                let row = (y * self.width) as usize;
                for dx in -rad..=rad {
                    let d2 = (dx * dx + dy * dy) as f32;
                    if d2 > r2 {
                        continue;
                    }
                    let x = cx + dx;
                    if x < 0 || x >= self.width {
                        continue;
                    }
                    let f = 1.0 - (d2 / r2).sqrt();
                    let a = f * f * 0.30;
                    let i = (row + x as usize) * 4;
                    self.fb[i] = ((self.fb[i] as f32 + 255.0 * a).min(255.0)) as u8;
                    self.fb[i + 1] = ((self.fb[i + 1] as f32 + 176.0 * a).min(255.0)) as u8;
                    self.fb[i + 2] = ((self.fb[i + 2] as f32 + 72.0 * a).min(255.0)) as u8;
                }
            }
        }
    }

    fn blit_tile_scaled(&mut self, tile: Tile, variant: usize, dx: i32, dy: i32, zoom: f32, tint: [f32; 3]) {
        let ti = match tile.index() {
            Some(i) => i,
            None => return,
        };
        let tw = TILE as usize;
        let aw = ATLAS_TILES * VARIANTS * tw;
        let ox = (ti * VARIANTS + variant) * tw;
        let s = (zoom as i32).max(1);

        for py in 0..tw {
            let sy = dy + py as i32 * s;
            if sy < 0 || sy >= self.height {
                continue;
            }
            let mut run_start = 0usize;
            let mut run_color: Option<[u8; 4]> = None;
            for px in 0..=tw {
                let color = if px < tw {
                    let src = self.atlas[py * aw + ox + px];
                    if src[3] == 0 {
                        None
                    } else {
                        Some([
                            (src[0] as f32 * tint[0]) as u8,
                            (src[1] as f32 * tint[1]) as u8,
                            (src[2] as f32 * tint[2]) as u8,
                            255,
                        ])
                    }
                } else {
                    None
                };

                if color != run_color {
                    if let Some(c) = run_color {
                        let x = dx + run_start as i32 * s;
                        let w = (px - run_start) as i32 * s;
                        for k in 0..s {
                            self.span(x, sy + k, w, c);
                        }
                    }
                    run_color = color;
                    run_start = px;
                }
            }
        }
    }

    pub fn draw_player(&mut self, player: &crate::player::Player, cam_x: f32, cam_y: f32, zoom: f32) {
        let s = zoom as i32;
        let x = ((player.x - cam_x) * zoom) as i32;
        let y = ((player.y - cam_y) * zoom) as i32;
        let w = 12 * s;
        let h = 28 * s;

        self.fill_rect(x + s, y + 12 * s, w - 2 * s, h - 12 * s, [64, 88, 168, 255]);
        self.fill_rect(x + 2 * s, y + 2 * s, w - 4 * s, 10 * s, [214, 178, 140, 255]);
        self.fill_rect(x + 2 * s, y + 1 * s, w - 4 * s, 4 * s, [48, 38, 32, 255]);
        let eo = if player.facing > 0 { 2 * s } else { 0 };
        self.fill_rect(x + 3 * s + eo, y + 6 * s, 2 * s, 2 * s, [24, 20, 18, 255]);
        self.fill_rect(x + w - 5 * s + eo, y + 6 * s, 2 * s, 2 * s, [24, 20, 18, 255]);
        self.fill_rect(x + s, y + h - 3 * s, 4 * s, 3 * s, [38, 34, 30, 255]);
        self.fill_rect(x + w - 5 * s, y + h - 3 * s, 4 * s, 3 * s, [38, 34, 30, 255]);
    }

    pub fn draw_selection(&mut self, tx: i32, ty: i32, cam_x: f32, cam_y: f32, zoom: f32, mining: f32) {
        let s = zoom as i32;
        let x = ((tx as f32 * 16.0 - cam_x) * zoom) as i32;
        let y = ((ty as f32 * 16.0 - cam_y) * zoom) as i32;
        let size = 16 * s;
        let white = [255, 255, 255, 200];
        for i in 0..size {
            self.put(x + i, y, white);
            self.put(x + i, y + size, white);
            self.put(x, y + i, white);
            self.put(x + size, y + i, white);
        }
        if mining > 0.0 {
            let bw = (size as f32 * mining.clamp(0.0, 1.0)) as i32;
            self.fill_rect(x, y + size - 3 * s, bw, 3 * s, [120, 220, 140, 255]);
        }
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.fb
    }
}
