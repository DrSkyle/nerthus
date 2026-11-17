pub const TILE: i32 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Air,
    Dirt,
    Grass,
    Stone,
    Wood,
    Leaves,
    Ore,
    Torch,
    Plank,
}

impl Tile {
    pub fn solid(self) -> bool {
        matches!(
            self,
            Tile::Dirt | Tile::Grass | Tile::Stone | Tile::Wood | Tile::Ore | Tile::Plank
        )
    }

    pub fn from_u8(v: u8) -> Tile {
        match v {
            1 => Tile::Dirt,
            2 => Tile::Grass,
            3 => Tile::Stone,
            4 => Tile::Wood,
            5 => Tile::Leaves,
            6 => Tile::Ore,
            7 => Tile::Torch,
            8 => Tile::Plank,
            _ => Tile::Air,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Tile::Air => 0,
            Tile::Dirt => 1,
            Tile::Grass => 2,
            Tile::Stone => 3,
            Tile::Wood => 4,
            Tile::Leaves => 5,
            Tile::Ore => 6,
            Tile::Torch => 7,
            Tile::Plank => 8,
        }
    }

    pub fn index(self) -> Option<usize> {
        match self {
            Tile::Dirt => Some(0),
            Tile::Grass => Some(1),
            Tile::Stone => Some(2),
            Tile::Wood => Some(3),
            Tile::Leaves => Some(4),
            Tile::Ore => Some(5),
            Tile::Plank => Some(6),
            _ => None,
        }
    }
}

pub fn clampf(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

pub fn hash2(x: i32, y: i32, seed: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (seed as u32).wrapping_mul(1442695041);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h as f32) / 4294967296.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

pub fn value_noise(x: f32, y: f32, seed: i32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let xf = smooth(x - xi);
    let yf = smooth(y - yi);
    let (xi, yi) = (xi as i32, yi as i32);
    let a = hash2(xi, yi, seed);
    let b = hash2(xi + 1, yi, seed);
    let c = hash2(xi, yi + 1, seed);
    let d = hash2(xi + 1, yi + 1, seed);
    let ab = a + (b - a) * xf;
    let cd = c + (d - c) * xf;
    ab + (cd - ab) * yf
}

pub fn fbm(x: f32, y: f32, octaves: u32, seed: i32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5f32;
    let mut norm = 0.0f32;
    let mut fx = x;
    let mut fy = y;
    for i in 0..octaves {
        sum += value_noise(fx, fy, seed.wrapping_add((i as i32) * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        fx *= 2.03;
        fy *= 2.01;
    }
    if norm > 0.0 {
        sum / norm
    } else {
        0.0
    }
}

pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng { state: seed }
    }

    pub fn next(&mut self) -> f32 {
        self.state = self.state.wrapping_add(0x6d2b79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f32) / 4294967296.0
    }
}

#[derive(Clone, Copy)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}
