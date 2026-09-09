use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;

pub struct Audio {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    shoot_data: &'static [u8],
    pistol_data: &'static [u8],
    reload_data: &'static [u8],
}

impl Audio {
    pub fn new() -> Self {
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("Warning: failed to initialize audio device: {e}");
                (None, None)
            }
        };

        let shoot_data = include_bytes!("gameasset/sounds/gun-shhot-sound.mp3");
        let pistol_data = include_bytes!("gameasset/sounds/pistol-sound.mp3");
        let reload_data = include_bytes!("gameasset/sounds/gun-loading.mp3");

        Self {
            _stream: stream,
            stream_handle,
            shoot_data,
            pistol_data,
            reload_data,
        }
    }

    pub fn play_shoot(&self) {
        self.play_sound(self.shoot_data);
    }

    pub fn play_pistol_shoot(&self) {
        self.play_sound(self.pistol_data);
    }

    pub fn play_switch_weapon(&self) {
        self.play_sound(self.reload_data);
    }

    pub fn play_reload(&self) {
        self.play_sound(self.reload_data);
    }

    fn play_sound(&self, data: &'static [u8]) {
        if let Some(handle) = &self.stream_handle {
            let cursor = Cursor::new(data);
            if let Ok(source) = Decoder::new(cursor) {
                if let Ok(sink) = Sink::try_new(handle) {
                    sink.append(source);
                    sink.detach();
                }
            }
        }
    }
}
