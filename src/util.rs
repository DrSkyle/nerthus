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
    Sand,
    Sandstone,
    Snow,
    Ice,
    Water,
    Cactus,
    Mushroom,
    Herb,
    Ash,
    Hellstone,
    GrowingPlant,
}

impl Tile {
    pub fn solid(self) -> bool {
        matches!(
            self,
            Tile::Dirt
                | Tile::Grass
                | Tile::Stone
                | Tile::Wood
                | Tile::Ore
                | Tile::Plank
                | Tile::Sand
                | Tile::Sandstone
                | Tile::Snow
                | Tile::Ice
                | Tile::Cactus
                | Tile::Ash
                | Tile::Hellstone
        )
    }

    pub fn breakable(self) -> bool {
        !matches!(self, Tile::Air | Tile::Water)
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
            9 => Tile::Sand,
            10 => Tile::Sandstone,
            11 => Tile::Snow,
            12 => Tile::Ice,
            13 => Tile::Water,
            14 => Tile::Cactus,
            15 => Tile::Mushroom,
            16 => Tile::Herb,
            17 => Tile::Ash,
            18 => Tile::Hellstone,
            19 => Tile::GrowingPlant,
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
            Tile::Sand => 9,
            Tile::Sandstone => 10,
            Tile::Snow => 11,
            Tile::Ice => 12,
            Tile::Water => 13,
            Tile::Cactus => 14,
            Tile::Mushroom => 15,
            Tile::Herb => 16,
            Tile::Ash => 17,
            Tile::Hellstone => 18,
            Tile::GrowingPlant => 19,
        }
    }

    /// Index into the texture atlas.
    pub fn index(self) -> Option<usize> {
        match self {
            Tile::Dirt => Some(0),
            Tile::Grass => Some(1),
            Tile::Stone => Some(2),
            Tile::Wood => Some(3),
            Tile::Leaves => Some(4),
            Tile::Ore => Some(5),
            Tile::Plank => Some(6),
            Tile::Sand => Some(7),
            Tile::Sandstone => Some(8),
            Tile::Snow => Some(9),
            Tile::Ice => Some(10),
            Tile::Cactus => Some(11),
            Tile::Mushroom => Some(12),
            Tile::Herb => Some(13),
            Tile::Ash => Some(14),
            Tile::Hellstone => Some(15),
            Tile::GrowingPlant => Some(16),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Tile::Air => "air",
            Tile::Dirt => "dirt",
            Tile::Grass => "grass",
            Tile::Stone => "stone",
            Tile::Wood => "wood",
            Tile::Leaves => "leaves",
            Tile::Ore => "ore",
            Tile::Torch => "torch",
            Tile::Plank => "plank",
            Tile::Sand => "sand",
            Tile::Sandstone => "sandstone",
            Tile::Snow => "snow",
            Tile::Ice => "ice",
            Tile::Water => "water",
            Tile::Cactus => "cactus",
            Tile::Mushroom => "mushroom",
            Tile::Herb => "herb",
            Tile::Ash => "ash",
            Tile::Hellstone => "hellstone",
            Tile::GrowingPlant => "plant",
        }
    }

    /// Seconds to mine, before tool modifiers.
    pub fn hardness(self) -> f32 {
        match self {
            Tile::Grass | Tile::Dirt | Tile::Sand | Tile::Snow => 0.30,
            Tile::Leaves | Tile::Herb | Tile::GrowingPlant | Tile::Mushroom => 0.12,
            Tile::Wood | Tile::Plank | Tile::Cactus => 0.55,
            Tile::Stone | Tile::Sandstone | Tile::Ice => 1.10,
            Tile::Ore => 1.55,
            Tile::Ash => 0.45,
            Tile::Hellstone => 2.20,
            _ => 0.35,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Biome {
    Forest,
    Desert,
    Snow,
    Jungle,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Forest => "forest",
            Biome::Desert => "desert",
            Biome::Snow => "tundra",
            Biome::Jungle => "jungle",
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

pub fn hash1(x: i32, seed: i32) -> f32 {
    hash2(x, 0x5f3a, seed)
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

impl Default for Rng {
    fn default() -> Self {
        Rng { state: 1 }
    }
}

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng { state: seed | 1 }
    }

    pub fn next(&mut self) -> f32 {
        self.state = self.state.wrapping_add(0x6d2b79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f32) / 4294967296.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next() * (hi - lo)
    }



}

#[derive(Clone, Copy, Default)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

