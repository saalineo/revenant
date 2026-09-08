use crate::entities::player::Player;
use crate::rendering::textures::{shade, TextureSet, TEX_SIZE};
use crate::world::map;

pub const FOV: f32 = std::f32::consts::PI / 3.0; // 60 degrees

pub struct Hit {
    pub dist: f32,
    pub tex_id: u8,
    pub tex_x: usize,
    pub side: bool, // true = N/S wall, false = E/W wall
}

/// Digital differential analysis raycast, classic grid-based ray march.
pub fn cast_ray(px: f32, py: f32, angle: f32) -> Hit {
    let dir_x = angle.cos();
    let dir_y = angle.sin();

    let mut map_x = px.floor() as i32;
    let mut map_y = py.floor() as i32;

    let delta_dist_x = if dir_x.abs() < 1e-6 {
        1e30
    } else {
        (1.0 / dir_x).abs()
    };
    let delta_dist_y = if dir_y.abs() < 1e-6 {
        1e30
    } else {
        (1.0 / dir_y).abs()
    };

    let (step_x, mut side_dist_x) = if dir_x < 0.0 {
        (-1, (px - map_x as f32) * delta_dist_x)
    } else {
        (1, (map_x as f32 + 1.0 - px) * delta_dist_x)
    };
    let (step_y, mut side_dist_y) = if dir_y < 0.0 {
        (-1, (py - map_y as f32) * delta_dist_y)
    } else {
        (1, (map_y as f32 + 1.0 - py) * delta_dist_y)
    };

    let mut side; // false = vertical wall hit (x-side), true = horizontal (y-side)
    let mut tex_id;
    loop {
        if side_dist_x < side_dist_y {
            side_dist_x += delta_dist_x;
            map_x += step_x;
            side = false;
        } else {
            side_dist_y += delta_dist_y;
            map_y += step_y;
            side = true;
        }
        tex_id = map::cell(map_x, map_y);
        if tex_id >= 1 && tex_id <= 5 {
            break;
        }
        if tex_id == 9 {
            // Exit tile is walkable, not a wall
            continue;
        }
    }

    let perp_dist = if !side {
        (map_x as f32 - px + (1 - step_x) as f32 / 2.0) / dir_x
    } else {
        (map_y as f32 - py + (1 - step_y) as f32 / 2.0) / dir_y
    };
    let perp_dist = perp_dist.max(0.0001);

    let wall_hit_x = if !side {
        py + perp_dist * dir_y
    } else {
        px + perp_dist * dir_x
    };
    let wall_frac = wall_hit_x - wall_hit_x.floor();
    let mut tex_x = (wall_frac * TEX_SIZE as f32) as usize;
    if (!side && dir_x > 0.0) || (side && dir_y < 0.0) {
        tex_x = TEX_SIZE - 1 - tex_x;
    }

    Hit {
        dist: perp_dist,
        tex_id,
        tex_x,
        side,
    }
}

/// Renders the walls for one frame into `buf` (w*h RGB u32) and fills `depth`
/// (len w) with the perpendicular distance per column for sprite occlusion.
pub fn render_walls(
    buf: &mut [u32],
    depth: &mut [f32],
    w: usize,
    h: usize,
    player: &Player,
    textures: &TextureSet,
) {
    let half_h = h as f32 / 2.0;
    for x in 0..w {
        let camera_x = 2.0 * x as f32 / w as f32 - 1.0;
        let ray_angle = player.dir_angle + camera_x * (FOV / 2.0);
        let hit = cast_ray(player.x, player.y, ray_angle);

        // Correct fisheye distortion.
        let corrected = hit.dist * (ray_angle - player.dir_angle).cos();
        depth[x] = corrected;

        let line_h = (h as f32 / corrected).min(4000.0);
        let draw_start = (half_h - line_h / 2.0).max(0.0) as usize;
        let draw_end = (half_h + line_h / 2.0).min(h as f32 - 1.0) as usize;

        let tex = &textures.walls[(hit.tex_id.max(1) - 1) as usize];
        let base_shade = if hit.side { 0.7 } else { 1.0 };
        let fog = (1.0 - corrected / 20.0).clamp(0.25, 1.0);
        let final_shade = base_shade * fog;

        for y in draw_start..=draw_end.max(draw_start) {
            if y >= h {
                break;
            }
            let d = y as f32 - (half_h - line_h / 2.0);
            let tex_y = ((d / line_h.max(1.0)) * TEX_SIZE as f32) as usize;
            let color = tex.get(hit.tex_x, tex_y.min(TEX_SIZE - 1));
            buf[y * w + x] = shade(color, final_shade);
        }

        // Floor and ceiling: flat shaded, distance fogged.
        let floor_color = shade(0x2a2a2a, fog.max(0.15));
        let ceil_color = shade(0x151515, fog.max(0.15));
        for y in (draw_end + 1)..h {
            buf[y * w + x] = floor_color;
        }
        for y in 0..draw_start {
            buf[y * w + x] = ceil_color;
        }
    }
}
