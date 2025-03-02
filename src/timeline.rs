use core::f32;
use std::time::{Duration, Instant};

use gl::types::{GLint, GLuint};
use glfw::{Key, MouseButton};
use nalgebra::{Point2, Vector2};
use rodio::{Sink, Source};

use crate::{engine::window::Window, resources::Resources};

#[derive(Default, PartialEq)]
enum RecordDirection {
    #[default] Undefined,
    Right,
    Left,
}

#[derive(Default)]
struct Record {
    direction: RecordDirection,
    clamp_x: f32,
}

// TODO: Store lines as chunks and point to them from change history and in RenderSystem render the lines using instancing instead of drawing in texture
#[derive(Default)]
pub struct RecordSystem {
    history: Vec<Vec<Line>>,
    undo_history: Vec<Vec<Line>>,

    current: Record,
}
impl RecordSystem {
    pub fn new_record(&mut self) {
        self.current = Record::default();
        self.undo_history.clear();

        if let Some(lines) = self.history.last() {
            if lines.is_empty() {
                self.history.pop();
            }
        }
        self.history.push(Vec::new());
    }
    pub fn add_line(&mut self, start: Point2<f32>, end: Point2<f32>) -> Option<&Line> {
        if self.current.direction == RecordDirection::Undefined {
            if start.x < end.x {
                self.current.direction = RecordDirection::Right;
                self.current.clamp_x = 0.0;
            } else {
                self.current.direction = RecordDirection::Left;
                self.current.clamp_x = f32::INFINITY;
            }
        }
        let clamping_func = if self.current.direction == RecordDirection::Right { f32::max } else { f32::min };
        
        let mut line = Line { start, end };
        line.start.x = clamping_func(line.start.x, self.current.clamp_x);
        line.end.x = clamping_func(line.end.x, self.current.clamp_x);
        
        self.current.clamp_x = clamping_func(self.current.clamp_x, clamping_func(line.start.x, line.end.x));
        if let Some(last) = self.history.last_mut() {
            last.push(line.clone());
            return last.last();
        }

        None
    }

    pub fn undo(&mut self) {
        if let Some(lines) = self.history.pop() {
            self.undo_history.push(lines);
        }
    }
    pub fn redo(&mut self) {
        if let Some(lines) = self.undo_history.pop() {
            self.history.push(lines);
        }
    }
}

pub struct DrawingSystem {
    texture_width: usize,
    texture_height: usize,

    texture_data: Box<[u8]>,
    texture_id: GLuint,

    mouse_click_x: f32,
    mouse_click_y: f32,

    dirty: bool,
}
impl DrawingSystem {
    pub fn new(texture_width: usize, texture_height: usize) -> Self {
        let mut texture_id = 0;
        unsafe {
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::R8 as GLint,
                texture_width as GLint,
                texture_height as GLint,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }

        Self {
            texture_width,
            texture_height,

            texture_data: vec![0; texture_width * texture_height].into_boxed_slice(),
            texture_id,

            mouse_click_x: 0.0,
            mouse_click_y: 0.0,

            dirty: false,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn update_texture_content(&mut self) {
        if !self.dirty {
            return;
        }

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::R8 as GLint,
                self.texture_width as GLint,
                self.texture_height as GLint,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                self.texture_data.as_ptr() as *const std::ffi::c_void,
            );
        }

        self.dirty = false;
    }

