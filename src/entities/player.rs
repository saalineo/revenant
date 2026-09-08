use crate::world::map;

pub struct Player {
    pub x: f32,
    pub y: f32,
    pub dir_angle: f32, // radians
    pub health: i32,
    pub ammo: i32,
    pub radius: f32,
    pub shoot_cooldown: f32,
    pub hurt_flash: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Player {
            x,
            y,
            dir_angle: 0.0,
            health: 100,
            ammo: 50,
            radius: 0.25,
            shoot_cooldown: 0.0,
            hurt_flash: 0.0,
        }
    }

    pub fn dir(&self) -> (f32, f32) {
        (self.dir_angle.cos(), self.dir_angle.sin())
    }

    pub fn try_move(&mut self, dx: f32, dy: f32) {
        let nx = self.x + dx;
        let ny = self.y + dy;
        if !self.collides(nx, self.y) {
            self.x = nx;
        }
        if !self.collides(self.x, ny) {
            self.y = ny;
        }
    }

    fn collides(&self, x: f32, y: f32) -> bool {
        let r = self.radius;
        for (ox, oy) in [(-r, -r), (r, -r), (-r, r), (r, r)] {
            if map::is_wall((x + ox).floor() as i32, (y + oy).floor() as i32) {
                return true;
            }
        }
        false
    }

    pub fn damage(&mut self, amount: i32) {
        self.health -= amount;
        self.hurt_flash = 0.25;
    }
}
