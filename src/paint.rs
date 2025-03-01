use gl::types::{GLint, GLuint};
use nalgebra::Point2;

#[derive(Default, PartialEq)]
enum RecordDirection {
    #[default] Undefined,
    Right,
    Left,
}

#[derive(Default)]
struct RecordContext {
    direction: RecordDirection,
    clamp_x: f32,
}

pub struct Canvas {
    width: usize,
    height: usize,

    data: Box<[u8]>,
    texture: GLuint,

    history: Vec<Vec<Line>>,
    undo_history: Vec<Vec<Line>>,

    dirty: bool,
    current_rc: RecordContext,
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
            current_rc: RecordContext::default(),
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
        self.current_rc = RecordContext::default();
    }
    fn draw_line(&mut self, line: &Line) {
        if line.start.x < 0.0 || line.start.y < 0.0 || line.end.x < 0.0 || line.end.y < 0.0 {
            return;
        }
        if line.start.x >= 1.0 || line.start.y >= 1.0 || line.end.x >= 1.0 || line.end.y >= 1.0 {
            return;
        }

        let start = Point2::new(
            (line.start.x * self.width as f32) as i32,
            (line.start.y * self.height as f32) as i32,
        );
        let end = Point2::new(
            (line.end.x * self.width as f32) as i32,
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
        if self.current_rc.direction == RecordDirection::Undefined {
            if start.x < end.x {
                self.current_rc.direction = RecordDirection::Right;
                self.current_rc.clamp_x = 0.0;
            } else {
                self.current_rc.direction = RecordDirection::Left;
                self.current_rc.clamp_x = f32::INFINITY;
            }
        }

        let clamping_func = if self.current_rc.direction == RecordDirection::Right { f32::max } else { f32::min };

        let mut line = Line { start, end };
        line.start.x = clamping_func(line.start.x, self.current_rc.clamp_x);
        line.end.x = clamping_func(line.end.x, self.current_rc.clamp_x);
        
        self.draw_line(&line);
        self.current_rc.clamp_x = clamping_func(self.current_rc.clamp_x, clamping_func(line.start.x, line.end.x));
        if let Some(last) = self.history.last_mut() { last.push(line); }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;

        self.data = vec![0; width * height].into_boxed_slice();
        self.dirty = true;

        for lines in &self.history.clone() {
            for line in lines {
                self.draw_line(line);
            }
        }
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