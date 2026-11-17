use crate::atlas::{draw_tile, Neighbours, VARIANT_COUNT};
use crate::util::{hash2, Tile, TILE};
use crate::world::World;

pub struct Renderer {
    pub width: i32,
    pub height: i32,
    fb: Vec<u8>,
    atlas: Vec<[u8; 4]>,
}

const VARIANTS: usize = VARIANT_COUNT;
const ATLAS_TILES: usize = 17;

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
            Tile::Sand,
            Tile::Sandstone,
            Tile::Snow,
            Tile::Ice,
            Tile::Cactus,
            Tile::Mushroom,
            Tile::Herb,
            Tile::Ash,
            Tile::Hellstone,
            Tile::GrowingPlant,
            Tile::Water,
        ];
        let tw = TILE as usize;
        let aw = ATLAS_TILES * VARIANTS * tw;
        self.atlas = vec![[0, 0, 0, 0]; aw * tw];

        for (ti, tile) in tiles.iter().enumerate() {
            for v in 0..VARIANTS {
                let ox = (ti * VARIANTS + v) * tw;
                // 0..VARIANTS-16 are plain fills, the rest are edge variants
                let edge = if v >= crate::atlas::VARIANTS {
                    let m = v - crate::atlas::VARIANTS;
                    Some(Neighbours {
                        up: m & 1 != 0,
                        right: m & 2 != 0,
                        down: m & 4 != 0,
                        left: m & 8 != 0,
                    })
                } else {
                    None
                };
                draw_tile(&mut self.atlas, aw, ox, *tile, v % crate::atlas::VARIANTS, edge);
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

    pub fn draw_sky(
        &mut self,
        world: &World,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
        day: f32,
        sun_x: f32,
    ) {
        use crate::util::Biome;

        // Day/night blend: 1.0 = noon, 0.0 = midnight.
        let night = [12u8, 14, 32];
        let dawn = [58u8, 44, 88];
        let noon = [96u8, 132, 196];
        let dusk = [34u8, 40, 74];

        let (top, bottom) = if day > 0.5 {
            let t = (day - 0.5) * 2.0;
            (
                Self::mix3(dawn, noon, t),
                Self::mix3(dusk, [168u8, 196, 226], t),
            )
        } else {
            let t = day * 2.0;
            (
                Self::mix3(night, dawn, t),
                Self::mix3([24u8, 26, 44], dusk, t),
            )
        };

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
            let limit = horizon.clamp(0.0, self.height as f32) as i32;

            // Biome tint on the lower sky band.
            let bwx = ((x as f32 / zoom + cam_x) / 16.0) as i32;
            let tint = match world.biome_at(bwx) {
                Biome::Desert => [240u8, 214, 160],
                Biome::Snow => [226u8, 238, 252],
                Biome::Jungle => [176u8, 214, 154],
                Biome::Forest => [255u8, 255, 255],
            };

            for y in 0..limit {
                let f = (y as f32 / self.height as f32).clamp(0.0, 1.0);
                let near = (f * f * 0.45).min(0.45);
                let r = top[0] as f32 + (bottom[0] as f32 - top[0] as f32) * f;
                let g = top[1] as f32 + (bottom[1] as f32 - top[1] as f32) * f;
                let b = top[2] as f32 + (bottom[2] as f32 - top[2] as f32) * f;
                self.put(
                    x,
                    y,
                    [
                        (r + (tint[0] as f32 - r) * near).clamp(0.0, 255.0) as u8,
                        (g + (tint[1] as f32 - g) * near).clamp(0.0, 255.0) as u8,
                        (b + (tint[2] as f32 - b) * near).clamp(0.0, 255.0) as u8,
                        255,
                    ],
                );
            }
            for y in limit..self.height {
                self.put(x, y, [14, 14, 22, 255]);
            }
        }

        // Sun / moon disc in screen space, only while the camera can see sky.
        let cam_surface = world.column_height((cam_x / 16.0) as i32) as f32 * 16.0;
        let sky_band = (cam_surface - cam_y) * zoom;
        if sky_band < self.height as f32 * 0.15 {
            // fully underground: no disc
        } else {
        let hx = sun_x * self.width as f32;
        let hy = (self.height as f32 * 0.18).min(sky_band * 0.45);
        let is_day = day > 0.35;
        let disc = if is_day { [255u8, 244, 186] } else { [206u8, 214, 240] };
        let rad = self.height as f32 * if is_day { 0.045 } else { 0.032 };
        let cx = if is_day { hx } else { (1.0 - sun_x) * self.width as f32 };
        for sy in -rad as i32..=rad as i32 {
            for sx in -rad as i32..=rad as i32 {
                let d = ((sx * sx + sy * sy) as f32).sqrt();
                if d > rad {
                    continue;
                }
                let a = (1.0 - d / rad).clamp(0.0, 1.0);
                let glow = (a * a * 0.85 + 0.15).min(1.0);
                let x = cx as i32 + sx;
                let y = hy as i32 + sy;
                if x < 0 || y < 0 || x >= self.width || y >= self.height {
                    continue;
                }
                let i = ((y * self.width + x) * 4) as usize;
                self.fb[i] = (self.fb[i] as f32 * (1.0 - glow) + disc[0] as f32 * glow) as u8;
                self.fb[i + 1] =
                    (self.fb[i + 1] as f32 * (1.0 - glow) + disc[1] as f32 * glow) as u8;
                self.fb[i + 2] =
                    (self.fb[i + 2] as f32 * (1.0 - glow) + disc[2] as f32 * glow) as u8;
            }
        }

        } // end sky-visible disc block

        // Stars fade in at night.
        let star_alpha = (1.0 - day * 1.6).clamp(0.0, 1.0);
        if star_alpha > 0.02 {
            for i in 0..220 {
                let sx = (hash2(i, 3, 11) * self.width as f32) as i32;
                let sy = (hash2(i, 7, 11) * self.height as f32) as i32;
                let wx = (sx as f32 / zoom + cam_x) / 16.0;
                let surface_px = world.column_height(wx as i32) as f32 * 16.0;
                if (sy as f32) > (surface_px - cam_y) * zoom {
                    continue;
                }
                let tw = 0.55 + 0.45 * ((i as f32 * 0.7).sin().abs());
                let a = (hash2(i, 13, 5) * 120.0 + 90.0) * star_alpha * tw;
                self.put(sx, sy, [a as u8, a as u8, a as u8, 255]);
            }
        }
    }

    fn mix3(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        [
            (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
            (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
            (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
        ]
    }

    pub fn draw_world(
        &mut self,
        world: &World,
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
        time: f32,
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

                if tile == Tile::Water {
                    let s = zoom as i32;
                    let wob = ((time * 2.4 + tx as f32 * 0.6 + ty as f32 * 0.35).sin() * 0.5 + 0.5)
                        * 18.0;
                    let top_a = if world.get(tx, ty - 1) == Tile::Water { 0.62 } else { 0.46 };
                    let c = [
                        (34.0 + wob * 0.35) as u8,
                        (96.0 + wob) as u8,
                        (168.0 + wob * 1.4).min(255.0) as u8,
                        255,
                    ];
                    let _ = top_a;
                    // body of water, slightly translucent-looking
                    for yy in 0..16 {
                        for xx in 0..16 {
                            let x = sx + xx * s;
                            let y = sy + yy * s;
                            if x < 0 || y < 0 || x >= self.width || y >= self.height {
                                continue;
                            }
                            let i = ((y * self.width + x) * 4) as usize;
                            let a = 0.72f32;
                            self.fb[i] = (self.fb[i] as f32 * (1.0 - a) + c[0] as f32 * a) as u8;
                            self.fb[i + 1] =
                                (self.fb[i + 1] as f32 * (1.0 - a) + c[1] as f32 * a) as u8;
                            self.fb[i + 2] =
                                (self.fb[i + 2] as f32 * (1.0 - a) + c[2] as f32 * a) as u8;
                        }
                    }
                    if world.get(tx, ty - 1) != Tile::Water {
                        let s = zoom as i32;
                        self.fill_rect(sx, sy, 16 * s, s.max(1) * 2, [198, 232, 255, 255]);
                    }
                    continue;
                }

                let variant = self.pick_variant(world, tx, ty, tile);
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
        const AMB_R: f32 = 0.085;
        const AMB_G: f32 = 0.090;
        const AMB_B: f32 = 0.115;

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

        // Precompute the interpolated horizon once per column: the per-pixel
        // closure was costing ~3ms/frame.
        let mut horizons = vec![0.0f32; self.width as usize];
        for sx in 0..self.width {
            let wx = (sx as f32 * inv_zoom + cam_x) / 16.0;
            let x0 = wx.floor();
            let fx = wx - x0;
            let h0 = world.column_height(x0 as i32) as f32 * 16.0;
            let h1 = world.column_height(x0 as i32 + 1) as f32 * 16.0;
            horizons[sx as usize] = (h0 + (h1 - h0) * fx - cam_y) * zoom;
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

                // Sky above the terrain line must keep its own colour; only the
                // ground and underground get multiplied by the light field.
                if (sy as f32) < horizons[sx as usize] - 0.5 {
                    continue;
                }

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

    /// Edge-sensitive tiles get a variant chosen from which neighbours are
    /// solid; everything else just picks a deterministic noise variant.
    fn pick_variant(&self, world: &World, tx: i32, ty: i32, tile: Tile) -> usize {
        let edge_sensitive = matches!(
            tile,
            Tile::Grass | Tile::Dirt | Tile::Stone | Tile::Sand | Tile::Snow | Tile::Ash
        );
        let plain = (hash2(tx, ty, 4242) * crate::atlas::VARIANTS as f32) as usize
            % crate::atlas::VARIANTS;
        if !edge_sensitive {
            return plain;
        }
        let n = Neighbours {
            up: world.get(tx, ty - 1).solid(),
            down: world.get(tx, ty + 1).solid(),
            left: world.get(tx - 1, ty).solid(),
            right: world.get(tx + 1, ty).solid(),
        };
        let m = n.mask();
        // Only use an edge variant when it actually differs from plain fill,
        // otherwise prefer the textured plain tile.
        if m == 0 {
            return plain;
        }
        crate::atlas::VARIANTS + m
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

    pub fn draw_enemies(
        &mut self,
        enemies: &[crate::entities::Enemy],
        cam_x: f32,
        cam_y: f32,
        zoom: f32,
    ) {
        use crate::entities::EnemyKind;
        let s = (zoom as i32).max(1);
        for e in enemies {
            let x = ((e.x - cam_x) * zoom) as i32;
            let y = ((e.y - cam_y) * zoom) as i32;
            let w = (e.w() * zoom) as i32;
            let h = (e.h() * zoom) as i32;
            if x + w < 0 || y + h < 0 || x >= self.width || y >= self.height {
                continue;
            }
            let flash = e.hurt > 0.0;
            let (body, dark) = match e.kind {
                EnemyKind::Slime => ([76u8, 176, 96], [46u8, 118, 66]),
                EnemyKind::Bat => ([94u8, 74, 122], [58u8, 44, 78]),
                EnemyKind::Skeleton => ([206u8, 206, 190], [150u8, 150, 138]),
            };
            let (body, dark) = if flash {
                ([255u8, 120, 120], [200u8, 80, 80])
            } else {
                (body, dark)
            };

            match e.kind {
                EnemyKind::Slime => {
                    // rounded blob with a highlight
                    self.fill_rect(x, y + h / 3, w, h - h / 3, [body[0], body[1], body[2], 255]);
                    self.fill_rect(x + s, y + h / 6, w - 2 * s, h / 3, [body[0], body[1], body[2], 255]);
                    self.fill_rect(x + w / 3, y + h / 2, w / 4, h / 6, [230, 250, 230, 255]);
                    self.fill_rect(x + w / 4, y + h - h / 4, s * 2, s, [dark[0], dark[1], dark[2], 255]);
                    self.fill_rect(x + w - w / 3, y + h - h / 4, s * 2, s, [dark[0], dark[1], dark[2], 255]);
                    // eyes
                    self.fill_rect(x + w / 3, y + h / 2, s, s, [20, 20, 24, 255]);
                    self.fill_rect(x + w - w / 3 - s, y + h / 2, s, s, [20, 20, 24, 255]);
                }
                EnemyKind::Bat => {
                    let flap = ((e.bob * 9.0).sin() * 0.5 + 0.5) * (h as f32 * 0.8);
                    let wing = flap.max(1.0) as i32;
                    self.fill_rect(x + w / 3, y + h / 3, w / 3, h / 2, [body[0], body[1], body[2], 255]);
                    self.fill_rect(x, y + h / 3 - wing / 3, w / 3, wing, [dark[0], dark[1], dark[2], 255]);
                    self.fill_rect(x + w - w / 3, y + h / 3 - wing / 3, w / 3, wing, [dark[0], dark[1], dark[2], 255]);
                    self.fill_rect(x + w / 2 - s, y + h / 2, s * 2, s * 2, [220, 60, 60, 255]);
                }
                EnemyKind::Skeleton => {
                    // skull + ribcage
                    self.fill_rect(x + s, y, w - 2 * s, h / 3, [body[0], body[1], body[2], 255]);
                    self.fill_rect(x + 2 * s, y + h / 3, w - 4 * s, h - h / 3, [dark[0], dark[1], dark[2], 255]);
                    for i in 1..4 {
                        let ry = y + h / 3 + i * (h / 5);
                        self.fill_rect(x + s, ry, w - 2 * s, (s).max(1), [body[0], body[1], body[2], 255]);
                    }
                    self.fill_rect(x + 2 * s, y + h / 8, s, s, [20, 20, 24, 255]);
                    self.fill_rect(x + w - 3 * s, y + h / 8, s, s, [20, 20, 24, 255]);
                }
            }

            // health bar when damaged
            if e.hp < e.kind.hp() {
                let frac = (e.hp as f32 / e.kind.hp() as f32).clamp(0.0, 1.0);
                self.fill_rect(x, y - 3 * s.max(1), w, s.max(1), [20, 20, 24, 255]);
                self.fill_rect(
                    x,
                    y - 3 * s.max(1),
                    (w as f32 * frac) as i32,
                    s.max(1),
                    [220, 70, 70, 255],
                );
            }
        }
    }

    pub fn draw_particles(&mut self, particles: &[crate::entities::Particle], cam_x: f32, cam_y: f32, zoom: f32) {
        let s = (zoom as i32).max(1);
        for p in particles {
            let x = ((p.x - cam_x) * zoom) as i32;
            let y = ((p.y - cam_y) * zoom) as i32;
            let a = (p.life / p.max_life).clamp(0.0, 1.0);
            let size = (p.size * zoom * 0.5).max(1.0) as i32;
            let c = [
                (p.color[0] as f32 * a) as u8,
                (p.color[1] as f32 * a) as u8,
                (p.color[2] as f32 * a) as u8,
                255,
            ];
            self.fill_rect(x, y, size, size, c);
            let _ = s;
        }
    }

    pub fn draw_swing(&mut self, player: &crate::player::Player, cam_x: f32, cam_y: f32, zoom: f32, swing: f32) {
        if swing <= 0.0 {
            return;
        }
        let s = (zoom as i32).max(1);
        let t = (1.0 - swing / 0.18).clamp(0.0, 1.0);
        let dir = player.facing as f32;
        let base_x = ((player.cx() - cam_x) * zoom) as i32;
        let base_y = ((player.cy() - cam_y) * zoom) as i32;
        let arc = 26.0 * s as f32;
        for i in 0..8 {
            let f = i as f32 / 7.0;
            let ang = (t * 2.4 - 0.9 + f * 0.5) * dir;
            let x = base_x + (ang * arc) as i32;
            let y = base_y - ((f * 14.0 - 7.0) * s as f32) as i32;
            let a = (255.0 * (1.0 - f * 0.6)) as u8;
            self.fill_rect(x, y, (2 * s) as i32, s, [a, a, (a as f32 * 0.75) as u8, 255]);
        }
    }

    pub fn draw_hud(&mut self, hp: f32, max_hp: f32, kills: i32, inv: &crate::Inventory, clock: f32, biome: &str, fps: f32) {
        let pad = 5.max(self.width / 200);
        let bar_w = (self.width as f32 * 0.16).min(160.0) as i32;
        let bar_h = (pad + 2).max(5);
        let x = pad;
        let y = pad;

        // hearts / health bar
        self.fill_rect(x - 2, y - 2, bar_w + 4, bar_h + 4, [12, 14, 20, 255]);
        self.fill_rect(x, y, bar_w, bar_h, [58, 24, 26, 255]);
        let frac = (hp / max_hp).clamp(0.0, 1.0);
        let fill = (bar_w as f32 * frac) as i32;
        self.fill_rect(x, y, fill, bar_h, [206, 62, 54, 255]);
        self.fill_rect(x, y, fill, bar_h / 3, [238, 104, 90, 255]);
        // hp pips
        for i in 0..10 {
            let px = x + i * (bar_w / 10);
            self.fill_rect(px, y + bar_h + 2, 2, 2, [90, 96, 108, 255]);
        }

        // clock dial
        let cx = x + bar_w + 3 * pad;
        let cy = y + bar_h / 2;
        let r = (bar_h as f32 * 1.6) as i32;
        let is_day = clock > 0.25 && clock < 0.75;
        let col = if is_day { [240u8, 214, 120] } else { [150u8, 170, 220] };
        for sy in -r..=r {
            for sx in -r..=r {
                let d2 = sx * sx + sy * sy;
                if d2 > r * r {
                    continue;
                }
                if d2 > (r - 2) * (r - 2) {
                    self.put(cx + sx, cy + sy, [40, 46, 58, 255]);
                } else {
                    self.put(cx + sx, cy + sy, [col[0], col[1], col[2], 255]);
                }
            }
        }
        let ang = (clock * std::f32::consts::TAU) - std::f32::consts::FRAC_PI_2;
        for t in 0..r {
            let px = cx + (ang.cos() * t as f32) as i32;
            let py = cy + (ang.sin() * t as f32) as i32;
            self.put(px, py, [30, 32, 40, 255]);
        }

        // item tally as little colored chips
        let chips = [
            (inv.dirt, [116u8, 78, 46]),
            (inv.stone, [110u8, 110, 120]),
            (inv.wood, [122u8, 88, 50]),
            (inv.sand, [206u8, 186, 122]),
            (inv.snow, [232u8, 240, 250]),
            (inv.ore, [212u8, 186, 86]),
            (inv.gems, [140u8, 200, 240]),
        ];
        let mut px = x;
        let py = y + bar_h + 2 * pad;
        for (n, c) in chips {
            if n <= 0 {
                continue;
            }
            self.fill_rect(px, py, 3 * pad, 3 * pad, [c[0], c[1], c[2], 255]);
            self.fill_rect(px, py, 3 * pad, pad, [255, 255, 255, 60]);
            let _ = n;
            px += 4 * pad;
        }

        let _ = (kills, biome, fps);
    }

    /// Crude 5x7 bitmap text, centered. Used for the death screen.
    pub fn draw_center_text(&mut self, text: &str, color: [u8; 3], scale: i32) {
        let scale = scale.max(1);
        let glyph_w = 6 * scale;
        let total = text.len() as i32 * glyph_w;
        let x0 = (self.width - total) / 2;
        let y0 = self.height / 2 - 3 * scale;
        for (i, ch) in text.chars().enumerate() {
            let bits = glyph(ch);
            let gx = x0 + i as i32 * glyph_w;
            for (row, line) in bits.iter().enumerate() {
                for col in 0..5 {
                    if line & (1 << (4 - col)) == 0 {
                        continue;
                    }
                    self.fill_rect(
                        gx + col as i32 * scale,
                        y0 + row as i32 * scale,
                        scale,
                        scale,
                        [color[0], color[1], color[2], 255],
                    );
                }
            }
        }
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.fb
    }
}

fn glyph(c: char) -> [u8; 7] {
    match c {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}
