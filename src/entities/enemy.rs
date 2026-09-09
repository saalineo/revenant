use crate::rendering::textures::pack;
use crate::world::map;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum StateId {
    Idle,
    Chasing,
    Attacking,
    Dead,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EnemyKind {
    Normal,
    MidLevel,
    Boss,
}

impl EnemyKind {
    pub fn from_tier(tier: u8) -> Self {
        match tier {
            0 => Self::Normal,
            1 => Self::MidLevel,
            _ => Self::Boss,
        }
    }

    pub fn max_health(self) -> i32 {
        match self {
            Self::Normal => 45,
            Self::MidLevel => 90,
            Self::Boss => 220,
        }
    }

    pub fn move_speed(self) -> f32 {
        match self {
            Self::Normal => 1.35,
            Self::MidLevel => 1.7,
            Self::Boss => 0.9,
        }
    }

    pub fn attack_damage(self) -> i32 {
        match self {
            Self::Normal => 6,
            Self::MidLevel => 10,
            Self::Boss => 18,
        }
    }

    pub fn attack_interval(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::MidLevel => 0.8,
            Self::Boss => 1.4,
        }
    }

    pub fn sprite_size(self) -> f32 {
        match self {
            Self::Normal => 0.65,
            Self::MidLevel => 0.85,
            Self::Boss => 1.25,
        }
    }

    pub fn asset_path(self) -> &'static str {
        match self {
            Self::Normal => "src/gameasset/actors/normal/orc-idle.png",
            Self::MidLevel => "src/gameasset/actors/mid_level/walk - sword.png",
            Self::Boss => "src/gameasset/actors/boss/andromalius-57x88.png",
        }
    }
}

pub type ActionFn = fn(enemy: &mut Enemy, player_pos: (f32, f32), dt: f32) -> Option<i32>;

pub struct State {
    pub id: StateId,
    pub action: ActionFn,
}

const DETECT_RADIUS: f32 = 8.0;
const ATTACK_RADIUS: f32 = 1.2;

fn idle_action(enemy: &mut Enemy, (px, py): (f32, f32), _dt: f32) -> Option<i32> {
    let dx = px - enemy.x;
    let dy = py - enemy.y;
    let dist = dx.hypot(dy);
    if dist < DETECT_RADIUS && has_line_of_sight(enemy.x, enemy.y, px, py) {
        enemy.state = StateId::Chasing;
    }
    None
}

fn chasing_action(enemy: &mut Enemy, (px, py): (f32, f32), dt: f32) -> Option<i32> {
    let dx = px - enemy.x;
    let dy = py - enemy.y;
    let dist = dx.hypot(dy);

    if dist < ATTACK_RADIUS {
        enemy.state = StateId::Attacking;
    } else if dist < DETECT_RADIUS * 1.5 {
        let speed = enemy.kind.move_speed();
        let step = speed * dt / dist.max(0.001);
        let nx = enemy.x + dx * step;
        let ny = enemy.y + dy * step;
        if !map::is_wall(nx.floor() as i32, enemy.y.floor() as i32) {
            enemy.x = nx;
        }
        if !map::is_wall(enemy.x.floor() as i32, ny.floor() as i32) {
            enemy.y = ny;
        }
    } else {
        enemy.state = StateId::Idle;
    }
    None
}

fn attacking_action(enemy: &mut Enemy, (px, py): (f32, f32), dt: f32) -> Option<i32> {
    let dx = px - enemy.x;
    let dy = py - enemy.y;
    let dist = dx.hypot(dy);

    if dist > ATTACK_RADIUS * 1.3 {
        enemy.state = StateId::Chasing;
        None
    } else {
        enemy.attack_cooldown -= dt;
        if enemy.attack_cooldown <= 0.0 {
            enemy.attack_cooldown = enemy.kind.attack_interval();
            Some(enemy.kind.attack_damage())
        } else {
            None
        }
    }
}

fn dead_action(_enemy: &mut Enemy, _player_pos: (f32, f32), _dt: f32) -> Option<i32> {
    None
}

pub static STATE_TABLE: [State; 4] = [
    State {
        id: StateId::Idle,
        action: idle_action,
    },
    State {
        id: StateId::Chasing,
        action: chasing_action,
    },
    State {
        id: StateId::Attacking,
        action: attacking_action,
    },
    State {
        id: StateId::Dead,
        action: dead_action,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub x: f32,
    pub y: f32,
    pub kind: EnemyKind,
    pub health: i32,
    pub state: StateId,
    pub attack_cooldown: f32,
    pub hit_flash: f32,
}

impl Enemy {
    pub fn new(x: f32, y: f32, kind: EnemyKind) -> Self {
        Enemy {
            x,
            y,
            kind,
            health: kind.max_health(),
            state: StateId::Idle,
            attack_cooldown: 0.0,
            hit_flash: 0.0,
        }
    }

    #[inline(always)]
    pub fn is_alive(&self) -> bool {
        self.state != StateId::Dead
    }

    pub fn take_damage(&mut self, amount: i32) {
        if !self.is_alive() {
            return;
        }
        self.health -= amount;
        self.hit_flash = 0.15;
        if self.health <= 0 {
            self.state = StateId::Dead;
        } else {
            self.state = StateId::Chasing;
        }
    }

    pub fn update(&mut self, dt: f32, player_pos: (f32, f32)) -> Option<i32> {
        if self.hit_flash > 0.0 {
            self.hit_flash -= dt;
        }
        if !self.is_alive() {
            return None;
        }

        let state_idx = match self.state {
            StateId::Idle => 0,
            StateId::Chasing => 1,
            StateId::Attacking => 2,
            StateId::Dead => 3,
        };

        (STATE_TABLE[state_idx].action)(self, player_pos, dt)
    }

    pub fn color(&self) -> u32 {
        if self.hit_flash > 0.0 {
            pack(255, 255, 255)
        } else {
            match (self.state, self.kind) {
                (StateId::Dead, _) => pack(80, 20, 20),
                (_, EnemyKind::Normal) => pack(180, 40, 40),
                (_, EnemyKind::MidLevel) => pack(180, 80, 210),
                (_, EnemyKind::Boss) => pack(40, 180, 150),
            }
        }
    }

    #[inline(always)]
    pub fn sprite_size(&self) -> f32 {
        self.kind.sprite_size()
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
