use crate::enemy::{has_line_of_sight, Enemy};
use crate::font;
use crate::map;
use crate::player::Player;
use crate::raycaster::{self, FOV};
use crate::textures::{pack, TextureSet};
use minifb::{Key, Window};
use std::f32::consts::PI;

fn normalize_angle(angle: f32) -> f32 {
    (angle + PI).rem_euclid(2.0 * PI) - PI
}

pub struct Pickup {
    pub x: f32,
    pub y: f32,
    pub kind: PickupKind,
    pub taken: bool,
}

#[derive(Clone, Copy)]
pub enum PickupKind {
    Health,
    Ammo,
}

pub struct Game {
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub pickups: Vec<Pickup>,
    pub textures: TextureSet,
    pub won: bool,
    pub game_over: bool,
    pub move_speed: f32,
    pub turn_speed: f32,
}

impl Game {
    pub fn new() -> Self {
        let sp = map::spawns();
        let mut pickups = Vec::new();
        for (x, y) in sp.health_pickups {
            pickups.push(Pickup { x, y, kind: PickupKind::Health, taken: false });
        }
        for (x, y) in sp.ammo_pickups {
            pickups.push(Pickup { x, y, kind: PickupKind::Ammo, taken: false });
        }
        Game {
            player: Player::new(sp.player_start.0, sp.player_start.1),
            enemies: sp.enemy_spawns.into_iter().map(|(x, y)| Enemy::new(x, y)).collect(),
            pickups,
            textures: TextureSet::generate(),
            won: false,
            game_over: false,
            move_speed: 3.0,
            turn_speed: 2.5,
        }
    }

    pub fn update(&mut self, window: &Window, dt: f32) {
        if self.won || self.game_over {
            return;
        }

        if window.is_key_down(Key::Left) {
            self.player.dir_angle -= self.turn_speed * dt;
        }
        if window.is_key_down(Key::Right) {
            self.player.dir_angle += self.turn_speed * dt;
        }

        let (dx, dy) = self.player.dir();
        let mut mx = 0.0;
        let mut my = 0.0;
        if window.is_key_down(Key::W) {
            mx += dx;
            my += dy;
        }
        if window.is_key_down(Key::S) {
            mx -= dx;
            my -= dy;
        }
        if window.is_key_down(Key::A) {
            mx += dy;
            my -= dx;
        }
        if window.is_key_down(Key::D) {
            mx -= dy;
            my += dx;
        }
        let len = (mx * mx + my * my).sqrt();
        if len > 0.0001 {
            let speed = self.move_speed * dt / len;
            self.player.try_move(mx * speed, my * speed);
        }

        if self.player.shoot_cooldown > 0.0 {
            self.player.shoot_cooldown -= dt;
        }
        if self.player.hurt_flash > 0.0 {
            self.player.hurt_flash -= dt;
        }
        if window.is_key_down(Key::Space) && self.player.shoot_cooldown <= 0.0 && self.player.ammo > 0 {
            self.fire();
            self.player.shoot_cooldown = 0.35;
            self.player.ammo -= 1;
        }

        for enemy in &mut self.enemies {
            enemy.update(dt, &mut self.player);
        }

        for pickup in &mut self.pickups {
            if pickup.taken {
                continue;
            }
            let d = ((pickup.x - self.player.x).powi(2) + (pickup.y - self.player.y).powi(2)).sqrt();
            if d < 0.5 {
                pickup.taken = true;
                match pickup.kind {
                    PickupKind::Health => self.player.health = (self.player.health + 25).min(100),
                    PickupKind::Ammo => self.player.ammo += 20,
                }
            }
        }

        if map::is_exit(self.player.x.floor() as i32, self.player.y.floor() as i32) {
            self.won = true;
        }
        if self.player.health <= 0 {
            self.game_over = true;
        }
    }

    fn fire(&mut self) {
        let px = self.player.x;
        let py = self.player.y;
        let angle = self.player.dir_angle;
        let mut best: Option<(usize, f32)> = None;
        for (i, enemy) in self.enemies.iter().enumerate() {
            if !enemy.is_alive() {
                continue;
            }
            let ex = enemy.x - px;
            let ey = enemy.y - py;
            let dist = ex.hypot(ey);
            let target_angle = ey.atan2(ex);
            let diff = normalize_angle(target_angle - angle);
            if diff.abs() < 0.12 && has_line_of_sight(px, py, enemy.x, enemy.y) {
                if best.map_or(true, |(_, d)| dist < d) {
                    best = Some((i, dist));
                }
            }
        }
        if let Some((i, _)) = best {
            self.enemies[i].take_damage(25);
        }
    }

    pub fn render(&self, buf: &mut [u32], w: usize, h: usize) {
        let mut depth = vec![f32::MAX; w];
        raycaster::render_walls(buf, &mut depth, w, h, &self.player, &self.textures);
        self.render_sprites(buf, &depth, w, h);
        self.render_hud(buf, w, h);
    }