    pub fn update(&mut self, record_system: &mut RecordSystem, window: &Window) {
        if window.is_mouse_button_just_pressed(MouseButton::Left) {
            record_system.new_record();

            self.mouse_click_x = window.get_mouse_x() / window.get_width() as f32;
            self.mouse_click_y = window.get_mouse_y() / window.get_height() as f32;
        }
        
        const SAFE_RADIUS: f32 = 0.01;
        let mouse_x = window.get_mouse_x() / window.get_width() as f32;
        let mouse_y = window.get_mouse_y() / window.get_height() as f32;

        let is_out_of_safe_radius =
            (mouse_x - self.mouse_click_x).abs() > SAFE_RADIUS ||
            (mouse_y - self.mouse_click_y).abs() > SAFE_RADIUS;
        
        if window.is_mouse_button_pressed(MouseButton::Left) && is_out_of_safe_radius {
            let width = window.get_width() as f32;
            let height = window.get_height() as f32;

            let is_just_out_from_safe_radius = self.mouse_click_x != f32::INFINITY && self.mouse_click_y != f32::INFINITY;
            if is_just_out_from_safe_radius {
                record_system.add_line(
                    Point2::new(self.mouse_click_x, self.mouse_click_y),
                    Point2::new(mouse_x, mouse_y),
                );
            } else {
                let is_mouse_moved = window.get_mouse_x() != window.get_last_mouse_x() || window.get_mouse_y() != window.get_last_mouse_y();
                if is_mouse_moved {
                    record_system.add_line(
                        Point2::new(window.get_last_mouse_x() / width, window.get_last_mouse_y() / height),
                        Point2::new(mouse_x, mouse_y),
                    );
                }
            }

            self.mouse_click_x = f32::INFINITY;
            self.mouse_click_y = f32::INFINITY;
        }

        let is_window_resized = window.get_last_width() != window.get_width() || window.get_last_height() != window.get_height();
        if is_window_resized {
            self.resize(window.get_width() as usize, window.get_height() as usize, record_system);
        }

        self.update_texture_content();
    }

    fn id(&self, x: usize, y: usize) -> usize {
        (y * self.texture_width + x).min(self.texture_data.len())
    }

    fn draw_line(&mut self, line: &Line) {
        if line.start.x < 0.0 || line.start.y < 0.0 || line.end.x < 0.0 || line.end.y < 0.0 {
            return;
        }
        if line.start.x >= 1.0 || line.start.y >= 1.0 || line.end.x >= 1.0 || line.end.y >= 1.0 {
            return;
        }

        let start = Point2::new(
            (line.start.x * self.texture_width as f32) as i32,
            (line.start.y * self.texture_height as f32) as i32,
        );
        let end = Point2::new(
            (line.end.x * self.texture_width as f32) as i32,
            (line.end.y * self.texture_height as f32) as i32,
        );
        
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        let sx = (end.x - start.x).signum();
        let sy = (end.y - start.y).signum();
        let mut err = dx - dy;
        let mut x = start.x;
        let mut y = start.y;

        loop {
            self.texture_data[self.id(x as usize, y as usize)] = 255;

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
    fn resize(&mut self, width: usize, height: usize, record_system: &RecordSystem) {
        self.texture_width = width;
        self.texture_height = height;

        self.texture_data = vec![0; width * height].into_boxed_slice();
        self.dirty = true;

        for lines in &record_system.history.clone() {
            for line in lines {
                self.draw_line(line);
            }
        }
    }

    pub fn get_texture(&self) -> GLuint {
        self.texture_id
    }
}
impl Drop for DrawingSystem {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &self.texture_id); }
    }
}

pub struct RenderSystem;
impl RenderSystem {
    pub fn draw_timeline(&self, resources: &Resources, drawing_system: &DrawingSystem, zoom: &Vector2<f32>) {
        resources.timeline_shader.bind();
        resources.timeline_shader.set_vec2("u_Zoom", zoom);
        
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, drawing_system.get_texture());
        }

        resources.cmajor_template_texture.bind(1);
        resources.square_mesh.draw();
    }
    pub fn draw_playline(&self, resources: &Resources, play_time: f32) {
        resources.playline_shader.bind();
        resources.playline_shader.set_float("u_Time", play_time);

        resources.line_mesh.draw();
    }
}

pub struct Timeline {
    record_system: RecordSystem,
    drawing_system: DrawingSystem,
    render_system: RenderSystem,

    playing: bool,
    player_timer: Instant,
    player_duration: Duration,
}
impl Timeline {
    const NUM_NOTES: usize = 16;

    pub fn new(texture_width: usize, texture_height: usize) -> Self {
        Self {
            record_system: RecordSystem::default(),
            drawing_system: DrawingSystem::new(texture_width, texture_height),
            render_system: RenderSystem {},

            playing: false,
            player_timer: Instant::now(),
            player_duration: Duration::ZERO,
        }
    }

