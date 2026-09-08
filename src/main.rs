mod entities;
mod game;
mod input;
mod rendering;
mod weapons;
mod world;

use game::Game;
use minifb::{Key, Scale, ScaleMode, Window, WindowOptions};
use std::time::Instant;

const WIDTH: usize = 640;
const HEIGHT: usize = 400;

fn main() {
    let window_opts = WindowOptions {
        resize: true,
        scale: Scale::FitScreen,
        scale_mode: ScaleMode::AspectRatioStretch,
        ..WindowOptions::default()
    };
    let mut window =
        Window::new("rustyDOOM", WIDTH, HEIGHT, window_opts).expect("failed to create window");
    window.set_target_fps(60);

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut game = Game::new();
    let mut last = Instant::now();
    let mut accumulator = 0.0f32;
    const TICRATE: f32 = 35.0;
    const TIC_DT: f32 = 1.0 / TICRATE;

    while window.is_open() && !window.is_key_down(Key::Escape) && !game.quit_requested {
        let now = Instant::now();
        let frame_time = (now - last).as_secs_f32().min(0.1);
        last = now;
        accumulator += frame_time;

        while accumulator >= TIC_DT {
            game.update(&window, TIC_DT);
            accumulator -= TIC_DT;
            if game.quit_requested {
                break;
            }
        }

        game.render(&mut buffer, WIDTH, HEIGHT);

        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .expect("failed to update window buffer");
    }
}