    fn render_sprites(&self, buf: &mut [u32], depth: &[f32], w: usize, h: usize) {
        struct Sprite {
            dist: f32,
            x: f32,
            y: f32,
            color: u32,
            size: f32,
        }

        let mut drawables: Vec<Sprite> = Vec::new();
        for e in &self.enemies {
            if e.is_alive() {
                drawables.push(Sprite { dist: 0.0, x: e.x, y: e.y, color: e.color(), size: 0.7 });
            }
        }
        for p in &self.pickups {
            if !p.taken {
                let color = match p.kind {
                    PickupKind::Health => pack(60, 220, 90),
                    PickupKind::Ammo => pack(220, 200, 60),
                };
                drawables.push(Sprite { dist: 0.0, x: p.x, y: p.y, color, size: 0.35 });
            }
        }

        for sprite in &mut drawables {
            sprite.dist = (sprite.x - self.player.x).hypot(sprite.y - self.player.y);
        }
        drawables.sort_by(|a, b| b.dist.partial_cmp(&a.dist).unwrap());

        let half_h = h as f32 / 2.0;
        for sprite in drawables {
            let dx = sprite.x - self.player.x;
            let dy = sprite.y - self.player.y;
            let angle_to = dy.atan2(dx);
            let rel = normalize_angle(angle_to - self.player.dir_angle);
            if rel.abs() > FOV / 2.0 + 0.3 || sprite.dist < 0.2 {
                continue;
            }
            let corrected = sprite.dist * rel.cos();
            let screen_x = (0.5 + rel / FOV) * w as f32;
            let sprite_h = (h as f32 / corrected) * sprite.size;
            let sprite_w = sprite_h;

            let x0 = (screen_x - sprite_w / 2.0).max(0.0) as i32;
            let x1 = (screen_x + sprite_w / 2.0).min(w as f32) as i32;
            let y0 = (half_h - sprite_h / 2.0).max(0.0) as i32;
            let y1 = (half_h + sprite_h / 2.0).min(h as f32) as i32;

            let fog = (1.0 - corrected / 20.0).clamp(0.2, 1.0);
            let shaded = crate::textures::shade(sprite.color, fog);

            for x in x0.max(0)..x1.min(w as i32) {
                if corrected >= depth[x as usize] {
                    continue;
                }
                for y in y0.max(0)..y1.min(h as i32) {
                    buf[y as usize * w + x as usize] = shaded;
                }
            }
        }
    }

    fn render_hud(&self, buf: &mut [u32], w: usize, h: usize) {
        // Hurt flash overlay.
        if self.player.hurt_flash > 0.0 {
            let alpha = (self.player.hurt_flash / 0.25).clamp(0.0, 1.0);
            for px in buf.iter_mut() {
                let r = ((*px >> 16) & 0xFF) as f32;
                let g = ((*px >> 8) & 0xFF) as f32;
                let b = (*px & 0xFF) as f32;
                let nr = r + (255.0 - r) * alpha * 0.4;
                *px = pack(nr as u8, (g * (1.0 - alpha * 0.4)) as u8, (b * (1.0 - alpha * 0.4)) as u8);
            }
        }

        // Bottom HUD bar.
        let bar_h = 28;
        for y in (h - bar_h)..h {
            for x in 0..w {
                buf[y * w + x] = pack(15, 15, 18);
            }
        }
        font::draw_text(buf, w, h, 10, (h - bar_h + 10) as i32, "HP", 2, pack(220, 60, 60));
        font::draw_text(
            buf,
            w,
            h,
            50,
            (h - bar_h + 10) as i32,
            &format!("{:03}", self.player.health.max(0)),
            2,
            pack(230, 230, 230),
        );
        font::draw_text(buf, w, h, 150, (h - bar_h + 10) as i32, "AM", 2, pack(220, 200, 60));
        font::draw_text(
            buf,
            w,
            h,
            190,
            (h - bar_h + 10) as i32,
            &format!("{:03}", self.player.ammo.max(0)),
            2,
            pack(230, 230, 230),
        );

        let alive = self.enemies.iter().filter(|e| e.is_alive()).count();
        font::draw_text(
            buf,
            w,
            h,
            290,
            (h - bar_h + 10) as i32,
            &format!("{:02}", alive),
            2,
            pack(200, 80, 80),
        );

        // Center crosshair.
        let cx = w as i32 / 2;
        let cy = h as i32 / 2;
        for i in -4..=4 {
            if cx + i >= 0 && (cx + i as i32) < w as i32 {
                buf[cy as usize * w + (cx + i) as usize] = pack(0, 255, 0);
            }
            if cy + i >= 0 && (cy + i as i32) < h as i32 {
                buf[(cy + i) as usize * w + cx as usize] = pack(0, 255, 0);
            }
        }

        if self.won {
            font::draw_text(buf, w, h, w as i32 / 2 - 90, h as i32 / 2 - 5, "LEVEL CLEAR", 4, pack(80, 220, 100));
        }
        if self.game_over {
            font::draw_text(buf, w, h, w as i32 / 2 - 60, h as i32 / 2 - 5, "YOU DIED", 4, pack(220, 40, 40));
        }
    }
}
