use crate::entities::enemy::{has_line_of_sight, Enemy, EnemyKind};
use crate::entities::player::Player;
use crate::input::InputEvent;
use crate::platform::{Platform, SoundKind};
use crate::rendering::font;
use crate::rendering::raycaster::{self, FOV};
use crate::rendering::textures::{pack, TextureSet};
use crate::weapons::{Explosion, PlayerArsenal, WeaponKind, WeaponState};
use crate::world::map;
use image::RgbaImage;
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

struct Sprite {
    dist: f32,
    x: f32,
    y: f32,
    color: u32,
    size: f32,
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
    pub arsenal: PlayerArsenal,
    pub explosions: Vec<Explosion>,
    grenades: Vec<GrenadeProjectile>,
    pub quit_requested: bool,
    hud_gun: HudSprite,
    hud_short_gun: HudSprite,
    // Zero-allocation reusable frame buffers (Rule 8)
    depth_buffer: Vec<f32>,
    drawables_buffer: Vec<Sprite>,
}

struct HudSprite {
    image: RgbaImage,
    columns: u32,
    rows: u32,
    dest_w: i32,
    dest_h: i32,
}

impl HudSprite {
    fn assault_rifle() -> Self {
        let image = image::load_from_memory(include_bytes!(
            "../gameasset/hud/weapons/first-person-assault-gun.png"
        ))
        .expect("invalid assault-gun HUD PNG")
        .to_rgba8();
        Self {
            image,
            columns: 4,
            rows: 3,
            dest_w: 430,
            dest_h: 295,
        }
    }

    fn pistol() -> Self {
        let image =
            image::load_from_memory(include_bytes!("../gameasset/hud/weapons/pistol-asset.png"))
                .expect("invalid pistol HUD PNG")
                .to_rgba8();
        Self {
            image,
            columns: 5,
            rows: 1,
            dest_w: 40 * 4,
            dest_h: 30 * 4,
        }
    }

    fn draw_frame(&self, buffer: &mut [u32], screen_w: usize, frame: u8, y_offset: f32) {
        let cell_w = self.image.width() / self.columns;
        let cell_h = self.image.height() / self.rows;
        let frame = (frame as u32).min(self.columns * self.rows - 1);
        let src_x = (frame % self.columns) * cell_w;
        let src_y = (frame / self.columns) * cell_h;
        let dest_w = self.dest_w;
        let dest_h = self.dest_h;
        let screen_h = (buffer.len() / screen_w) as i32;
        let dest_x = screen_w as i32 - dest_w;
        let dest_y = (screen_h - dest_h) + (y_offset * dest_h as f32) as i32;

        for dy in 0..dest_h {
            let sy = src_y + (dy as u32 * cell_h / dest_h as u32);
            for dx in 0..dest_w {
                let sx = src_x + (dx as u32 * cell_w / dest_w as u32);
                let pixel = self.image.get_pixel(
                    sx.min(self.image.width() - 1),
                    sy.min(self.image.height() - 1),
                );
                let alpha = pixel[3] as u32;
                let x = dest_x + dx;
                let y = dest_y + dy;
                if alpha == 0 || x < 0 || y < 0 {
                    continue;
                }
                let x = x as usize;
                let y = y as usize;
                let height = buffer.len() / screen_w;
                if x >= screen_w || y >= height {
                    continue;
                }
                let old = buffer[y * screen_w + x];
                let inv = 255 - alpha;
                let r = (pixel[0] as u32 * alpha + ((old >> 16) & 0xff) * inv) / 255;
                let g = (pixel[1] as u32 * alpha + ((old >> 8) & 0xff) * inv) / 255;
                let b = (pixel[2] as u32 * alpha + (old & 0xff) * inv) / 255;
                buffer[y * screen_w + x] = pack(r as u8, g as u8, b as u8);
            }
        }
    }
}

struct GrenadeProjectile {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    fuse: f32,
}

