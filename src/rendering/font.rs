// Tiny 3x5 bitmap font, digits and a few HUD glyphs only
// Each glyph is 5 rows of 3 bits (MSB unused), stored top-to-bottom

pub const GLYPH_W: usize = 3;

fn glyph(ch: char) -> [u8; 5] {
    match ch {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'E' => [0b111, 0b100, 0b111, 0b100, 0b111],
        'R' => [0b111, 0b101, 0b111, 0b110, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
        _ => [0b000, 0b000, 0b000, 0b000, 0b000],
    }
}

/// Draws text into an RGB framebuffer at (x, y) with the given pixel scale and color
pub fn draw_text(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    text: &str,
    scale: i32,
    color: u32,
) {
    let mut cx = x;
    for ch in text.chars() {
        let rows = glyph(ch);
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..GLYPH_W {
                if (bits >> (GLYPH_W - 1 - col)) & 1 == 1 {
                    let px0 = cx + col as i32 * scale;
                    let py0 = y + row as i32 * scale;
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = px0 + sx;
                            let py = py0 + sy;
                            if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                                buf[py as usize * w + px as usize] = color;
                            }
                        }
                    }
                }
            }
        }
        cx += (GLYPH_W as i32 + 1) * scale;
    }
}
