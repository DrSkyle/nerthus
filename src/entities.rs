use crate::util::{Rng, Tile};
use crate::world::World;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    Slime,
    Bat,
    Skeleton,
}

impl EnemyKind {
    pub fn hp(self) -> i32 {
        match self {
            EnemyKind::Slime => 22,
            EnemyKind::Bat => 14,
            EnemyKind::Skeleton => 34,
        }
    }

    pub fn damage(self) -> i32 {
        match self {
            EnemyKind::Slime => 6,
            EnemyKind::Bat => 4,
            EnemyKind::Skeleton => 9,
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            EnemyKind::Slime => 34.0,
            EnemyKind::Bat => 62.0,
            EnemyKind::Skeleton => 42.0,
        }
    }

    pub fn size(self) -> (f32, f32) {
        match self {
            EnemyKind::Slime => (14.0, 12.0),
            EnemyKind::Bat => (14.0, 10.0),
            EnemyKind::Skeleton => (12.0, 26.0),
        }
    }

    pub fn flying(self) -> bool {
        matches!(self, EnemyKind::Bat)
    }

}

pub struct Enemy {
    pub kind: EnemyKind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hp: i32,
    pub hurt: f32,
    pub on_ground: bool,
    pub bob: f32,
}

impl Enemy {
    pub fn new(kind: EnemyKind, x: f32, y: f32) -> Enemy {
        Enemy {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hp: kind.hp(),
            hurt: 0.0,
            on_ground: false,
            bob: 0.0,
        }
    }

    pub fn w(&self) -> f32 {
        self.kind.size().0
    }

    pub fn h(&self) -> f32 {
        self.kind.size().1
    }

    pub fn update(&mut self, dt: f32, world: &World, target: (f32, f32), rng: &mut Rng) {
        self.hurt = (self.hurt - dt).max(0.0);
        self.bob += dt * 6.0;

        let (tx, ty) = target;
        let dx = tx - (self.x + self.w() * 0.5);
        let dy = ty - (self.y + self.h() * 0.5);
        let dist = (dx * dx + dy * dy).sqrt().max(1.0);
        let dir = if dx > 0.0 { 1.0 } else { -1.0 };

        match self.kind {
            EnemyKind::Slime => {
                // Hops toward you on a timer.
                if self.on_ground {
                    self.vx = dir * self.kind.speed();
                    if dist < 260.0 && rng.next() > 0.982 {
                        self.vy = -210.0;
                        self.on_ground = false;
                    }
                }
                self.vy += 1150.0 * dt;
            }
            EnemyKind::Skeleton => {
                self.vx = dir * self.kind.speed();
                // hop over single-tile steps
                if self.on_ground && self.blocked_ahead(world, dir) {
                    self.vy = -260.0;
                    self.on_ground = false;
                }
                self.vy += 1150.0 * dt;
            }
            EnemyKind::Bat => {
                if dist < 300.0 {
                    let norm = dist;
                    self.vx = (dx / norm) * self.kind.speed();
                    self.vy = (dy / norm) * self.kind.speed() + (self.bob * 3.0).sin() * 22.0;
                } else {
                    self.vx = dir * self.kind.speed() * 0.4;
                    self.vy = (self.bob * 2.0).sin() * 18.0;
                }
            }
        }

        if self.vy > 800.0 {
            self.vy = 800.0;
        }

        self.move_axis(world, self.vx * dt, 0.0);
        let was = self.on_ground;
        self.on_ground = false;
        self.move_axis(world, 0.0, self.vy * dt);
        if self.kind == EnemyKind::Bat && (was || !self.on_ground) {
            // bats ignore gravity
            self.vy *= 0.9;
        }

        // flyers despawn far away rather than falling forever
        if self.y > world.h as f32 * 16.0 + 300.0 {
            self.hp = 0;
        }
    }

    fn blocked_ahead(&self, world: &World, dir: f32) -> bool {
        let probe_x = if dir > 0.0 {
            self.x + self.w() + 2.0
        } else {
            self.x - 2.0
        };
        let tx = (probe_x / 16.0) as i32;
        let ty = ((self.y + self.h() - 2.0) / 16.0) as i32;
        world.is_solid(tx, ty)
    }

    fn move_axis(&mut self, world: &World, dx: f32, dy: f32) {
        if dx != 0.0 {
            self.x += dx;
            let (x0, x1, _, _) = self.bounds(world);
            if x0 != i32::MAX {
                if dx > 0.0 {
                    self.x = x0 as f32 * 16.0 - self.w() - 0.01;
                } else {
                    self.x = (x1 + 1) as f32 * 16.0 + 0.01;
                }
                self.vx = 0.0;
            }
        }
        if dy != 0.0 {
            self.y += dy;
            let (_, _, y0, y1) = self.bounds(world);
            if y0 != i32::MAX {
                if dy > 0.0 {
                    self.y = y0 as f32 * 16.0 - self.h() - 0.01;
                    self.on_ground = true;
                } else {
                    self.y = (y1 + 1) as f32 * 16.0 + 0.01;
                }
                self.vy = 0.0;
            }
        }
    }