    pub fn update(&mut self, window: &Window) {
        if window.is_key_pressed(Key::LeftControl) && window.is_key_just_pressed(Key::Z) {
            if window.is_key_pressed(Key::LeftShift) {
                self.record_system.redo();
            } else {
                self.record_system.undo();
            }

            self.drawing_system.mark_dirty();
        }
        self.drawing_system.update(&mut self.record_system, window);

        if self.playing {
            if self.player_timer.elapsed() >= self.player_duration {
                self.playing = false;
                self.player_duration = Duration::ZERO;
            }
        } else {
            self.player_timer = Instant::now();
        }
    }
    pub fn play(&mut self, sink: &Sink) {
        let audio = self.render_audio();

        sink.stop();
        sink.append(self.render_audio());

        self.playing = true;
        self.player_timer = Instant::now();

        if let Some(duration) = audio.total_duration() {
            self.player_duration = duration;
        }
    }

    pub fn draw(&self, resources: &Resources) {
        self.render_system.draw_timeline(resources, &self.drawing_system, &Vector2::new(1.0, 1.0 / Self::NUM_NOTES as f32));

        if self.playing {
            self.render_system.draw_playline(resources, self.player_timer.elapsed().as_secs_f32());
        }
    }

    pub fn render_audio(&self) -> PlayerSource {
        const SAMPLE_RATE: usize = 44100;

        let mut tones_samples = Vec::new();
        for lines in &self.record_system.history {
            let mut samples = vec![Tone { frequency: 0.0, amplitude: 0.0 }; SAMPLE_RATE];
            for line in lines {
                let min = if line.start.x < line.end.x { line.start } else { line.end };
                let max = if line.start.x > line.end.x { line.start } else { line.end };
                
                for (i, sample) in samples
                        .iter_mut()
                        .enumerate()
                        .skip((min.x * SAMPLE_RATE as f32) as usize)
                        .take(((max.x - min.x) * SAMPLE_RATE as f32) as usize + 1) {
                    let value = (1.0 - min.y + (max.y - min.y) * (i as f32 / SAMPLE_RATE as f32 - min.x)) * Self::NUM_NOTES as f32 + 0.5 ;
                    let frequency = 440.0 * f32::powf(2.0, (value + 3.0) / 12.0);
                    let amplitude = 0.33;

                    *sample = Tone { frequency, amplitude };
                }
            }

            tones_samples.push(ToneSamples::new(samples.into_boxed_slice()));
        }

        PlayerSource::new(tones_samples.into_boxed_slice(), SAMPLE_RATE as u32)
    }
}

#[derive(Clone)]
pub struct Line {
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

#[derive(Clone)]
struct Tone {
    frequency: f32,
    amplitude: f32,
}
struct ToneSamples {
    samples: Box<[Tone]>,
    i: usize,
    time: f32,
}
impl ToneSamples {
    pub fn new(samples: Box<[Tone]>) -> Self {
        Self {
            samples,
            i: 0,
            time: 0.0,
        }
    }

    fn get_sample(&self) -> f32 {
        (f32::sin(self.time) + f32::sin(self.time * 1.01)) * 0.5
    }

    pub fn next(&mut self) -> f32 {
        
        let tone = &self.samples[self.i];
        let sample = self.get_sample() * tone.amplitude;
        
        self.i += 1;
        if self.i >= self.samples.len() {
            self.i = self.samples.len() - 1;
            return 0.0;
        }

        self.time += f32::consts::PI * 2.0 * tone.frequency / 44100.0;
        sample
    }

    pub fn last_amplitude(&self) -> f32 {
        self.samples[self.i].amplitude
    }
}

pub  struct PlayerSource {
    sample_rate: u32,
    tones_samples: Box<[ToneSamples]>,
}
impl PlayerSource {
    fn new(tones_samples: Box<[ToneSamples]>, sample_rate: u32) -> Self {
        Self {
            sample_rate,
            tones_samples,
        }
    }
}
impl Iterator for PlayerSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut sample = 0.0;
        let mut accumulated_amplitude = 0.0;
        
        for tone_samples in &mut self.tones_samples {
            sample += tone_samples.next();
            accumulated_amplitude += tone_samples.last_amplitude();
        }
        if accumulated_amplitude > 0.0 {
            sample /= f32::sqrt(accumulated_amplitude);
        }

        Some(sample)
    }
}
impl Source for PlayerSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let max_samples = self.tones_samples.iter()
            .map(|tone_samples| tone_samples.samples.len())
            .max()
            .unwrap_or(0);

        if max_samples == 0 {
            return None;
        }

        Some(Duration::from_secs_f32(max_samples as f32 / self.sample_rate as f32))
    }
}