impl Game {
    pub fn new() -> Self {
        let sp = map::spawns();
        let mut pickups = Vec::new();
        for (x, y) in sp.health_pickups {
            pickups.push(Pickup {
                x,
                y,
                kind: PickupKind::Health,
                taken: false,
            });
        }
        for (x, y) in sp.ammo_pickups {
            pickups.push(Pickup {
                x,
                y,
                kind: PickupKind::Ammo,
                taken: false,
            });
        }
        Game {
            player: Player::new(sp.player_start.0, sp.player_start.1),
            enemies: sp
                .enemy_spawns
                .into_iter()
                .map(|(x, y, tier)| Enemy::new(x, y, EnemyKind::from_tier(tier)))
                .collect(),
            pickups,
            textures: TextureSet::generate(),
            won: false,
            game_over: false,
            move_speed: 3.0,
            turn_speed: 2.5,
            arsenal: PlayerArsenal::new(),
            explosions: Vec::with_capacity(32),
            grenades: Vec::with_capacity(16),
            quit_requested: false,
            hud_gun: HudSprite::assault_rifle(),
            hud_short_gun: HudSprite::pistol(),
            depth_buffer: vec![f32::MAX; 640],
            drawables_buffer: Vec::with_capacity(128),
        }
    }

    pub fn tick<P: Platform>(&mut self, platform: &mut P, events: &[InputEvent], dt: f32) {
        if self.won || self.game_over {
            return;
        }

        for &event in events {
            self.handle_event(platform, event, dt);
        }
        self.advance_world(dt);
    }

    pub fn handle_event<P: Platform>(&mut self, platform: &mut P, event: InputEvent, dt: f32) {
        if matches!(event, InputEvent::Quit) {
            self.quit_requested = true;
            return;
        }
        match event {
            InputEvent::TurnLeft => self.player.dir_angle -= self.turn_speed * dt,
            InputEvent::TurnRight => self.player.dir_angle += self.turn_speed * dt,
            InputEvent::MoveForward
            | InputEvent::MoveBackward
            | InputEvent::StrafeLeft
            | InputEvent::StrafeRight => {
                let (dx, dy) = self.player.dir();
                let (mut mx, mut my) = (0.0, 0.0);
                match event {
                    InputEvent::MoveForward => {
                        mx += dx;
                        my += dy;
                    }
                    InputEvent::MoveBackward => {
                        mx -= dx;
                        my -= dy;
                    }
                    InputEvent::StrafeLeft => {
                        mx += dy;
                        my -= dx;
                    }
                    InputEvent::StrafeRight => {
                        mx -= dy;
                        my += dx;
                    }
                    _ => unreachable!(),
                }
                self.player
                    .try_move(mx * self.move_speed * dt, my * self.move_speed * dt);
            }
            InputEvent::Fire
                if matches!(
                    self.arsenal.state,
                    WeaponState::Idle {
                        weapon: WeaponKind::Gun
                    }
                ) && self.player.ammo > 0
                    && self.player.shoot_cooldown <= 0.0 =>
            {
                self.fire(25);
                platform.play_sound(SoundKind::Shoot);
                self.player.ammo -= 1;
                self.player.shoot_cooldown = 0.25;
                self.arsenal.handle_input(event);
            }
            InputEvent::Fire
                if matches!(
                    self.arsenal.state,
                    WeaponState::Idle {
                        weapon: WeaponKind::ShortGun
                    }
                ) && self.player.ammo > 0
                    && self.player.shoot_cooldown <= 0.0 =>
            {
                self.fire(35);
                platform.play_sound(SoundKind::PistolShoot);
                self.player.ammo -= 1;
                self.player.shoot_cooldown = 0.35;
                self.arsenal.handle_input(event);
            }
            other => {
                if self.arsenal.handle_input(other) {
                    platform.play_sound(SoundKind::SwitchWeapon);
                }
            }
        }
    }

