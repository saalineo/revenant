mod audio;
mod entities;
mod game;
mod input;
mod platform;
mod rendering;
mod weapons;
mod world;

use game::Game;
use platform::minifb_platform::MinifbPlatform;
use platform::Platform;
use std::time::Instant;

const WIDTH: usize = 640;
const HEIGHT: usize = 400;
const TICRATE: f32 = 35.0;
const TIC_DT: f32 = 1.0 / TICRATE;

fn main() {
    let mut platform = MinifbPlatform::new("rustyDOOM", WIDTH, HEIGHT, 60);
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut game = Game::new();
    let mut last = Instant::now();
    let mut accumulator = 0.0f32;

    while platform.is_open() && !game.quit_requested {
        let now = Instant::now();
        let frame_time = (now - last).as_secs_f32().min(0.1);
        last = now;
        accumulator += frame_time;

        let events = platform.poll_events();

        while accumulator >= TIC_DT {
            game.tick(&mut platform, &events, TIC_DT);
            accumulator -= TIC_DT;
            if game.quit_requested {
                break;
            }
        }

        game.render(&mut buffer, WIDTH, HEIGHT);
        platform.present_frame(&buffer, WIDTH, HEIGHT);
    }
}
