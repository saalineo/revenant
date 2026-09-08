use crate::entities::enemy::{has_line_of_sight, Enemy, EnemyKind};
use crate::entities::player::Player;
use crate::input::InputEvent;
use crate::rendering::font;
use crate::rendering::raycaster::{self, FOV};
use crate::rendering::textures::{pack, TextureSet};
use crate::weapons::{Explosion, PlayerArsenal, WeaponKind, WeaponState};
use crate::world::map;
use image::RgbaImage;
use minifb::{Key, KeyRepeat, MouseButton, Window};
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
    pub arsenal: PlayerArsenal,
    pub explosions: Vec<Explosion>,
    grenades: Vec<GrenadeProjectile>,
    pub quit_requested: bool,
    hud_gun: HudSprite,
}

struct HudSprite {
    image: RgbaImage,
    columns: u32,
    rows: u32,
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
        }
    }

    fn draw_frame(&self, buffer: &mut [u32], screen_w: usize, frame: u8, y_offset: f32) {
        let cell_w = self.image.width() / self.columns;
        let cell_h = self.image.height() / self.rows;
        let frame = (frame as u32).min(self.columns * self.rows - 1);
        let src_x = (frame % self.columns) * cell_w;
        let src_y = (frame / self.columns) * cell_h;
        let dest_w = 430i32;
        let dest_h = 295i32;
        let dest_x = (screen_w as i32 - dest_w) / 2;
        let dest_y = 78 + (y_offset * dest_h as f32) as i32;

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
            explosions: Vec::new(),
            grenades: Vec::new(),
            quit_requested: false,
            hud_gun: HudSprite::assault_rifle(),
        }
    }

    pub fn update(&mut self, window: &Window, dt: f32) {
        if self.won || self.game_over {
            return;
        }

        for event in Self::input_events(window) {
            self.update_event(event, dt);
        }
        self.advance_world(dt);
    }

    /// a script can send the same semantic events as the minifb adapter below.
    pub fn update_event(&mut self, event: InputEvent, dt: f32) {
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
                self.fire();
                self.player.ammo -= 1;
                self.player.shoot_cooldown = 0.35;
                self.arsenal.handle_input(event);
            }
            other => self.arsenal.handle_input(other),
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

        for grenade in &mut self.grenades {
            grenade.x += grenade.dx * 5.0 * dt;
            grenade.y += grenade.dy * 5.0 * dt;
            grenade.fuse -= dt;
        }
        let mut bursts = Vec::new();
        self.grenades.retain(|g| {
            if g.fuse <= 0.0 {
                bursts.push((g.x, g.y));
                false
            } else {
                true
            }
        });
        self.explosions
            .extend(bursts.into_iter().map(|(x, y)| Explosion::new(x, y)));
        for explosion in &mut self.explosions {
            explosion.update(dt);
        }
        self.explosions.retain(|e| !e.remove);

        for enemy in &mut self.enemies {
            enemy.update(dt, &mut self.player);
        }

        for pickup in &mut self.pickups {
            if pickup.taken {
                continue;
            }
            let d =
                ((pickup.x - self.player.x).powi(2) + (pickup.y - self.player.y).powi(2)).sqrt();
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

    fn input_events(window: &Window) -> Vec<InputEvent> {
        let mut events = Vec::new();
        let held = [
            (Key::W, InputEvent::MoveForward),
            (Key::S, InputEvent::MoveBackward),
            (Key::A, InputEvent::StrafeLeft),
            (Key::D, InputEvent::StrafeRight),
            (Key::Left, InputEvent::TurnLeft),
            (Key::Right, InputEvent::TurnRight),
        ];
        for (key, event) in held {
            if window.is_key_down(key) {
                events.push(event);
            }
        }
        if window.is_key_down(Key::Space) || window.get_mouse_down(MouseButton::Left) {
            events.push(InputEvent::Fire);
        }
        for (key, slot) in [(Key::Key1, 1), (Key::Key2, 2), (Key::Key3, 3)] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                events.push(InputEvent::EquipSlot(slot));
            }
        }
        for (key, event) in [
            (Key::V, InputEvent::QuickMelee),
            (Key::F, InputEvent::QuickMelee),
            (Key::G, InputEvent::QuickThrowGrenade),
        ] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                events.push(event);
            }
        }
        // minifb 0.28 exposes only three mouse buttons; Mouse4/5 should be
        // mapped by a richer platform adapter and emitted as QuickMelee here.
        if window.get_mouse_down(MouseButton::Right) {
            events.push(InputEvent::QuickMelee);
        }
        if let Some((_, scroll_y)) = window.get_scroll_wheel() {
            if scroll_y > 0.0 {
                events.push(InputEvent::ScrollUp);
            }
            if scroll_y < 0.0 {
                events.push(InputEvent::ScrollDown);
            }
        }
        events
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
                drawables.push(Sprite {
                    dist: 0.0,
                    x: e.x,
                    y: e.y,
                    color: e.color(),
                    size: e.sprite_size(),
                });
                // The software renderer currently uses color silhouettes. This
                // is the asset binding point for loading the matching PNG atlas.
                let _asset_path = e.kind.asset_path();
            }
        }
        for p in &self.pickups {
            if !p.taken {
                let color = match p.kind {
                    PickupKind::Health => pack(60, 220, 90),
                    PickupKind::Ammo => pack(220, 200, 60),
                };
                drawables.push(Sprite {
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
            drawables.push(Sprite {
                dist: 0.0,
                x: explosion.x,
                y: explosion.y,
                color: pack(255, intensity, 30),
                size: 0.9 + explosion.frame as f32 * 0.12,
            });
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
            let shaded = crate::rendering::textures::shade(sprite.color, fog);

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
                *px = pack(
                    nr as u8,
                    (g * (1.0 - alpha * 0.4)) as u8,
                    (b * (1.0 - alpha * 0.4)) as u8,
                );
            }
        }

        // Bottom HUD bar.
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
            10,
            (h - bar_h + 10) as i32,
            "HP",
            2,
            pack(220, 60, 60),
        );
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
        font::draw_text(
            buf,
            w,
            h,
            150,
            (h - bar_h + 10) as i32,
            "AM",
            2,
            pack(220, 200, 60),
        );
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
        font::draw_text(
            buf,
            w,
            h,
            370,
            (h - bar_h + 10) as i32,
            "LV",
            2,
            pack(100, 180, 220),
        );
        font::draw_text(
            buf,
            w,
            h,
            410,
            (h - bar_h + 10) as i32,
            &format!("{:02}", map::zone_tier(self.player.x, self.player.y)),
            2,
            pack(230, 230, 230),
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

        // Draw the actual first-person weapon over the 3D scene. Knife and
        // grenade atlases can use the same binding point when added.
        if matches!(self.arsenal.equipped, WeaponKind::Gun) {
            let frame = match self.arsenal.state {
                WeaponState::Shooting { frame } => frame,
                WeaponState::PullingOut { frame, .. } => frame,
                _ => 0,
            };
            self.hud_gun
                .draw_frame(buf, w, frame, self.arsenal.y_offset());
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
