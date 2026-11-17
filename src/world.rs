use crate::util::{fbm, hash1, Biome, Rng, Tile};

pub struct World {
    pub w: i32,
    pub h: i32,
    pub seed: u32,
    pub tiles: Vec<u8>,
    pub surface: Vec<i32>,
    pub biome: Vec<Biome>,
    pub water_level: i32,
}

impl World {
    pub fn new(w: i32, h: i32, seed: u32) -> World {
        let mut world = World {
            w,
            h,
            seed,
            tiles: vec![0u8; (w * h) as usize],
            surface: vec![0i32; w as usize],
            biome: vec![Biome::Forest; w as usize],
            water_level: (h as f32 * 0.52) as i32,
        };
        world.generate();
        world
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.w || y >= self.h {
            return Tile::Stone;
        }
        if y < 0 {
            return Tile::Air;
        }
        Tile::from_u8(self.tiles[self.idx(x, y)])
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, v: Tile) {
        if x < 0 || x >= self.w || y < 0 || y >= self.h {
            return;
        }
        let i = self.idx(x, y);
        self.tiles[i] = v.as_u8();
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        self.get(x, y).solid()
    }

    #[inline]
    pub fn is_solid_at(&self, px: f32, py: f32) -> bool {
        self.is_solid((px / 16.0).floor() as i32, (py / 16.0).floor() as i32)
    }

    pub fn biome_at(&self, x: i32) -> Biome {
        self.biome[x.max(0).min(self.w - 1) as usize]
    }

