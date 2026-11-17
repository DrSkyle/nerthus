use crate::util::clampf;
use crate::world::World;

pub const PLAYER_W: f32 = 12.0;
pub const PLAYER_H: f32 = 28.0;

const GRAVITY: f32 = 1350.0;
const MAX_FALL: f32 = 900.0;
const MOVE_ACCEL: f32 = 1500.0;
const MOVE_SPEED: f32 = 175.0;
const FRICTION: f32 = 12.0;
const JUMP_VELOCITY: f32 = -430.0;

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub facing: i32,
    pub walk_phase: f32,
    jump_held: bool,
    coyote: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Player {
        Player {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            on_ground: false,
            facing: 1,
            walk_phase: 0.0,
            jump_held: false,
            coyote: 0.0,
        }
    }

    pub fn cx(&self) -> f32 {
        self.x + PLAYER_W * 0.5
    }

    pub fn cy(&self) -> f32 {
        self.y + PLAYER_H * 0.5
    }

    pub fn update(&mut self, dt: f32, input: &Input, world: &World) {
        let dir = (input.right as i32 - input.left as i32) as f32;
        if dir != 0.0 {
            self.vx += dir * MOVE_ACCEL * dt;
            self.vx = clampf(self.vx, -MOVE_SPEED, MOVE_SPEED);
            self.facing = if dir > 0.0 { 1 } else { -1 };
        } else {
            let damp = (1.0 - FRICTION * dt).max(0.0);
            self.vx *= damp;
            if self.vx.abs() < 1.0 {
                self.vx = 0.0;
            }
        }

        if self.on_ground {
            self.coyote = 0.1;
        } else {
            self.coyote = (self.coyote - dt).max(0.0);
        }

        if input.jump && !self.jump_held && self.coyote > 0.0 {
            self.vy = JUMP_VELOCITY;
            self.on_ground = false;
            self.coyote = 0.0;
            self.jump_held = true;
        }
        if !input.jump {
            self.jump_held = false;
        }

        self.vy += GRAVITY * dt;
        if self.vy > MAX_FALL {
            self.vy = MAX_FALL;
        }

        self.move_axis(world, self.vx * dt, 0.0);
        self.on_ground = false;
        self.move_axis(world, 0.0, self.vy * dt);

        if self.vx.abs() > 8.0 && self.on_ground {
            self.walk_phase += dt * self.vx.abs() * 0.09;
        } else {
            self.walk_phase = 0.0;
        }

        if self.y > world.h as f32 * 16.0 + 200.0 {
            let (sx, sy) = world.spawn_point();
            self.x = sx - PLAYER_W * 0.5;
            self.y = sy - PLAYER_H;
            self.vx = 0.0;
            self.vy = 0.0;
        }
    }

    fn probes(&self, world: &World) -> (i32, i32, i32, i32) {
        let tx0 = (self.x / 16.0).floor() as i32;
        let tx1 = ((self.x + PLAYER_W - 0.01) / 16.0).floor() as i32;
        let ty0 = (self.y / 16.0).floor() as i32;
        let ty1 = ((self.y + PLAYER_H - 0.01) / 16.0).floor() as i32;
        let (mut min_tx, mut max_tx, mut min_ty, mut max_ty) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                if !world.is_solid(tx, ty) {
                    continue;
                }
                min_tx = min_tx.min(tx);
                max_tx = max_tx.max(tx);
                min_ty = min_ty.min(ty);
                max_ty = max_ty.max(ty);
            }
        }
        (min_tx, max_tx, min_ty, max_ty)
    }

    fn move_axis(&mut self, world: &World, dx: f32, dy: f32) {
        if dx != 0.0 {
            let prev = self.x;
            self.x += dx;
            let (min_tx, max_tx, _, _) = self.probes(world);
            if min_tx != i32::MAX {
                if dx > 0.0 {
                    self.x = min_tx as f32 * 16.0 - PLAYER_W - 0.01;
                } else {
                    self.x = (max_tx + 1) as f32 * 16.0 + 0.01;
                }
                self.vx = 0.0;
            }
            if !self.x.is_finite() || (self.x - prev).abs() > 64.0 {
                self.x = prev;
            }
        }
        if dy != 0.0 {
            self.y += dy;
            let (_, _, min_ty, max_ty) = self.probes(world);
            if min_ty != i32::MAX {
                if dy > 0.0 {
                    self.y = min_ty as f32 * 16.0 - PLAYER_H - 0.01;
                    self.on_ground = true;
                } else {
                    self.y = (max_ty + 1) as f32 * 16.0 + 0.01;
                }
                self.vy = 0.0;
            }
        }
    }
}
