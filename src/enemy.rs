use crate::map;
use crate::player::Player;
use crate::textures::pack;

#[derive(PartialEq, Clone, Copy)]
pub enum EnemyState {
    Idle,
    Chasing,
    Attacking,
    Dead,
}

pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub health: i32,
    pub state: EnemyState,
    pub attack_cooldown: f32,
    pub hit_flash: f32,
}

const DETECT_RADIUS: f32 = 8.0;
const ATTACK_RADIUS: f32 = 1.2;
const SPEED: f32 = 1.4;

impl Enemy {
    pub fn new(x: f32, y: f32) -> Self {
        Enemy {
            x,
            y,
            health: 60,
            state: EnemyState::Idle,
            attack_cooldown: 0.0,
            hit_flash: 0.0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.state != EnemyState::Dead
    }

    pub fn take_damage(&mut self, amount: i32) {
        if !self.is_alive() {
            return;
        }
        self.health -= amount;
        self.hit_flash = 0.15;
        if self.health <= 0 {
            self.state = EnemyState::Dead;
        } else {
            self.state = EnemyState::Chasing;
        }
    }

    pub fn update(&mut self, dt: f32, player: &mut Player) {
        if self.hit_flash > 0.0 {
            self.hit_flash -= dt;
        }
        if !self.is_alive() {
            return;
        }

        let dx = player.x - self.x;
        let dy = player.y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();

        match self.state {
            EnemyState::Idle => {
                if dist < DETECT_RADIUS && has_line_of_sight(self.x, self.y, player.x, player.y) {
                    self.state = EnemyState::Chasing;
                }
            }
            EnemyState::Chasing => {
                if dist < ATTACK_RADIUS {
                    self.state = EnemyState::Attacking;
                } else if dist < DETECT_RADIUS * 1.5 {
                    let step = SPEED * dt / dist.max(0.001);
                    let nx = self.x + dx * step;
                    let ny = self.y + dy * step;
                    if !map::is_wall(nx.floor() as i32, self.y.floor() as i32) {
                        self.x = nx;
                    }
                    if !map::is_wall(self.x.floor() as i32, ny.floor() as i32) {
                        self.y = ny;
                    }
                } else {
                    self.state = EnemyState::Idle;
                }
            }
            EnemyState::Attacking => {
                if dist > ATTACK_RADIUS * 1.3 {
                    self.state = EnemyState::Chasing;
                } else {
                    self.attack_cooldown -= dt;
                    if self.attack_cooldown <= 0.0 {
                        player.damage(8);
                        self.attack_cooldown = 1.0;
                    }
                }
            }
            EnemyState::Dead => {}
        }
    }

    pub fn color(&self) -> u32 {
        if self.hit_flash > 0.0 {
            pack(255, 255, 255)
        } else {
            match self.state {
                EnemyState::Dead => pack(80, 20, 20),
                _ => pack(180, 40, 40),
            }
        }
    }
}

/// Coarse line-of-sight check by sampling points along the segment.
pub fn has_line_of_sight(x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
    let steps = 32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = x0 + (x1 - x0) * t;
        let y = y0 + (y1 - y0) * t;
        if map::is_wall(x.floor() as i32, y.floor() as i32) {
            return false;
        }
    }
    true
}