    /// Depth below the surface, in tiles.
    fn generate(&mut self) {
        let w = self.w;
        let h = self.h;
        let seed = self.seed as i32;
        let base = (h as f32 * 0.30) as i32;

        // ---- biome bands -------------------------------------------------
        // Pick biomes by sampling one slow noise field, then cutting it into
        // quartiles. Quantile thresholds (not fixed constants) keep every seed
        // producing a mix of all four biomes instead of one dominant band.
        let mut profile = vec![0f32; w as usize];
        for x in 0..w {
            profile[x as usize] = fbm(x as f32 * 0.004, 41.7, 2, seed.wrapping_add(900));
        }
        let mut sorted = profile.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let q = |f: f32| sorted[((sorted.len() - 1) as f32 * f) as usize];
        let (q1, q2, q3) = (q(0.22), q(0.42), q(0.60));
        for x in 0..w {
            let t = profile[x as usize];
            self.biome[x as usize] = if t <= q1 {
                Biome::Desert
            } else if t <= q2 {
                Biome::Snow
            } else if t <= q3 {
                Biome::Jungle
            } else {
                Biome::Forest
            };
        }

        // ---- surface height ---------------------------------------------
        for x in 0..w {
            let biome = self.biome[x as usize];
            let amp = match biome {
                Biome::Forest => 0.24,
                Biome::Jungle => 0.20,
                Biome::Desert => 0.14,
                Biome::Snow => 0.30,
            };
            let hill = (fbm(x as f32 * 0.006, 0.5, 4, seed) - 0.5) * h as f32 * amp;
            let detail = (fbm(x as f32 * 0.045, 9.1, 2, seed.wrapping_add(17)) - 0.5) * 5.0;
            self.surface[x as usize] = (base as f32 + hill + detail).round() as i32;
        }

        // ---- terrain body ------------------------------------------------
        for x in 0..w {
            let surface = self.surface[x as usize];
            let biome = self.biome[x as usize];
            let lower = (h as f32 * 0.74) as i32;
            for y in 0..h {
                let i = self.idx(x, y);
                if y < surface {
                    self.tiles[i] = Tile::Air.as_u8();
                    continue;
                }
                let depth = (y - surface) as f32;
                let dirt_depth =
                    6.0 + fbm(x as f32 * 0.08, depth as f32 * 0.08, 2, seed.wrapping_add(53)) * 10.0;

                let tile = match biome {
                    Biome::Desert => {
                        if depth < 3.0 {
                            Tile::Sand
                        } else if depth < dirt_depth + 6.0 {
                            Tile::Sandstone
                        } else {
                            Tile::Stone
                        }
                    }
                    Biome::Snow => {
                        if depth < 1.0 {
                            Tile::Snow
                        } else if depth < dirt_depth {
                            Tile::Dirt
                        } else {
                            Tile::Stone
                        }
                    }
                    Biome::Jungle => {
                        if depth < 1.0 {
                            Tile::Grass
                        } else if depth < dirt_depth {
                            Tile::Dirt
                        } else {
                            Tile::Stone
                        }
                    }
                    Biome::Forest => {
                        if depth < 1.0 {
                            Tile::Grass
                        } else if depth < dirt_depth {
                            Tile::Dirt
                        } else {
                            Tile::Stone
                        }
                    }
                };

                // frozen water table => ice sheets in tundra
                if biome == Biome::Snow && depth < 3.0 && y > self.water_level - 6 {
                    self.tiles[i] = Tile::Ice.as_u8();
                    continue;
                }

                self.tiles[i] = tile.as_u8();
                if y >= lower && fbm(x as f32 * 0.05, y as f32 * 0.06, 2, seed + 771) > 0.52 {
                    self.tiles[i] = Tile::Ash.as_u8();
                }
            }
        }

        // ---- caves -------------------------------------------------------
        for x in 0..w {
            let surface = self.surface[x as usize];
            for y in (surface + 3)..(h - 1) {
                let i = self.idx(x, y);
                let cur = self.tiles[i];
                let is_rock = cur == Tile::Stone.as_u8()
                    || cur == Tile::Dirt.as_u8()
                    || cur == Tile::Sandstone.as_u8()
                    || cur == Tile::Ash.as_u8();
                if !is_rock {
                    continue;
                }
                let blob = fbm(x as f32 * 0.020, y as f32 * 0.028, 3, seed.wrapping_add(77));
                let tunnel = (fbm(x as f32 * 0.012, y as f32 * 0.016, 3, seed.wrapping_add(31))
                    - 0.5)
                    .abs();
                let cavern = (fbm(x as f32 * 0.007, y as f32 * 0.010, 4, seed + 1301) - 0.5).abs();
                if blob > 0.66 || tunnel < 0.022 || cavern < 0.012 {
                    self.tiles[i] = Tile::Air.as_u8();
                }
            }
        }

        // ---- water: only fill genuinely enclosed pools ----------------------
        // A pool is a run of air whose bottom rests on solid ground and whose
        // top has solid ground on at least one side, and which sits below sea
        // level. This avoids painting a stripe across every open cave.
        for x in 1..(w - 1) {
            let start = self.water_level.max(self.surface[x as usize] + 2);
            let mut y = start;
            while y < h - 1 {
                if self.tiles[self.idx(x, y)] != Tile::Air.as_u8() {
                    y += 1;
                    continue;
                }
                let run_start = y;
                while y < h - 1 && self.tiles[self.idx(x, y)] == Tile::Air.as_u8() {
                    y += 1;
                }
                let floor_solid = self.tiles[self.idx(x, y)] != Tile::Air.as_u8();
                // side walls: at least one neighbour column is solid at mid-pool
                let mid = (run_start + y) / 2;
                let walled = self.tiles[self.idx(x - 1, mid)] != Tile::Air.as_u8()
                    || self.tiles[self.idx(x + 1, mid)] != Tile::Air.as_u8();
                if floor_solid && walled && run_start > self.surface[x as usize] {
                    for wy in run_start..y {
                        let i = (wy * w + x) as usize;
                        self.tiles[i] = Tile::Water.as_u8();
                    }
                }
            }
        }

        // ---- ore by depth -------------------------------------------------
        for x in 0..w {
            let surface = self.surface[x as usize];
            for y in (surface + 4)..(h - 2) {
                let i = self.idx(x, y);
                if self.tiles[i] != Tile::Stone.as_u8() && self.tiles[i] != Tile::Sandstone.as_u8() {
                    continue;
                }
                let depth = (y - surface) as f32 / h as f32;
                let n = fbm(x as f32 * 0.07, y as f32 * 0.07, 2, seed.wrapping_add(5));
                let threshold = 0.70 - depth * 0.28;
                if n > threshold {
                    self.tiles[i] = Tile::Ore.as_u8();
                }
                if depth > 0.72
                    && fbm(x as f32 * 0.11, y as f32 * 0.11, 2, seed + 4409) > 0.60
                {
                    self.tiles[i] = Tile::Hellstone.as_u8();
                }
            }
        }

        // ---- vegetation ---------------------------------------------------
        let mut rng = Rng::new(self.seed.wrapping_add(991));
        let mut last_tree = -99i32;
        for x in 4..(w - 4) {
            if x - last_tree < 4 {
                continue;
            }
            let surface = self.surface[x as usize];
            let biome = self.biome[x as usize];
            match biome {
                Biome::Forest | Biome::Jungle => {
                    if self.get(x, surface) != Tile::Grass || rng.next() > 0.20 {
                        continue;
                    }
                    let trunk = if biome == Biome::Jungle {
                        7 + (rng.next() * 8.0) as i32
                    } else {
                        4 + (rng.next() * 5.0) as i32
                    };
                    let top_y = surface - trunk;
                    for i in 1..=trunk {
                        let ty = surface - i;
                        if ty > 0 {
                            self.set(x, ty, Tile::Wood);
                        }
                    }
                    let r = 2 + (rng.next() * 2.0) as i32;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if dx * dx + dy * dy > r * r + 1 {
                                continue;
                            }
                            let lx = x + dx;
                            let ly = top_y + dy;
                            if lx < 0 || lx >= w || ly <= 0 {
                                continue;
                            }
                            if self.get(lx, ly) == Tile::Air {
                                self.set(lx, ly, Tile::Leaves);
                            }
                        }
                    }
                    last_tree = x;
                }
                Biome::Desert => {
                    if self.get(x, surface - 1) != Tile::Air || rng.next() > 0.09 {
                        continue;
                    }
                    let tall = 2 + (rng.next() * 3.0) as i32;
                    for i in 1..=tall {
                        if surface - i > 0 {
                            self.set(x, surface - i, Tile::Cactus);
                        }
                    }
                    last_tree = x;
                }
                Biome::Snow => {}
            }
        }

        // ---- surface decoration ------------------------------------------
        for x in 2..(w - 2) {
            let surface = self.surface[x as usize];
            let biome = self.biome[x as usize];
            if biome == Biome::Desert || biome == Biome::Snow {
                continue;
            }
            let r = hash1(x, seed.wrapping_add(31));
            if r < 0.10 && self.get(x, surface - 1) == Tile::Air {
                self.set(x, surface - 1, Tile::Herb);
            } else if r > 0.955 && self.get(x, surface - 1) == Tile::Air {
                self.set(x, surface - 1, Tile::Mushroom);
            }
        }
    }

    pub fn spawn_point(&self) -> (f32, f32) {
        let x = self.w / 2;
        let mut y = 0;
        while y < self.h - 1 && !self.is_solid(x, y) {
            y += 1;
        }
        ((x as f32 + 0.5) * 16.0, ((y - 1) as f32) * 16.0)
    }

    pub fn column_height(&self, x: i32) -> i32 {
        let x = x.max(0).min(self.w - 1);
        self.surface[x as usize]
    }

    pub fn tile_counts(&self) -> [usize; 20] {
        let mut counts = [0usize; 20];
        for &t in &self.tiles {
            counts[t as usize] += 1;
        }
        counts
    }

    pub fn biome_counts(&self) -> [usize; 4] {
        let mut counts = [0usize; 4];
        for b in &self.biome {
            counts[*b as usize] += 1;
        }
        counts
    }

}

