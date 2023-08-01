use serde::{Serialize, Deserialize};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGTH: u32 = 800;
const BLOCK_SIZE: u32 = 5;

fn calculate_object_right(object: &Block) -> i32 {
    (BLOCK_SIZE * object.rect.w / BLOCK_SIZE) as i32
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    tick: u64,
    clients: Vec<Block>,
    is_started: bool,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rect {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: u32, h:u32) -> Self {
        Self { x: x, y: y, w: w, h: h }
    }
}

pub trait Ball {
    fn handle_wall(&mut self);
    fn react_object(&mut self, object: &Block) -> bool;
    fn handle_score(&mut self, score: &mut (u32, u32));
    fn reset(&mut self);
}

impl Ball for Block {
    fn handle_wall(&mut self){
        if self.border.x <= 0 || self.border.x + BLOCK_SIZE as i32 * 10   >= WINDOW_WIDTH as i32 {
            self.velocity_x = -self.velocity_x;
        }
        //self.border.y + BLOCK_SIZE as i32 * 10 >= WINDOW_HEIGTH as i32
        if self.border.y <= 0  {
            self.velocity_y = -self.velocity_y;
        }
    } 
    fn react_object(&mut self, object: &Block) -> bool {
        if (self.border.y == object.border.y || self.border.y + BLOCK_SIZE as i32 * 10 == object.border.y)
            && (self.border.x >= object.border.x && self.border.x <= object.border.x + calculate_object_right(object))
        {
            self.velocity_y = -self.velocity_y;
            return true;
        }
        false
    }

    fn handle_score(&mut self, score: &mut (u32, u32)) {
        if self.border.y + self.border.h as i32 <= 0 {
            score.1 += 1;
            self.reset();
        }
        if self.border.y >= WINDOW_HEIGTH as i32 {
            score.0 += 1;
            self.reset();
        }
    }

    fn reset(&mut self) {
        self.rect.x = 400;
        self.rect.y = 400;
        self.border.x = 400;
        self.border.y = 400;
        self.velocity_x = 0;
        self.velocity_y = 0;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    pub rect: Rect,
    pub border: Rect,
    pub velocity_x: i32,
    pub velocity_y: i32,
}

impl Block {
    pub fn new(rect: Rect, velocity_x: i32, velocity_y: i32) -> Self {
        let border = Rect::new(rect.x, rect.y, rect.w, rect.h);
        Self {
            rect: rect,
            border: border,
            velocity_x: velocity_x,
            velocity_y: velocity_y,
        }
    }

    pub fn update_position(&mut self) {
        self.rect.x += self.velocity_x;
        self.rect.y += self.velocity_y;
        self.border.x += self.velocity_x;
        self.border.y += self.velocity_y;
    }

    pub fn get_position(&self) -> (i32, i32) {
        (self.rect.x, self.rect.y)
    }
    
    pub fn get_size(&self) -> (u32, u32){
        (self.rect.w, self.rect.h)
    }
}
