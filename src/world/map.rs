//! Deterministic large dungeon layout built from connected room sectors.

pub const MAP_W: usize = 96;
pub const MAP_H: usize = 64;
const SECTOR_W: i32 = 8;
const SECTOR_H: i32 = 8;

fn hash(x: i32, y: i32) -> u32 {
    let mut n =
        (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ 0x9e37_79b9;
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    n ^ (n >> 16)
}

/// Returns a wall/material id, 0 for floor, or 9 for the final exit.
pub fn cell(x: i32, y: i32) -> u8 {
    if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
        return 1;
    }
    if x == MAP_W as i32 - 4 && y == MAP_H as i32 - 4 {
        return 9;
    }
    if x == 0 || y == 0 || x == MAP_W as i32 - 1 || y == MAP_H as i32 - 1 {
        return 1;
    }

    let sx = x / SECTOR_W;
    let sy = y / SECTOR_H;
    let lx = x % SECTOR_W;
    let ly = y % SECTOR_H;
    let room_material = (hash(sx, sy) % 5 + 1) as u8;

    // Every room has two-cell doorways to its neighbors, so all 96 sectors are reachable.
    if lx == 0 || lx == SECTOR_W - 1 {
        return if (ly == 3 || ly == 4) && sx > 0 && sx < MAP_W as i32 / SECTOR_W {
            0
        } else {
            room_material
        };
    }
    if ly == 0 || ly == SECTOR_H - 1 {
        return if (lx == 3 || lx == 4) && sy > 0 && sy < MAP_H as i32 / SECTOR_H {
            0
        } else {
            room_material
        };
    }

    // Coordinate-specific cover creates distinct rooms without sealing the routes.
    let variant = hash(sx * 7 + lx, sy * 11 + ly) % 11;
    let pillar = (lx == 2 || lx == 5) && (ly == 2 || ly == 5) && variant < 3;
    if pillar {
        room_material
    } else {
        0
    }
}

pub fn is_wall(x: i32, y: i32) -> bool {
    (1..=5).contains(&cell(x, y))
}
pub fn is_exit(x: i32, y: i32) -> bool {
    cell(x, y) == 9
}

/// Encounter progression band: 1 is the entrance, 3 is the deep dungeon.
pub fn zone_tier(x: f32, y: f32) -> u8 {
    let sx = (x as i32 / SECTOR_W).max(0);
    let sy = (y as i32 / SECTOR_H).max(0);
    if sy >= 5 || (sx >= 8 && sy >= 2) {
        3
    } else if sy >= 2 || sx >= 5 {
        2
    } else {
        1
    }
}

pub struct SpawnData {
    pub player_start: (f32, f32),
    /// Third field is the dungeon tier: 0 normal, 1 mid-level, 2 boss.
    pub enemy_spawns: Vec<(f32, f32, u8)>,
    pub health_pickups: Vec<(f32, f32)>,
    pub ammo_pickups: Vec<(f32, f32)>,
}

pub fn spawns() -> SpawnData {
    let mut enemy_spawns = Vec::new();
    let mut health_pickups = Vec::new();
    let mut ammo_pickups = Vec::new();

    for sy in 0..(MAP_H as i32 / SECTOR_H) {
        for sx in 0..(MAP_W as i32 / SECTOR_W) {
            if (sx, sy) == (0, 0)
                || (sx, sy) == (MAP_W as i32 / SECTOR_W - 1, MAP_H as i32 / SECTOR_H - 1)
            {
                continue;
            }
            let cx = sx * SECTOR_W + 4;
            let cy = sy * SECTOR_H + 4;
            let roll = hash(sx, sy) % 100;
            let tier = if sy >= 5 || (sx >= 8 && sy >= 2) {
                2
            } else if sy >= 2 || sx >= 5 {
                1
            } else {
                0
            };

            if roll < 70 {
                enemy_spawns.push((cx as f32 + 0.5, cy as f32 + 0.5, tier));
            }
            if tier >= 1 && roll % 3 == 0 {
                enemy_spawns.push(((cx - 1) as f32 + 0.5, cy as f32 + 0.5, tier));
            }
            if roll % 13 == 0 {
                health_pickups.push((cx as f32 + 0.5, (cy - 1) as f32 + 0.5));
            }
            if roll % 9 == 0 {
                ammo_pickups.push(((cx + 1) as f32 + 0.5, cy as f32 + 0.5));
            }
        }
    }

    SpawnData {
        player_start: (4.5, 4.5),
        enemy_spawns,
        health_pickups,
        ammo_pickups,
    }
}
