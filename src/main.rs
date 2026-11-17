mod entities;
mod light;
mod player;
mod render;
mod util;
mod world;

use entities::{aabb, spark_particles, tile_particles, Enemy, Particle, Spawner};
use light::{Lighting, TorchLight};
use player::{Input, Player};
use render::Renderer;
use util::{Rgb, Tile};

const REACH: f32 = 6.0;
const ATTACK_RANGE: f32 = 46.0;

#[derive(Default)]
struct Inventory {
    dirt: i32,
    stone: i32,
    wood: i32,
    sand: i32,
    snow: i32,
    ore: i32,
    hellstone: i32,
    gems: i32,
}

impl Inventory {
    fn add(&mut self, t: Tile) {
        match t {
            Tile::Dirt | Tile::Grass => self.dirt += 1,
            Tile::Stone | Tile::Sandstone | Tile::Ash => self.stone += 1,
            Tile::Wood | Tile::Plank | Tile::Cactus => self.wood += 1,
            Tile::Sand => self.sand += 1,
            Tile::Snow | Tile::Ice => self.snow += 1,
            Tile::Ore => {
                self.ore += 1;
                if self.ore % 3 == 0 {
                    self.gems += 1;
                }
            }
            Tile::Hellstone => self.hellstone += 1,
            _ => {}
        }
    }

}

struct Game {
    world: world::World,
    player: Player,
    lighting: Lighting,
    renderer: Renderer,
    cam: (f32, f32),
    zoom: f32,
    mining: (i32, i32, f32),
    sky: f32,
    clock: f32,
    day: f32,
    sun_x: f32,
    enemies: Vec<Enemy>,
    particles: Vec<Particle>,
    spawner: Spawner,
    inv: Inventory,
    hp: f32,
    max_hp: f32,
    invuln: f32,
    attack_cd: f32,
    swing: f32,
    kills: i32,
    rng: util::Rng,
    game_over: f32,
}

impl Game {
    fn new(out_w: i32, out_h: i32, seed: u32) -> Game {
        let scale = if out_h >= 720 { 2 } else { 1 };
        let width = (out_w / scale).max(320);
        let height = (out_h / scale).max(240);
        let world = world::World::new(420, 300, seed);
        let lighting = Lighting::new(&world);
        let renderer = Renderer::new(width, height);
        let (sx, sy) = world.spawn_point();
        let player = Player::new(sx - 6.0, sy - 44.0);
        let zoom = (height as f32 / 420.0).max(1.0);
        let mut g = Game {
            world,
            player,
            lighting,
            renderer,
            cam: (0.0, 0.0),
            zoom,
            mining: (-1, -1, 0.0),
            sky: 1.0,
            clock: 0.28,
            day: 1.0,
            sun_x: 0.5,
            enemies: Vec::new(),
            particles: Vec::new(),
            spawner: Spawner::new(seed),
            inv: Inventory::default(),
            hp: 100.0,
            max_hp: 100.0,
            invuln: 0.0,
            attack_cd: 0.0,
            swing: 0.0,
            kills: 0,
            rng: util::Rng::new(seed ^ 0x5eed),
            game_over: 0.0,
        };
        g.place_starter_torches();
        g.center_camera();
        g
    }

    fn place_starter_torches(&mut self) {
        let px = (self.player.x / 16.0) as i32;
        let py = (self.player.y / 16.0) as i32;
        for dy in 1..40 {
            if self.world.is_solid(px, py + dy) {
                for ox in [-4, 3] {
                    let tx = px + ox;
                    let ty = py + dy - 1;
                    if self.world.get(tx, ty) == Tile::Air {
                        self.world.set(tx, ty, Tile::Torch);
                    }
                }
                break;
            }
        }
        let gy = py + 20;
        for ox in [-7, -2, 5] {
            let tx = px + ox;
            if self.world.get(tx, gy) == Tile::Air {
                self.world.set(tx, gy, Tile::Torch);
            }
        }
    }

