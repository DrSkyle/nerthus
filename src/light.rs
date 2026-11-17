use crate::util::Rgb;
use crate::world::World;

const ATT_OPEN: f32 = 0.045;
const ATT_SOLID: f32 = 0.22;
const MAX_LEVELS: usize = 96;

#[derive(Clone, Copy)]
pub struct TorchLight {
    pub x: i32,
    pub y: i32,
    pub rgb: Rgb,
}

pub struct Lighting {
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
    buckets: Vec<Vec<(u32, f32, f32, f32)>>,
}

impl Lighting {
    pub fn new(world: &World) -> Lighting {
        let n = (world.w * world.h) as usize;
        Lighting {
            r: vec![0.0; n],
            g: vec![0.0; n],
            b: vec![0.0; n],
            buckets: (0..=MAX_LEVELS).map(|_| Vec::new()).collect(),
        }
    }

    fn clear_buckets(&mut self) {
        for bucket in self.buckets.iter_mut() {
            bucket.clear();
        }
    }

    #[inline]
    fn push(&mut self, i: u32, r: f32, g: f32, b: f32) {
        if r <= 0.004 && g <= 0.004 && b <= 0.004 {
            return;
        }
        let m = r.max(g).max(b);
        let mut k = (m * MAX_LEVELS as f32).round() as i32;
        if k > MAX_LEVELS as i32 {
            k = MAX_LEVELS as i32;
        }
        if k < 0 {
            k = 0;
        }
        self.buckets[k as usize].push((i, r, g, b));
    }

    #[inline]
    fn take_max(&mut self, i: usize, r: f32, g: f32, b: f32) {
        if r > self.r[i] {
            self.r[i] = r;
        }
        if g > self.g[i] {
            self.g[i] = g;
        }
        if b > self.b[i] {
            self.b[i] = b;
        }
    }

    pub fn compute(&mut self, world: &World, x0: i32, y0: i32, x1: i32, y1: i32, sky: f32, torches: &[TorchLight]) {
        let x0 = x0.max(0);
        let y0 = y0.max(0);
        let x1 = x1.min(world.w - 1);
        let y1 = y1.min(world.h - 1);
        if x1 < x0 || y1 < y0 {
            return;
        }
        let w = world.w;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = (y * w + x) as usize;
                self.r[i] = 0.0;
                self.g[i] = 0.0;
                self.b[i] = 0.0;
            }
        }
        self.clear_buckets();

        for x in x0..=x1 {
            let mut atten = 0.0f32;
            for y in 0..=y1 {
                let i = (y * w + x) as usize;
                if world.tiles[i] != 0 && crate::util::Tile::from_u8(world.tiles[i]).solid() {
                    atten = (atten + 0.16).min(1.0);
                }
                let s = sky * (1.0 - atten).max(0.0);
                if y >= y0 && s > 0.0 {
                    self.take_max(i, s * 0.56, s * 0.68, s);
                    if s > 0.02 {
                        self.push(i as u32, s * 0.56, s * 0.68, s);
                    }
                }
            }
        }

        for t in torches {
            if t.x < x0 || t.x > x1 || t.y < y0 || t.y > y1 {
                continue;
            }
            let i = (t.y * w + t.x) as usize;
            self.take_max(i, t.rgb.r, t.rgb.g, t.rgb.b);
            self.push(i as u32, t.rgb.r, t.rgb.g, t.rgb.b);
        }

        let mut level = MAX_LEVELS as i32;
        while level >= 0 {
            let bucket = self.buckets[level as usize].clone();
            for (i, r, g, b) in bucket {
                let i = i as usize;
                if r < self.r[i] - 1e-4 && g < self.g[i] - 1e-4 && b < self.b[i] - 1e-4 {
                    continue;
                }
                let x = (i as i32) % w;
                let y = (i as i32) / w;
                let neighbours = [
                    (x - 1, y, i.wrapping_sub(1) as i64),
                    (x + 1, y, (i + 1) as i64),
                    (x, y - 1, i.wrapping_sub(w as usize) as i64),
                    (x, y + 1, (i + w as usize) as i64),
                ];
                for (nx, ny, ni) in neighbours {
                    if nx < x0 || nx > x1 || ny < y0 || ny > y1 {
                        continue;
                    }
                    if ni < 0 {
                        continue;
                    }
                    let ni = ni as usize;
                    if ni >= world.tiles.len() {
                        continue;
                    }
                    let solid = world.tiles[ni] != 0 && crate::util::Tile::from_u8(world.tiles[ni]).solid();
                    let att = if solid { ATT_SOLID } else { ATT_OPEN };
                    let (nr, ng, nb) = (r - att, g - att, b - att);
                    if nr <= 0.004 && ng <= 0.004 && nb <= 0.004 {
                        continue;
                    }
                    if nr <= self.r[ni] + 1e-4 && ng <= self.g[ni] + 1e-4 && nb <= self.b[ni] + 1e-4 {
                        continue;
                    }
                    self.take_max(ni, nr, ng, nb);
                    self.push(ni as u32, nr, ng, nb);
                }
            }
            self.buckets[level as usize].clear();
            level -= 1;
        }
    }

}
