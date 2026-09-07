mod enemy;
mod font;
mod game;
mod map;
mod player;
mod raycaster;
mod textures;

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
    let mut window = Window::new("rustyDOOM", WIDTH, HEIGHT, window_opts)
        .expect("failed to create window");
    window.set_target_fps(60);

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut game = Game::new();
    let mut last = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;

        game.update(&window, dt);
        game.render(&mut buffer, WIDTH, HEIGHT);

        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .expect("failed to update window buffer");
    }
}