    fn center_camera(&mut self) {
        let vw = self.renderer.width as f32 / self.zoom;
        let vh = self.renderer.height as f32 / self.zoom;
        self.cam = (self.player.cx() - vw * 0.5, self.player.cy() - vh * 0.5);
    }

    fn torches(&self) -> Vec<TorchLight> {
        let vw = self.renderer.width as f32 / self.zoom;
        let vh = self.renderer.height as f32 / self.zoom;
        let x0 = ((self.cam.0 / 16.0).floor() as i32 - 1).max(0);
        let y0 = ((self.cam.1 / 16.0).floor() as i32 - 1).max(0);
        let x1 = (((self.cam.0 + vw) / 16.0).ceil() as i32 + 1).min(self.world.w - 1);
        let y1 = (((self.cam.1 + vh) / 16.0).ceil() as i32 + 1).min(self.world.h - 1);
        let mut out = Vec::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.world.get(x, y) == Tile::Torch {
                    out.push(TorchLight {
                        x,
                        y,
                        rgb: Rgb { r: 1.0, g: 0.72, b: 0.36 },
                    });
                }
            }
        }
        out
    }

    fn update(&mut self, dt: f32, input: &Input) {
        // Full day cycle = 6 minutes.
        self.clock = (self.clock + dt / 360.0) % 1.0;
        // day: 1 at noon, 0 at midnight
        let ang = (self.clock - 0.25) * std::f32::consts::TAU;
        self.day = (ang.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        self.sun_x = self.clock;
        self.player.update(dt, input, &self.world);

        let vw = self.renderer.width as f32 / self.zoom;
        let vh = self.renderer.height as f32 / self.zoom;
        let tx = self.player.cx() - vw * 0.5;
        let ty = self.player.cy() - vh * 0.5;
        let k = 1.0 - 0.0015f32.powf(dt);
        self.cam.0 += (tx - self.cam.0) * k;
        self.cam.1 += (ty - self.cam.1) * k;
        self.cam.0 = self.cam.0.clamp(0.0, (self.world.w as f32 * 16.0 - vw).max(0.0));
        self.cam.1 = self.cam.1.clamp(0.0, (self.world.h as f32 * 16.0 - vh).max(0.0));

        let surface = self.world.column_height((self.player.cx() / 16.0) as i32);
        let depth = (self.player.cy() / 16.0) as i32 - surface;
        // Depth dims you; time of day sets the ceiling on how bright the sky is.
        let daylight = 0.10 + self.day * 0.90;
        self.sky = ((1.0 - depth as f32 * 0.010).clamp(0.08, 1.0)) * daylight;

        self.invuln = (self.invuln - dt).max(0.0);
        self.attack_cd = (self.attack_cd - dt).max(0.0);
        self.swing = (self.swing - dt).max(0.0);

        // ---- enemy spawning -------------------------------------------
        let night = 1.0 - self.day;
        if self.hp > 0.0 {
            let view = (vw, vh);
            if let Some(e) = self.spawner.try_spawn(dt, &self.world, self.cam, view, night) {
                self.enemies.push(e);
            }
        }

        // ---- enemy simulation -----------------------------------------
        let target = (self.player.cx(), self.player.cy());
        let mut rng = std::mem::take(&mut self.rng);
        self.enemies.retain(|_| true);
        for e in self.enemies.iter_mut() {
            e.update(dt, &self.world, target, &mut rng);
        }
        self.rng = rng;

        // ---- contact damage -------------------------------------------
        for e in self.enemies.iter() {
            if aabb(
                self.player.x,
                self.player.y,
                player::PLAYER_W,
                player::PLAYER_H,
                e.x,
                e.y,
                e.w(),
                e.h(),
            ) {
                if self.invuln <= 0.0 && self.hp > 0.0 {
                    self.hp -= e.kind.damage() as f32;
                    self.invuln = 0.75;
                    let knock = if self.player.cx() < e.x + e.w() * 0.5 {
                        -1.0
                    } else {
                        1.0
                    };
                    self.player.vx = knock * 180.0;
                    self.player.vy = -140.0;
                    let mut rng = std::mem::take(&mut self.rng);
                    spark_particles(&mut self.particles, &mut rng, self.player.cx(), self.player.cy(), [220, 70, 70]);
                    self.rng = rng;
                    if self.hp <= 0.0 {
                        self.hp = 0.0;
                        self.game_over = 2.5;
                    }
                }
            }
        }

        // ---- particles ------------------------------------------------
        let world = &self.world;
        for p in self.particles.iter_mut() {
            p.update(dt, world);
        }
        self.particles.retain(|p| p.life > 0.0);

        self.enemies.retain(|e| {
            e.hp > 0
                && e.x > self.cam.0 - 900.0
                && e.x < self.cam.0 + vw + 900.0
                && e.y < self.world.h as f32 * 16.0
        });
        if self.enemies.len() > 90 {
            self.enemies.drain(0..(self.enemies.len() - 90));
        }

        if self.game_over > 0.0 {
            self.game_over -= dt;
            if self.game_over <= 0.0 {
                self.respawn();
            }
        }
    }

    fn width_scale(&self) -> i32 {
        (self.renderer.width / 200).max(2)
    }

    fn attack(&mut self) {
        if self.attack_cd > 0.0 || self.hp <= 0.0 {
            return;
        }
        self.attack_cd = 0.34;
        self.swing = 0.18;
        let reach = ATTACK_RANGE;
        let px = self.player.cx();
        let py = self.player.cy();
        let facing = self.player.facing as f32;

        let mut rng = std::mem::take(&mut self.rng);
        let mut killed = 0;
        for e in self.enemies.iter_mut() {
            let ex = e.x + e.w() * 0.5;
            let ey = e.y + e.h() * 0.5;
            let dx = ex - px;
            let dy = ey - py;
            let ahead = if facing > 0.0 { dx > -6.0 } else { dx < 6.0 };
            if ahead && dx.abs() < reach && dy.abs() < 30.0 {
                let dmg = 9;
                if e.hit(dmg, px) {
                    killed += 1;
                    spark_particles(&mut self.particles, &mut rng, ex, ey, [240, 220, 120]);
                } else {
                    spark_particles(&mut self.particles, &mut rng, ex, ey, [220, 150, 90]);
                }
            }
        }
        self.rng = rng;
        self.kills += killed;
    }

    fn respawn(&mut self) {
        let (sx, sy) = self.world.spawn_point();
        self.player.x = sx - 6.0;
        self.player.y = sy - 44.0;
        self.player.vx = 0.0;
        self.player.vy = 0.0;
        self.hp = self.max_hp;
        self.invuln = 1.5;
        self.enemies.clear();
        self.particles.clear();
    }

    fn mine(&mut self, tx: i32, ty: i32, dt: f32) {
        let dx = (tx as f32 + 0.5) * 16.0 - self.player.cx();
        let dy = (ty as f32 + 0.5) * 16.0 - self.player.cy();
        let max = REACH * 16.0;
        if dx * dx + dy * dy > max * max {
            return;
        }
        let cur = self.world.get(tx, ty);
        if cur == Tile::Air {
            return;
        }
        if !cur.breakable() {
            return;
        }
        if self.mining.0 != tx || self.mining.1 != ty {
            self.mining = (tx, ty, 0.0);
        }
        self.mining.2 += dt / cur.hardness().max(0.05);
        // chipping particles while digging
        if self.mining.2 > 0.05 && (self.mining.2 * 26.0) as i32 % 3 == 0 {
            let mut rng = std::mem::take(&mut self.rng);
            tile_particles(&mut self.particles, &mut rng, tx, ty, cur, 2);
            self.rng = rng;
        }
        if self.mining.2 >= 1.0 {
            self.world.set(tx, ty, Tile::Air);
            self.inv.add(cur);
            let mut rng = std::mem::take(&mut self.rng);
            tile_particles(&mut self.particles, &mut rng, tx, ty, cur, 10);
            self.rng = rng;
            self.mining = (-1, -1, 0.0);
            // support check: floating vegetation above falls
            for dy in 1..4 {
                let a = self.world.get(tx, ty - dy);
                if a == Tile::Leaves || a == Tile::Cactus || a == Tile::GrowingPlant {
                    self.world.set(tx, ty - dy, Tile::Air);
                }
            }
        }
    }

    fn place(&mut self, tx: i32, ty: i32) {
        if self.world.get(tx, ty) != Tile::Air {
            return;
        }
        let px = (self.player.x / 16.0) as i32;
        let py = (self.player.y / 16.0) as i32;
        if tx == px && (ty == py || ty == py + 1) {
            return;
        }
        let dx = (tx as f32 + 0.5) * 16.0 - self.player.cx();
        let dy = (ty as f32 + 0.5) * 16.0 - self.player.cy();
        let max = REACH * 16.0;
        if dx * dx + dy * dy > max * max {
            return;
        }
        self.world.set(tx, ty, Tile::Plank);
    }

    fn render(&mut self, mouse: (i32, i32), selected: Option<(i32, i32)>) {
        let vw = self.renderer.width as f32 / self.zoom;
        let vh = self.renderer.height as f32 / self.zoom;
        let _ = mouse;

        let torches = self.torches();
        self.lighting.compute(
            &self.world,
            (self.cam.0 / 16.0).floor() as i32,
            (self.cam.1 / 16.0).floor() as i32,
            ((self.cam.0 + vw) / 16.0).ceil() as i32,
            ((self.cam.1 + vh) / 16.0).ceil() as i32,
            self.sky,
            &torches,
        );

        self.renderer.clear([0, 0, 0, 255]);
        self.renderer
            .draw_sky(&self.world, self.cam.0, self.cam.1, self.zoom, self.day, self.sun_x);
        self.renderer
            .draw_world(&self.world, self.cam.0, self.cam.1, self.zoom, self.clock * 360.0);
        self.renderer
            .composite_light(&self.lighting, &self.world, self.cam.0, self.cam.1, self.zoom, &torches);
        self.renderer
            .draw_enemies(&self.enemies, self.cam.0, self.cam.1, self.zoom);
        if self.invuln > 0.0 && (self.invuln * 12.0) as i32 % 2 == 0 {
            // blink while invulnerable
        } else {
            self.renderer
                .draw_player(&self.player, self.cam.0, self.cam.1, self.zoom);
        }
        self.renderer
            .draw_swing(&self.player, self.cam.0, self.cam.1, self.zoom, self.swing);
        self.renderer
            .draw_particles(&self.particles, self.cam.0, self.cam.1, self.zoom);

        let biome = self
            .world
            .biome_at((self.player.cx() / 16.0) as i32)
            .name();
        self.renderer.draw_hud(
            self.hp,
            self.max_hp,
            self.kills,
            &self.inv,
            self.clock,
            biome,
            0.0,
        );

        if self.game_over > 0.0 {
            self.renderer
                .draw_center_text("YOU DIED", [220, 70, 70], self.width_scale());
        }

        if let Some((tx, ty)) = selected {
            let mining = if self.mining.0 == tx && self.mining.1 == ty {
                self.mining.2
            } else {
                0.0
            };
            self.renderer
                .draw_selection(tx, ty, self.cam.0, self.cam.1, self.zoom, mining);
        }
    }
}

