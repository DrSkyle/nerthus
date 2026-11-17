use crate::util::{fbm, Rng, Tile};

pub struct World {
    pub w: i32,
    pub h: i32,
    pub seed: u32,
    pub tiles: Vec<u8>,
    pub surface: Vec<i32>,
}

impl World {
    pub fn new(w: i32, h: i32, seed: u32) -> World {
        let mut world = World {
            w,
            h,
            seed,
            tiles: vec![0u8; (w * h) as usize],
            surface: vec![0i32; w as usize],
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

    fn generate(&mut self) {
        let w = self.w;
        let h = self.h;
        let seed = self.seed as i32;
        let base = (h as f32 * 0.34) as i32;

        for x in 0..w {
            let hill = (fbm(x as f32 * 0.006, 0.5, 4, seed) - 0.5) * h as f32 * 0.26;
            let detail = (fbm(x as f32 * 0.045, 9.1, 2, seed.wrapping_add(17)) - 0.5) * 5.0;
            let surface = (base as f32 + hill + detail).round() as i32;
            self.surface[x as usize] = surface;

            for y in 0..h {
                let i = self.idx(x, y);
                if y < surface {
                    self.tiles[i] = Tile::Air.as_u8();
                    continue;
                }
                let depth = y - surface;
                if depth == 0 {
                    self.tiles[i] = Tile::Grass.as_u8();
                } else {
                    let dirt_depth = 6.0
                        + fbm(
                            x as f32 * 0.08,
                            depth as f32 * 0.08,
                            2,
                            seed.wrapping_add(53),
                        ) * 10.0;
                    self.tiles[i] = if (depth as f32) < dirt_depth {
                        Tile::Dirt.as_u8()
                    } else {
                        Tile::Stone.as_u8()
                    };
                }
            }
        }

        for x in 0..w {
            let surface = self.surface[x as usize];
            for y in (surface + 3)..(h - 1) {
                let blob = fbm(x as f32 * 0.02, y as f32 * 0.028, 3, seed.wrapping_add(77));
                let tunnel = (fbm(x as f32 * 0.012, y as f32 * 0.016, 3, seed.wrapping_add(31))
                    - 0.5)
                    .abs();
                let i = self.idx(x, y);
                if self.tiles[i] == Tile::Stone.as_u8() || self.tiles[i] == Tile::Dirt.as_u8() {
                    if blob > 0.66 || tunnel < 0.022 {
                        self.tiles[i] = Tile::Air.as_u8();
                    }
                }
            }
        }

        for x in 0..w {
            let surface = self.surface[x as usize];
            for y in (surface + 8)..(h - 2) {
                let i = self.idx(x, y);
                if self.tiles[i] != Tile::Stone.as_u8() {
                    continue;
                }
                if fbm(x as f32 * 0.07, y as f32 * 0.07, 2, seed.wrapping_add(5)) > 0.665 {
                    self.tiles[i] = Tile::Ore.as_u8();
                }
            }
        }

        let mut rng = Rng::new(self.seed.wrapping_add(991));
        let mut last_tree = -99i32;
        for x in 4..(w - 4) {
            if x - last_tree < 4 {
                continue;
            }
            let surface = self.surface[x as usize];
            if self.get(x, surface) != Tile::Grass {
                continue;
            }
            if rng.next() > 0.18 {
                continue;
            }
            let trunk = 4 + (rng.next() * 5.0) as i32;
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

    pub fn tile_counts(&self) -> [usize; 9] {
        let mut counts = [0usize; 9];
        for &t in &self.tiles {
            counts[t as usize] += 1;
        }
        counts
    }
}
