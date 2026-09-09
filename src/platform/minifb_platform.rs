use super::{Platform, SoundKind};
use crate::audio::Audio;
use crate::input::InputEvent;
use minifb::{Key, KeyRepeat, MouseButton, Scale, ScaleMode, Window, WindowOptions};

pub struct MinifbPlatform {
    window: Window,
    audio: Audio,
}

impl MinifbPlatform {
    pub fn new(title: &str, width: usize, height: usize, target_fps: usize) -> Self {
        let window_opts = WindowOptions {
            resize: true,
            scale: Scale::FitScreen,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        };
        let mut window =
            Window::new(title, width, height, window_opts).expect("failed to create window");
        window.set_target_fps(target_fps);

        Self {
            window,
            audio: Audio::new(),
        }
    }
}

impl Platform for MinifbPlatform {
    fn is_open(&self) -> bool {
        self.window.is_open() && !self.window.is_key_down(Key::Escape)
    }

    fn poll_events(&mut self) -> Vec<InputEvent> {
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
            if self.window.is_key_down(key) {
                events.push(event);
            }
        }
        if self.window.is_key_down(Key::Space) || self.window.get_mouse_down(MouseButton::Left) {
            events.push(InputEvent::Fire);
        }
        for (key, slot) in [
            (Key::Key1, 1),
            (Key::Key2, 2),
            (Key::Key3, 3),
            (Key::Key4, 4),
        ] {
            if self.window.is_key_pressed(key, KeyRepeat::No) {
                events.push(InputEvent::EquipSlot(slot));
            }
        }
        if self.window.is_key_pressed(Key::Q, KeyRepeat::No) {
            events.push(InputEvent::SwitchWeapon);
        }
        for (key, event) in [
            (Key::V, InputEvent::QuickMelee),
            (Key::F, InputEvent::QuickMelee),
            (Key::G, InputEvent::QuickThrowGrenade),
        ] {
            if self.window.is_key_pressed(key, KeyRepeat::No) {
                events.push(event);
            }
        }
        if self.window.get_mouse_down(MouseButton::Right) {
            events.push(InputEvent::QuickMelee);
        }
        if let Some((_, scroll_y)) = self.window.get_scroll_wheel() {
            if scroll_y > 0.0 {
                events.push(InputEvent::ScrollUp);
            }
            if scroll_y < 0.0 {
                events.push(InputEvent::ScrollDown);
            }
        }
        events
    }

    fn present_frame(&mut self, buffer: &[u32], width: usize, height: usize) {
        self.window
            .update_with_buffer(buffer, width, height)
            .expect("failed to update window buffer");
    }

    fn play_sound(&mut self, sound: SoundKind) {
        match sound {
            SoundKind::Shoot => self.audio.play_shoot(),
            SoundKind::PistolShoot => self.audio.play_pistol_shoot(),
            SoundKind::SwitchWeapon => self.audio.play_switch_weapon(),
            SoundKind::Reload => self.audio.play_reload(),
        }
    }
}