fn selftest() {
    println!("nerthus selftest");
    let world = world::World::new(420, 300, 424242);
    let counts = world.tile_counts();
    let total: usize = counts.iter().sum();
    for (i, c) in counts.iter().enumerate() {
        if *c == 0 {
            continue;
        }
        println!(
            "  {:12} {:8} ({:.2}%)",
            Tile::from_u8(i as u8).name(),
            c,
            *c as f32 / total as f32 * 100.0
        );
    }
    assert_eq!(total, (world.w * world.h) as usize, "tile count mismatch");

    let w2 = world::World::new(420, 300, 424242);
    assert_eq!(world.tiles, w2.tiles, "worldgen not deterministic");

    let w3 = world::World::new(420, 300, 999);
    assert_ne!(world.tiles, w3.tiles, "worldgen not seed sensitive");

    let mut dead = 0;
    for x in 0..world.w {
        let mut y = 0;
        while y < world.h && !world.is_solid(x, y) {
            y += 1;
        }
        if y >= world.h {
            dead += 1;
        }
    }
    assert_eq!(dead, 0, "columns with no ground");
    println!("  biomes: {:?}", world.biome_counts());
    {
        let mut spans: Vec<(String, i32, i32)> = Vec::new();
        let mut cur = world.biome_at(0);
        let mut start = 0;
        for x in 1..world.w {
            let b = world.biome_at(x);
            if b != cur {
                spans.push((cur.name().to_string(), start, x - 1));
                cur = b;
                start = x;
            }
        }
        spans.push((cur.name().to_string(), start, world.w - 1));
        for (n, a, b) in spans.iter().filter(|(_, a, b)| b - a > 12) {
            println!("   biome {}: x={}..{}", n, a, b);
        }
    }
    println!("  worldgen: deterministic, seed-sensitive, no dead columns");

    let mut lighting = Lighting::new(&world);
    let (sx, sy) = world.spawn_point();
    let mut player = Player::new(sx - 6.0, sy - 44.0);
    let idle = Input::default();
    for _ in 0..240 {
        player.update(1.0 / 60.0, &idle, &world);
    }
    println!(
        "  player settled at ({:.2}, {:.2}) on_ground={}",
        player.x, player.y, player.on_ground
    );
    assert!(player.on_ground, "player did not land");

    let settled = player.y;
    let mut min_y = player.y;
    let jump = Input { jump: true, ..Default::default() };
    for _ in 0..200 {
        player.update(1.0 / 60.0, &jump, &world);
        min_y = min_y.min(player.y);
    }
    let rise = (settled - min_y) / 16.0;
    println!("  jump height {:.2} tiles", rise);
    assert!(rise > 3.0 && rise < 6.0, "jump height out of range");

    let t0 = std::time::Instant::now();
    let torches = vec![TorchLight {
        x: 200,
        y: 120,
        rgb: Rgb { r: 1.0, g: 0.7, b: 0.35 },
    }];
    lighting.compute(&world, 0, 0, 120, 80, 1.0, &torches);
    println!("  lighting 121x81 region in {:?}", t0.elapsed());
    let lit = lighting.r.iter().filter(|v| **v > 0.01).count();
    println!("  lit tiles: {}", lit);
    assert!(lit > 0, "lighting produced no light");

    let mut full = Game::new(1280, 800, 424242);
    let tfull = std::time::Instant::now();
    for _ in 0..30 {
        full.update(1.0/60.0, &Input::default());
        full.render((0,0), None);
    }
    println!("  full frame @ {}x{} internal: {:?}/frame",
        full.renderer.width, full.renderer.height, tfull.elapsed()/30);
    println!("  -> {:.0} fps equivalent", 30.0/tfull.elapsed().as_secs_f64());

    let mut renderer = Renderer::new(1280, 800);
    let t1 = std::time::Instant::now();
    for _ in 0..10 {
        renderer.draw_sky(&world, 3200.0, 1400.0, 2.0, 1.0, 0.5);
        renderer.draw_world(&world, 3200.0, 1400.0, 2.0, 0.0);
        renderer.composite_light(&lighting, &world, 3200.0, 1400.0, 2.0, &torches);
        renderer.draw_player(&player, 3200.0, 1400.0, 2.0);
        renderer.draw_selection(10, 10, 3200.0, 1400.0, 2.0, 0.5);
    }
    println!("  10 frames at 1280x800: {:?}", t1.elapsed());
    assert_eq!(renderer.framebuffer().len(), (1280 * 800 * 4) as usize);

    println!("ALL OK");
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for i in 0..256u32 {
        let mut c = i;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB88320 ^ (c >> 1) } else { c >> 1 };
        }
        table[i as usize] = c;
    }
    let mut c = 0xFFFFFFFFu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFFFFFF
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let mut with_kind = kind.to_vec();
    with_kind.extend_from_slice(body);
    out.extend_from_slice(&with_kind);
    out.extend_from_slice(&crc32(&with_kind).to_be_bytes());
}

