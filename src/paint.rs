use gl::types::{GLint, GLuint};
use nalgebra::Point2;

pub struct Canvas {
    width: usize,
    height: usize,

    data: Box<[u8]>,
    texture: GLuint,

    history: Vec<Vec<Line>>,
    undo_history: Vec<Vec<Line>>,

    dirty: bool,
    min_x: f32,
}
impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let mut texture = 0;
        unsafe {
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::R8 as GLint,
                width as GLint,
                height as GLint,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }

        Self {
            width,
            height,

            data: vec![0; width * height].into_boxed_slice(),
            texture,

            history: Vec::new(),
            undo_history: Vec::new(),

            dirty: false,
            min_x: 0.0,
        }
    }

    const fn id(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn update(&mut self) {
        if !self.dirty {
            return;
        }

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::R8 as GLint,
                self.width as GLint,
                self.height as GLint,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                self.data.as_ptr() as *const std::ffi::c_void,
            );
        }

        self.dirty = false;
    }

    pub fn new_record(&mut self) {
        if let Some(lines) = self.history.last() {
            if lines.is_empty() {
                self.history.pop();
            }
        }

        self.undo_history.clear();
        self.history.push(Vec::new());
        self.min_x = 0.0;
    }
    fn draw_line(&mut self, line: &Line) {
        if line.start.x < 0.0 || line.start.y < 0.0 || line.end.x < 0.0 || line.end.y < 0.0 {
            return;
        }
        if line.start.x >= 1.0 || line.start.y >= 1.0 || line.end.x >= 1.0 || line.end.y >= 1.0 {
            return;
        }

        let start = Point2::new(
            (f32::max(line.start.x, self.min_x) * self.width as f32) as i32,
            (line.start.y * self.height as f32) as i32,
        );
        let end = Point2::new(
            (f32::max(line.end.x, self.min_x) * self.width as f32) as i32,
            (line.end.y * self.height as f32) as i32,
        );
        
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        let sx = (end.x - start.x).signum();
        let sy = (end.y - start.y).signum();
        let mut err = dx - dy;
        let mut x = start.x;
        let mut y = start.y;

        loop {
            self.data[self.id(x as usize, y as usize)] = 255;

            if x == end.x && y == end.y {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }

        self.dirty = true;
    }
    pub fn line(&mut self, start: Point2<f32>, end: Point2<f32>) {
        let line = Line { start, end };
        
        self.draw_line(&line);
        self.min_x = f32::max(self.min_x, f32::max(line.start.x, line.end.x));
        if let Some(last) = self.history.last_mut() { last.push(line); }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        self.data = vec![0; width * height].into_boxed_slice();
        self.dirty = true;

        let last_min_x = self.min_x;
        self.min_x = 0.0;
        
        for lines in &self.history.clone() {
            for line in lines {
                self.draw_line(line);
            }
        }

        self.min_x = last_min_x;
    }

    pub fn undo(&mut self) {
        if let Some(lines) = self.history.pop() {
            self.undo_history.push(lines);
            self.resize(self.width, self.height);
        }
    }
    pub fn redo(&mut self) {
        if let Some(lines) = self.undo_history.pop() {
            for line in &lines {
                self.draw_line(line);
            }

            self.history.push(lines);
        }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }
    pub fn get_height(&self) -> usize {
        self.height
    }
    pub fn get_texture(&self) -> GLuint {
        self.texture
    }

    pub fn get_lines(&self) -> &Vec<Vec<Line>> {
        &self.history
    }
}
impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &self.texture); }
    }
}

#[derive(Clone)]
pub struct Line {
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}