    fn bounds(&self, world: &World) -> (i32, i32, i32, i32) {
        let tx0 = (self.x / 16.0).floor() as i32;
        let tx1 = ((self.x + self.w() - 0.01) / 16.0).floor() as i32;
        let ty0 = (self.y / 16.0).floor() as i32;
        let ty1 = ((self.y + self.h() - 0.01) / 16.0).floor() as i32;
        let (mut a, mut b, mut c, mut d) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if !world.is_solid(tx, ty) {
                    continue;
                }
                a = a.min(tx);
                b = b.max(tx);
                c = c.min(ty);
                d = d.max(ty);
            }
        }
        (a, b, c, d)
    }

    pub fn hit(&mut self, dmg: i32, from_x: f32) -> bool {
        self.hp -= dmg;
        self.hurt = 0.25;
        let away = if self.x + self.w() * 0.5 > from_x { 1.0 } else { -1.0 };
        self.vx = away * 170.0;
        self.vy = -130.0;
        self.hp <= 0
    }
}

pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: [u8; 3],
    pub size: f32,
    pub gravity: f32,
}

impl Particle {
    pub fn update(&mut self, dt: f32, world: &World) {
        self.life -= dt;
        self.vy += self.gravity * dt;
        let nx = self.x + self.vx * dt;
        let ny = self.y + self.vy * dt;
        if world.is_solid_at(nx, ny) {
            self.vx *= -0.35;
            self.vy *= -0.25;
        } else {
            self.x = nx;
            self.y = ny;
        }
        self.vx *= 1.0 - 1.8 * dt;
    }
}

pub struct Spawner {
    pub rng: Rng,
    pub timer: f32,
    pub difficulty: f32,
}

impl Spawner {
    pub fn new(seed: u32) -> Spawner {
        Spawner {
            rng: Rng::new(seed ^ 0xbeef),
            timer: 2.0,
            difficulty: 1.0,
        }
    }

    /// Spawn near the camera edges, on solid ground, out of sight.
    pub fn try_spawn(
        &mut self,
        dt: f32,
        world: &World,
        cam: (f32, f32),
        view: (f32, f32),
        night: f32,
    ) -> Option<Enemy> {
        self.timer -= dt;
        if self.timer > 0.0 {
            return None;
        }
        let base = 3.4 / self.difficulty.max(0.2);
        self.timer = base * (0.6 + self.rng.next() * 0.9);

        // more spawns at night
        if self.rng.next() > 0.35 + night * 0.45 {
            return None;
        }

        let side = if self.rng.next() > 0.5 { 1.0 } else { -1.0 };
        let px = if side > 0.0 {
            cam.0 + view.0 + 40.0
        } else {
            cam.0 - 40.0
        };
        let tx = (px / 16.0) as i32;
        if tx < 2 || tx >= world.w - 2 {
            return None;
        }
        let surface = world.column_height(tx);

        // pick a kind by depth availability
        let roll = self.rng.next();
        let kind = if roll > 0.75 {
            EnemyKind::Skeleton
        } else if roll > 0.45 {
            EnemyKind::Bat
        } else {
            EnemyKind::Slime
        };

        if kind.flying() {
            let y = (surface as f32 - self.rng.range(2.0, 12.0)) * 16.0;
            return Some(Enemy::new(kind, tx as f32 * 16.0, y.max(16.0)));
        }

        // walk down from the surface to find open floor
        for dy in 0..4 {
            let y = surface + dy;
            if world.is_solid(tx, y) && !world.is_solid(tx, y - 1) {
                return Some(Enemy::new(
                    kind,
                    tx as f32 * 16.0,
                    (y - 1) as f32 * 16.0 - kind.size().1,
                ));
            }
        }
        None
    }
}

pub fn tile_particles(
    out: &mut Vec<Particle>,
    rng: &mut Rng,
    tx: i32,
    ty: i32,
    tile: Tile,
    count: usize,
) {
    let base = match tile {
        Tile::Grass => [104, 168, 62],
        Tile::Dirt => [116, 78, 46],
        Tile::Stone => [110, 110, 120],
        Tile::Sand => [206, 186, 122],
        Tile::Wood => [122, 88, 50],
        Tile::Leaves => [70, 140, 56],
        Tile::Ore => [212, 186, 86],
        Tile::Snow => [232, 240, 250],
        Tile::Ice => [160, 204, 236],
        Tile::Hellstone => [222, 104, 52],
        Tile::Ash => [86, 76, 70],
        _ => [130, 130, 130],
    };
    for _ in 0..count {
        out.push(Particle {
            x: tx as f32 * 16.0 + rng.range(2.0, 14.0),
            y: ty as f32 * 16.0 + rng.range(2.0, 14.0),
            vx: rng.range(-70.0, 70.0),
            vy: rng.range(-140.0, -20.0),
            life: rng.range(0.35, 0.8),
            max_life: 0.8,
            color: base,
            size: rng.range(1.0, 3.0),
            gravity: 620.0,
        });
    }
}

pub fn spark_particles(out: &mut Vec<Particle>, rng: &mut Rng, x: f32, y: f32, color: [u8; 3]) {
    for _ in 0..10 {
        out.push(Particle {
            x,
            y,
            vx: rng.range(-160.0, 160.0),
            vy: rng.range(-180.0, 60.0),
            life: rng.range(0.18, 0.45),
            max_life: 0.45,
            color,
            size: rng.range(1.0, 2.5),
            gravity: 300.0,
        });
    }
}

pub fn aabb(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

