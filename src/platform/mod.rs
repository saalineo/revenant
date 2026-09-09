use crate::input::InputEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundKind {
    Shoot,
    PistolShoot,
    SwitchWeapon,
    Reload,
}

pub trait Platform {
    fn is_open(&self) -> bool;
    fn poll_events(&mut self) -> Vec<InputEvent>;
    fn present_frame(&mut self, buffer: &[u32], width: usize, height: usize);
    fn play_sound(&mut self, sound: SoundKind);
}

pub mod minifb_platform;