    fn advance_world(&mut self, dt: f32) {
        if let Some(_) = self.arsenal.update(dt) {
            let (dx, dy) = self.player.dir();
            self.grenades.push(GrenadeProjectile {
                x: self.player.x,
                y: self.player.y,
                dx,
                dy,
                fuse: 0.65,
            });
        }

        if self.player.shoot_cooldown > 0.0 {
            self.player.shoot_cooldown -= dt;
        }
        if self.player.hurt_flash > 0.0 {
            self.player.hurt_flash -= dt;
        }
        self.player.shoot_cooldown = self.player.shoot_cooldown.max(0.0);

        let mut i = 0;
        while i < self.grenades.len() {
            self.grenades[i].x += self.grenades[i].dx * 5.0 * dt;
            self.grenades[i].y += self.grenades[i].dy * 5.0 * dt;
            self.grenades[i].fuse -= dt;
            if self.grenades[i].fuse <= 0.0 {
                let gx = self.grenades[i].x;
                let gy = self.grenades[i].y;
                self.grenades.swap_remove(i);
                self.explosions.push(Explosion::new(gx, gy));
            } else {
                i += 1;
            }
        }

        let mut i = 0;
        while i < self.explosions.len() {
            self.explosions[i].update(dt);
            if self.explosions[i].remove {
                self.explosions.swap_remove(i);
            } else {
                i += 1;
            }
        }

        let player_pos = (self.player.x, self.player.y);
        for enemy in &mut self.enemies {
            if let Some(damage) = enemy.update(dt, player_pos) {
                self.player.damage(damage);
            }
        }

        for pickup in &mut self.pickups {
            if pickup.taken {
                continue;
            }
            let d = (pickup.x - self.player.x).hypot(pickup.y - self.player.y);
            if d < 0.5 {
                pickup.taken = true;
                match pickup.kind {
                    PickupKind::Health => self.player.health = (self.player.health + 25).min(100),
                    PickupKind::Ammo => {
                        self.player.ammo += 20;
                    }
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

    fn fire(&mut self, damage: i32) {
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
            self.enemies[i].take_damage(damage);
        }
    }

    pub fn render(&mut self, buf: &mut [u32], w: usize, h: usize) {
        if self.depth_buffer.len() != w {
            self.depth_buffer.resize(w, f32::MAX);
        }
        self.depth_buffer.fill(f32::MAX);

        raycaster::render_walls(buf, &mut self.depth_buffer, w, h, &self.player, &self.textures);
        self.render_sprites(buf, w, h);
        self.render_hud(buf, w, h);
    }

    fn render_sprites(&mut self, buf: &mut [u32], w: usize, h: usize) {
        self.drawables_buffer.clear();

        for e in &self.enemies {
            if e.is_alive() {
                self.drawables_buffer.push(Sprite {
                    dist: 0.0,
                    x: e.x,
                    y: e.y,
                    color: e.color(),
                    size: e.sprite_size(),
                });
            }
        }
        for p in &self.pickups {
            if !p.taken {
                let color = match p.kind {
                    PickupKind::Health => pack(60, 220, 90),
                    PickupKind::Ammo => pack(220, 200, 60),
                };
                self.drawables_buffer.push(Sprite {
                    dist: 0.0,
                    x: p.x,
                    y: p.y,
                    color,
                    size: 0.35,
                });
            }
        }
        for explosion in &self.explosions {
            let intensity = 255u8.saturating_sub(explosion.frame.saturating_mul(28));
            self.drawables_buffer.push(Sprite {
                dist: 0.0,
                x: explosion.x,
                y: explosion.y,
                color: pack(255, intensity, 30),
                size: 0.9 + explosion.frame as f32 * 0.12,
            });
        }

        for sprite in &mut self.drawables_buffer {
            sprite.dist = (sprite.x - self.player.x).hypot(sprite.y - self.player.y);
        }
        self.drawables_buffer.sort_by(|a, b| b.dist.partial_cmp(&a.dist).unwrap());

        let half_h = h as f32 / 2.0;
        for sprite in &self.drawables_buffer {
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
            let shaded = crate::rendering::textures::shade(sprite.color, fog);

            for x in x0.max(0)..x1.min(w as i32) {
                if corrected >= self.depth_buffer[x as usize] {
                    continue;
                }
                for y in y0.max(0)..y1.min(h as i32) {
                    buf[y as usize * w + x as usize] = shaded;
                }
            }
        }
    }

    fn render_hud(&self, buf: &mut [u32], w: usize, h: usize) {
        if self.player.hurt_flash > 0.0 {
            let alpha = (self.player.hurt_flash / 0.25).clamp(0.0, 1.0);
            for px in buf.iter_mut() {
                let r = ((*px >> 16) & 0xFF) as f32;
                let g = ((*px >> 8) & 0xFF) as f32;
                let b = (*px & 0xFF) as f32;
                let nr = r + (255.0 - r) * alpha * 0.4;
                *px = pack(
                    nr as u8,
                    (g * (1.0 - alpha * 0.4)) as u8,
                    (b * (1.0 - alpha * 0.4)) as u8,
                );
            }
        }

        if matches!(self.arsenal.equipped, WeaponKind::Gun) {
            let frame = match self.arsenal.state {
                WeaponState::Shooting { frame } => frame,
                WeaponState::PullingOut { frame, .. } => frame,
                _ => 0,
            };
            self.hud_gun
                .draw_frame(buf, w, frame, self.arsenal.y_offset());
        } else if matches!(self.arsenal.equipped, WeaponKind::ShortGun) {
            let frame = match self.arsenal.state {
                WeaponState::Shooting { frame } => {
                    if frame == 0 {
                        3
                    } else {
                        4
                    }
                }
                WeaponState::PullingOut { frame, .. } => frame.min(2),
                _ => 2,
            };
            self.hud_short_gun
                .draw_frame(buf, w, frame, self.arsenal.y_offset());
        }

        let bar_h = 28;
        for y in (h - bar_h)..h {
            for x in 0..w {
                buf[y * w + x] = pack(15, 15, 18);
            }
        }
        font::draw_text(
            buf,
            w,
            h,
            12,
            (h - bar_h + 10) as i32,
            "HP",
            2,
            pack(220, 60, 60),
        );
        font::draw_text(
            buf,
            w,
            h,
            42,
            (h - bar_h + 10) as i32,
            &format!("{:03}", self.player.health.max(0)),
            2,
            pack(230, 230, 230),
        );
        font::draw_text(
            buf,
            w,
            h,
            86,
            (h - bar_h + 10) as i32,
            "AM",
            2,
            pack(220, 200, 60),
        );
        font::draw_text(
            buf,
            w,
            h,
            116,
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
            160,
            (h - bar_h + 10) as i32,
            "EN",
            2,
            pack(200, 80, 80),
        );
        font::draw_text(
            buf,
            w,
            h,
            186,
            (h - bar_h + 10) as i32,
            &format!("{:02}", alive),
            2,
            pack(230, 230, 230),
        );
        font::draw_text(
            buf,
            w,
            h,
            224,
            (h - bar_h + 10) as i32,
            "LV",
            2,
            pack(100, 180, 220),
        );
        font::draw_text(
            buf,
            w,
            h,
            250,
            (h - bar_h + 10) as i32,
            &format!("{:02}", map::zone_tier(self.player.x, self.player.y)),
            2,
            pack(230, 230, 230),
        );

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
            font::draw_text(
                buf,
                w,
                h,
                w as i32 / 2 - 90,
                h as i32 / 2 - 5,
                "LEVEL CLEAR",
                4,
                pack(80, 220, 100),
            );
        }
        if self.game_over {
            font::draw_text(
                buf,
                w,
                h,
                w as i32 / 2 - 60,
                h as i32 / 2 - 5,
                "YOU DIED",
                4,
                pack(220, 40, 40),
            );
        }
    }
}
