//! HUD weapon animation state and world grenade burst animation.

use crate::input::InputEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    Gun,
    Knife,
    Grenade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponState {
    PullingOut { weapon: WeaponKind, frame: u8 },
    Idle { weapon: WeaponKind },
    Shooting { frame: u8 },
    Swinging { frame: u8 },
    Throwing { frame: u8, return_to: WeaponKind },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrenadeSpawn {
    pub return_to: WeaponKind,
}

pub struct PlayerArsenal {
    pub equipped: WeaponKind,
    pub state: WeaponState,
    frame_timer: f32,
}

impl PlayerArsenal {
    pub fn new() -> Self {
        Self {
            equipped: WeaponKind::Gun,
            state: WeaponState::PullingOut {
                weapon: WeaponKind::Gun,
                frame: 0,
            },
            frame_timer: 0.0,
        }
    }

    pub fn handle_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::EquipSlot(slot) if (1..=3).contains(&slot) => {
                let weapon = match slot {
                    1 => WeaponKind::Gun,
                    2 => WeaponKind::Knife,
                    3 => WeaponKind::Grenade,
                    _ => unreachable!(),
                };
                self.select(weapon);
            }
            InputEvent::ScrollUp => self.cycle(1),
            InputEvent::ScrollDown => self.cycle(-1),
            InputEvent::QuickMelee => self.start_swing(),
            InputEvent::QuickThrowGrenade => self.start_throw(self.equipped),
            InputEvent::Fire => match self.state {
                WeaponState::Idle {
                    weapon: WeaponKind::Gun,
                } => self.start_shoot(),
                WeaponState::Idle {
                    weapon: WeaponKind::Knife,
                } => self.start_swing(),
                WeaponState::Idle {
                    weapon: WeaponKind::Grenade,
                } => self.start_throw(WeaponKind::Grenade),
                _ => {}
            },
            _ => {}
        }
    }

    fn select(&mut self, weapon: WeaponKind) {
        if self.equipped != weapon || !matches!(self.state, WeaponState::Idle { .. }) {
            self.equipped = weapon;
            self.state = WeaponState::PullingOut { weapon, frame: 0 };
            self.frame_timer = 0.0;
        }
    }

    fn cycle(&mut self, direction: i8) {
        let all = [WeaponKind::Gun, WeaponKind::Knife, WeaponKind::Grenade];
        let current = all.iter().position(|w| *w == self.equipped).unwrap_or(0) as i8;
        let next = (current + direction).rem_euclid(all.len() as i8) as usize;
        self.select(all[next]);
    }

    fn start_shoot(&mut self) {
        self.state = WeaponState::Shooting { frame: 0 };
        self.frame_timer = 0.0;
    }
    fn start_swing(&mut self) {
        self.state = WeaponState::Swinging { frame: 0 };
        self.frame_timer = 0.0;
    }
    fn start_throw(&mut self, return_to: WeaponKind) {
        self.state = WeaponState::Throwing {
            frame: 0,
            return_to,
        };
        self.frame_timer = 0.0;
    }

    /// Advances animation frames and reports a grenade spawn exactly once.
    pub fn update(&mut self, mut dt: f32) -> Option<GrenadeSpawn> {
        let mut spawned = None;
        while dt > 0.0 {
            let duration = self.frame_duration();
            let remaining = duration - self.frame_timer;
            let step = dt.min(remaining);
            self.frame_timer += step;
            dt -= step;
            if self.frame_timer + f32::EPSILON < duration {
                break;
            }
            self.frame_timer = 0.0;
            match self.state {
                WeaponState::PullingOut { weapon, frame } if frame < 3 => {
                    self.state = WeaponState::PullingOut {
                        weapon,
                        frame: frame + 1,
                    }
                }
                WeaponState::PullingOut { weapon, .. } => self.state = WeaponState::Idle { weapon },
                WeaponState::Shooting { frame } if frame < 2 => {
                    self.state = WeaponState::Shooting { frame: frame + 1 }
                }
                WeaponState::Shooting { .. } => {
                    self.state = WeaponState::Idle {
                        weapon: WeaponKind::Gun,
                    }
                }
                WeaponState::Swinging { frame } if frame < 3 => {
                    self.state = WeaponState::Swinging { frame: frame + 1 }
                }
                WeaponState::Swinging { .. } => {
                    self.state = WeaponState::Idle {
                        weapon: WeaponKind::Knife,
                    }
                }
                WeaponState::Throwing { frame, return_to } if frame < 3 => {
                    self.state = WeaponState::Throwing {
                        frame: frame + 1,
                        return_to,
                    }
                }
                WeaponState::Throwing { return_to, .. } => {
                    spawned = Some(GrenadeSpawn { return_to });
                    self.equipped = return_to;
                    self.state = WeaponState::PullingOut {
                        weapon: return_to,
                        frame: 0,
                    };
                }
                WeaponState::Idle { .. } => break,
            }
        }
        spawned
    }

    fn frame_duration(&self) -> f32 {
        match self.state {
            WeaponState::PullingOut { .. } => 0.07,
            WeaponState::Shooting { .. } => 0.06,
            WeaponState::Swinging { .. } => 0.075,
            WeaponState::Throwing { .. } => 0.10,
            WeaponState::Idle { .. } => f32::MAX,
        }
    }

    /// Normalized HUD lift: 1.0 is below the screen, 0.0 is fully raised.
    pub fn y_offset(&self) -> f32 {
        match self.state {
            WeaponState::PullingOut { frame, .. } => 1.0 - (frame as f32 / 4.0),
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Explosion {
    pub x: f32,
    pub y: f32,
    pub frame: u8,
    timer: f32,
    pub remove: bool,
}

impl Explosion {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            frame: 0,
            timer: 0.0,
            remove: false,
        }
    }

    pub fn update(&mut self, mut dt: f32) {
        while dt > 0.0 && !self.remove {
            let step = dt.min(0.08 - self.timer);
            self.timer += step;
            dt -= step;
            if self.timer < 0.08 {
                break;
            }
            self.timer = 0.0;
            if self.frame >= 5 {
                self.remove = true;
            } else {
                self.frame += 1;
            }
        }
    }
}
