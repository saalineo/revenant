// Procedural fallback wall textures. Actor/projectile art is catalogued under
// src/gameasset and can replace these silhouettes as the bitmap loader lands.

pub const TEX_SIZE: usize = 64;

pub struct Texture {
    pub pixels: Vec<u32>, // TEX_SIZE * TEX_SIZE, 0xRRGGBB
}

impl Texture {
    pub fn get(&self, x: usize, y: usize) -> u32 {
        self.pixels[(y % TEX_SIZE) * TEX_SIZE + (x % TEX_SIZE)]
    }
}

fn solid(mut f: impl FnMut(usize, usize) -> u32) -> Texture {
    let mut pixels = vec![0u32; TEX_SIZE * TEX_SIZE];
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            pixels[y * TEX_SIZE + x] = f(x, y);
        }
    }
    Texture { pixels }
}

/// Rough brick pattern.
pub fn brick_texture(base: (u8, u8, u8), mortar: (u8, u8, u8)) -> Texture {
    solid(|x, y| {
        let brick_h = 8;
        let brick_w = 16;
        let row = y / brick_h;
        let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
        let bx = (x + offset) % brick_w;
        let by = y % brick_h;
        let is_mortar = bx == 0 || by == 0;
        let (r, g, b) = if is_mortar { mortar } else { base };
        let shade = 1.0 - ((x as f32 / TEX_SIZE as f32) * 0.15);
        pack(
            (r as f32 * shade) as u8,
            (g as f32 * shade) as u8,
            (b as f32 * shade) as u8,
        )
    })
}

/// Tech panel with rivets.
pub fn panel_texture(base: (u8, u8, u8), accent: (u8, u8, u8)) -> Texture {
    solid(|x, y| {
        let border = x < 3 || y < 3 || x > TEX_SIZE - 4 || y > TEX_SIZE - 4;
        let rivet = (x % 16 < 3) && (y % 16 < 3);
        let (r, g, b) = if border || rivet { accent } else { base };
        pack(r, g, b)
    })
}

/// Hazard stripes.
pub fn hazard_texture(a: (u8, u8, u8), b: (u8, u8, u8)) -> Texture {
    solid(|x, y| {
        let stripe = ((x + y) / 8) % 2 == 0;
        let (r, g, bl) = if stripe { a } else { b };
        pack(r, g, bl)
    })
}

/// Cracked stone.
pub fn stone_texture(base: (u8, u8, u8)) -> Texture {
    solid(|x, y| {
        let mut hash = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263);
        hash = (hash ^ (hash >> 13)).wrapping_mul(1274126177);
        let noise = (hash % 40) as i32 - 20;
        let r = (base.0 as i32 + noise).clamp(0, 255) as u8;
        let g = (base.1 as i32 + noise).clamp(0, 255) as u8;
        let bl = (base.2 as i32 + noise).clamp(0, 255) as u8;
        pack(r, g, bl)
    })
}

pub fn pack(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn shade(color: u32, factor: f32) -> u32 {
    let r = ((color >> 16) & 0xFF) as f32 * factor;
    let g = ((color >> 8) & 0xFF) as f32 * factor;
    let b = (color & 0xFF) as f32 * factor;
    pack(
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    )
}

pub struct TextureSet {
    pub walls: Vec<Texture>,
}

impl TextureSet {
    pub fn generate() -> Self {
        TextureSet {
            walls: vec![
                brick_texture((150, 60, 50), (60, 60, 60)), // 1: red brick
                panel_texture((90, 95, 100), (200, 170, 40)), // 2: tech panel
                hazard_texture((200, 170, 20), (30, 30, 30)), // 3: hazard stripe
                stone_texture((100, 100, 110)),             // 4: cracked stone
                panel_texture((60, 70, 90), (40, 200, 200)), // 5: blue panel (exit)
            ],
        }
    }
}