fn write_png(path: &str, w: u32, h: u32, rgba: &[u8]) {
    let mut raw = Vec::with_capacity((h * (w * 4 + 1)) as usize);
    for y in 0..h {
        raw.push(0);
        let row = (y * w * 4) as usize;
        raw.extend_from_slice(&rgba[row..row + (w * 4) as usize]);
    }

    let mut z = vec![0x78u8, 0x01];
    let mut i = 0usize;
    while i < raw.len() {
        let n = (raw.len() - i).min(65535);
        let final_block = i + n >= raw.len();
        z.push(if final_block { 0x01 } else { 0x00 });
        z.extend_from_slice(&(n as u16).to_le_bytes());
        z.extend_from_slice(&(!(n as u16)).to_le_bytes());
        z.extend_from_slice(&raw[i..i + n]);
        i += n;
    }
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in raw.iter() {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&(((b << 16) | a).to_be_bytes()));

    let mut png = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &z);
    png_chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png).expect("write png");
}

fn dump_frame(seed: u32, path: &str, clock: Option<f32>, at_x: Option<f32>) {
    let out_w = 1280;
    let out_h = 800;
    let mut game = Game::new(out_w, out_h, seed);
    if let Some(c) = clock {
        game.clock = c;
    }
    for _ in 0..900 {
        game.update(1.0 / 60.0, &Input::default());
    }
    if let Some(c) = clock {
        game.clock = c;
        let ang = (c - 0.25) * std::f32::consts::TAU;
        game.day = (ang.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        game.sun_x = c;
    }
    if let Some(x) = at_x {
        let vw = game.renderer.width as f32 / game.zoom;
        game.cam.0 = (x * 16.0 - vw * 0.5).clamp(0.0, game.world.w as f32 * 16.0 - vw);
    } else {
        game.center_camera();
    }
    game.render((0, 0), None);
    let (iw, ih) = (game.renderer.width as u32, game.renderer.height as u32);
    write_png(path, iw, ih, game.renderer.framebuffer());
    println!("wrote {} ({}x{} internal, {}x{} window) seed {}", path, iw, ih, out_w, out_h, seed);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest" || a == "--headless") {
        selftest();
        return;
    }

    let seed_arg = args
        .iter()
        .find_map(|a| a.strip_prefix("--seed=").and_then(|v| v.parse::<u32>().ok()));

    if let Some(pos) = args.iter().position(|a| a == "--dump-frame") {
        let path = args.get(pos + 1).cloned().unwrap_or_else(|| "frame.png".to_string());
        let clock = args
            .iter()
            .find_map(|a| a.strip_prefix("--clock=").and_then(|v| v.parse::<f32>().ok()));
        let at_x = args
            .iter()
            .find_map(|a| a.strip_prefix("--at=").and_then(|v| v.parse::<f32>().ok()));
        dump_frame(seed_arg.unwrap_or(424242), &path, clock, at_x);
        return;
    }

    if let Some(pos) = args.iter().position(|a| a == "--probe") {
        let g = Game::new(1280, 800, seed_arg.unwrap_or(424242));
        let cols: Vec<i32> = args.get(pos + 1)
            .map(|v| v.split(',').filter_map(|n| n.parse::<i32>().ok()).collect())
            .unwrap_or_default();
        for x in cols {
            let surf = g.world.column_height(x);
            let mut col = String::new();
            for y in (surf - 3)..(surf + 6) {
                col.push_str(&format!("{} ", g.world.get(x, y).name()));
            }
            println!("x={:3} surface={:3} | {}", x, surf, col);
        }
        return;
    }

    let sdl = sdl2::init().expect("SDL init failed");
    let video = sdl.video().expect("SDL video failed");
    let out_w = 1280;
    let out_h = 800;
    let window = video
        .window("Nerthus", out_w as u32, out_h as u32)
        .position_centered()
        .build()
        .expect("window failed");
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .expect("canvas failed");

    let seed = seed_arg.unwrap_or_else(|| {
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(7))
        ^ 0x9e3779b9
    });

    println!("Nerthus — seed {} (rerun with --seed={} to reproduce)", seed, seed);
    let mut game = Game::new(out_w, out_h, seed);

    let tex_creator = canvas.texture_creator();
    let mut texture = tex_creator
        .create_texture_streaming(
            sdl2::pixels::PixelFormatEnum::RGBA32,
            game.renderer.width as u32,
            game.renderer.height as u32,
        )
        .expect("texture failed");

    let mut event_pump = sdl.event_pump().expect("event pump");
    let mut input = Input::default();
    let mut mouse_down = false;
    let mut right_down = false;
    let mut mouse = (0, 0);
    let mut shift = false;
    let mut input_attack = false;
    let mut last = std::time::Instant::now();
    let (scale_x, scale_y) = (
        out_w as f32 / game.renderer.width as f32,
        out_h as f32 / game.renderer.height as f32,
    );

    'running: loop {
        for ev in event_pump.poll_iter() {
            use sdl2::event::Event;
            use sdl2::keyboard::Keycode;
            use sdl2::mouse::MouseButton;
            match ev {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(k), repeat: false, ..
                } => match k {
                    Keycode::Escape => break 'running,
                    Keycode::A | Keycode::Left => input.left = true,
                    Keycode::D | Keycode::Right => input.right = true,
                    Keycode::W | Keycode::Up | Keycode::Space => input.jump = true,
                    Keycode::LShift | Keycode::RShift => shift = true,
                    Keycode::J | Keycode::F | Keycode::C => input_attack = true,
                    _ => {}
                },
                Event::KeyUp {
                    keycode: Some(k), ..
                } => match k {
                    Keycode::A | Keycode::Left => input.left = false,
                    Keycode::D | Keycode::Right => input.right = false,
                    Keycode::W | Keycode::Up | Keycode::Space => input.jump = false,
                    Keycode::LShift | Keycode::RShift => shift = false,
                    Keycode::J | Keycode::F | Keycode::C => input_attack = false,
                    _ => {}
                },
                Event::MouseMotion { x, y, .. } => mouse = (x, y),
                Event::MouseButtonDown {
                    mouse_btn,
                    x,
                    y,
                    ..
                } => {
                    mouse = (x, y);
                    match mouse_btn {
                        MouseButton::Left => mouse_down = true,
                        MouseButton::Right => right_down = true,
                        _ => {}
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => match mouse_btn {
                    MouseButton::Left => mouse_down = false,
                    MouseButton::Right => right_down = false,
                    _ => {}
                },
                _ => {}
            }
        }

        let now = std::time::Instant::now();
        let mut dt = now.duration_since(last).as_secs_f32();
        last = now;
        if dt > 0.05 {
            dt = 0.05;
        }

        let iw = game.renderer.width;
        let wx = mouse.0 as f32 / (scale_x * game.zoom) + game.cam.0;
        let wy = mouse.1 as f32 / (scale_y * game.zoom) + game.cam.1;
        let tx = (wx / 16.0) as i32;
        let ty = (wy / 16.0) as i32;
        let in_bounds = tx >= 0 && ty >= 0 && tx < game.world.w && ty < game.world.h;

        game.update(dt, &input);
        if right_down || input_attack {
            game.attack();
        }
        if mouse_down && in_bounds {
            if shift {
                game.place(tx, ty);
            } else {
                game.mine(tx, ty, dt);
            }
        }
        game.render(mouse, if in_bounds { Some((tx, ty)) } else { None });

        texture
            .update(None, game.renderer.framebuffer(), iw as usize * 4)
            .expect("texture update");
        canvas.clear();
        canvas.copy(&texture, None, None).expect("copy");
        canvas.present();
    }
}
