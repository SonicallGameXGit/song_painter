use core::f32;
use std::{cmp::Ordering, time::{Duration, Instant}};

use gl::types::{GLint, GLsizeiptr, GLuint};
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
    pub fn add_line(&mut self, start: Point2<f32>, end: Point2<f32>, tone_system: &mut ToneSystem) -> Option<&Line> {
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
            tone_system.mark_dirty();

            return last.last();
        }

        None
    }

    pub fn undo(&mut self, tone_system: &mut ToneSystem) {
        if let Some(lines) = self.history.pop() {
            self.undo_history.push(lines);
            tone_system.mark_dirty();
        }
    }
    pub fn redo(&mut self, tone_system: &mut ToneSystem) {
        if let Some(lines) = self.undo_history.pop() {
            self.history.push(lines);
            tone_system.mark_dirty();
        }
    }
}

#[derive(Default)]
pub struct ToneSystem {
    tones_lines_mesh: LinesMesh,
    dirty: bool,
}
impl ToneSystem {
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn update(&mut self, record_system: &RecordSystem) {
        if self.dirty {
            self.tones_lines_mesh.update(&record_system.history.iter().flatten().cloned().collect::<Vec<Line>>());
            self.dirty = false;
        }
    }
}

#[derive(Default)]
pub struct DrawingSystem {
    mouse_click_x: f32,
    mouse_click_y: f32,
}
impl DrawingSystem {
    pub fn update(&mut self, window: &Window, view: &View, tone_system: &mut ToneSystem, record_system: &mut RecordSystem) {
        if window.is_mouse_button_just_pressed(MouseButton::Left) {
            record_system.new_record();

            self.mouse_click_x = window.get_mouse_x() / window.get_width() as f32 * view.scale.x + view.offset.x;
            self.mouse_click_y = (1.0 - window.get_mouse_y() / window.get_height() as f32) * view.scale.y + view.offset.y;
        }
        
        const SAFE_RADIUS: f32 = 0.01;
        let mouse_x = window.get_mouse_x() / window.get_width() as f32 * view.scale.x + view.offset.x;
        let mouse_y = (1.0 - window.get_mouse_y() / window.get_height() as f32) * view.scale.y + view.offset.y;

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
                    tone_system,
                );
            } else {
                let is_mouse_moved = window.get_mouse_x() != window.get_last_mouse_x() || window.get_mouse_y() != window.get_last_mouse_y();
                if is_mouse_moved {
                    record_system.add_line(
                        Point2::new(
                            window.get_last_mouse_x() / width * view.scale.x + view.offset.x,
                            (1.0 - window.get_last_mouse_y() / height) * view.scale.y + view.offset.y,
                        ),
                        Point2::new(mouse_x, mouse_y),
                        tone_system,
                    );
                }
            }

            self.mouse_click_x = f32::INFINITY;
            self.mouse_click_y = f32::INFINITY;
        }
    }
}

pub struct RenderSystem;
impl RenderSystem {
    pub fn draw_timeline(&self, resources: &Resources, view: &View, bpm: f32) {
        resources.timeline_shader.bind();
        resources.timeline_shader.set_vec2("u_ViewOffset", &view.offset);
        resources.timeline_shader.set_vec2("u_ViewScale", &view.scale);
        resources.timeline_shader.set_float("u_BPM", bpm);

        resources.cmajor_template_texture.bind(0);
        resources.square_mesh.draw();
    }
    pub fn draw_timeline_tones(&self, resources: &Resources, tone_system: &ToneSystem, view: &View) {
        resources.timeline_tone_shader.bind();
        resources.timeline_tone_shader.set_vec2("u_ViewOffset", &view.offset);
        resources.timeline_tone_shader.set_vec2("u_ViewScale", &view.scale);

        tone_system.tones_lines_mesh.draw();
    }
    pub fn draw_playline(&self, resources: &Resources, play_time: f32, view: &View) {
        resources.playline_shader.bind();
        resources.playline_shader.set_float("u_Time", play_time);
        resources.playline_shader.set_float("u_ViewOffset", view.offset.x);
        resources.playline_shader.set_float("u_ViewScale", view.scale.x);

        resources.line_mesh.draw();
    }
}

pub struct View {
    offset: Vector2<f32>,
    scale: Vector2<f32>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            offset: Vector2::zeros(),
            scale: Vector2::new(4.0, 16.0),
        }
    }
}

pub struct Timeline {
    record_system: RecordSystem,
    drawing_system: DrawingSystem,
    render_system: RenderSystem,
    tone_system: ToneSystem,

    view: View,

    playing: bool,
    player_timer: Instant,
    player_duration: Duration,
    player_bpm: f32,
}
impl Timeline {
    fn update_record_system(&mut self, window: &Window) {
        if window.is_key_pressed(Key::LeftControl) && window.is_key_just_pressed(Key::Z) {
            if window.is_key_pressed(Key::LeftShift) {
                self.record_system.redo(&mut self.tone_system);
            } else {
                self.record_system.undo(&mut self.tone_system);
            }
        }
    }
    fn update_player(&mut self) {
        if self.playing {
            if self.player_timer.elapsed() >= self.player_duration {
                self.playing = false;
                self.player_duration = Duration::ZERO;
            }
        } else {
            self.player_timer = Instant::now();
        }
    }
    fn update_drawing_system(&mut self, window: &Window) {
        self.drawing_system.update(window, &self.view, &mut self.tone_system, &mut self.record_system);
    }
    fn update_view(&mut self, window: &Window) {
        const ZOOM_SPEED: f32 = 0.25;
        const SCROLL_SPEED_X: f32 = 0.1;
        const SCROLL_SPEED_Y: f32 = 0.5;

        const SCALE_X_MIN: f32 = 0.25;
        const SCALE_X_MAX: f32 = 400.0;

        const SCALE_Y_MIN: f32 = 6.0;
        const SCALE_Y_MAX: f32 = 48.0;

        let is_ctrl_pressed = window.is_key_pressed(Key::LeftControl) || window.is_key_pressed(Key::RightControl);
        let is_alt_pressed = window.is_key_pressed(Key::LeftAlt) || window.is_key_pressed(Key::RightAlt);

        if is_alt_pressed && !is_ctrl_pressed {
            self.view.scale.y -= window.get_scroll_dy() * ZOOM_SPEED;
            self.view.offset.y += window.get_scroll_dy() * ZOOM_SPEED * 0.5;
        }
        if is_ctrl_pressed && !is_alt_pressed {
            self.view.scale.x -= window.get_scroll_dy() * ZOOM_SPEED;
            self.view.offset.x += window.get_scroll_dy() * ZOOM_SPEED * 0.5;
        }
        
        if !is_ctrl_pressed && !is_alt_pressed {
            if window.is_key_pressed(Key::LeftShift) || window.is_key_pressed(Key::RightShift) {
                self.view.offset.x -= window.get_scroll_dy() * SCROLL_SPEED_X;
                self.view.offset.y -= window.get_scroll_dx() * SCROLL_SPEED_Y;
            } else {
                self.view.offset.y += window.get_scroll_dy() * SCROLL_SPEED_Y;
                self.view.offset.x -= window.get_scroll_dx() * SCROLL_SPEED_X;
            }
        }
        
        self.view.scale.x = self.view.scale.x.clamp(SCALE_X_MIN, SCALE_X_MAX);
        self.view.scale.y = self.view.scale.y.clamp(SCALE_Y_MIN, SCALE_Y_MAX);
        self.view.offset.x = f32::max(self.view.offset.x, 0.0);

        // TODO: Add mouse movement when Middle button is pressed
    }

    pub fn update(&mut self, window: &Window) {
        self.update_record_system(window);
        self.update_player();
        self.update_drawing_system(window);
        self.update_view(window);

        self.tone_system.update(&self.record_system);
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
        self.render_system.draw_timeline(resources, &self.view, self.player_bpm);
        self.render_system.draw_timeline_tones(resources, &self.tone_system, &self.view);

        if self.playing {
            self.render_system.draw_playline(resources, self.player_timer.elapsed().as_secs_f32() / 60.0 * self.player_bpm, &self.view);
        }
    }

    pub fn render_audio(&self) -> PlayerSource {
        const SAMPLE_RATE: usize = 44100;

        let mut tones_samples = Vec::new();
        let length = self.record_system.history
            .iter()
            .flatten()
            .max_by(|a, b| {
                if f32::max(a.start.x, a.end.x) > f32::max(b.start.x, b.end.x) {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            });
        
        if let Some(length) = length {
            let length = (f32::max(length.start.x, length.end.x) / (self.player_bpm / 60.0) * SAMPLE_RATE as f32) as usize + 1;

            for lines in &self.record_system.history {
                let mut samples = vec![Tone { frequency: 0.0, amplitude: 0.0 }; length];
                for line in lines {
                    let start = Point2::new(line.start.x / (self.player_bpm / 60.0), line.start.y);
                    let end = Point2::new(line.end.x / (self.player_bpm / 60.0), line.end.y);

                    let min = if start.x < end.x { start } else { end };
                    let max = if start.x > end.x { start } else { end };
                    
                    for (i, sample) in samples
                            .iter_mut()
                            .enumerate()
                            .skip((min.x * SAMPLE_RATE as f32) as usize)
                            .take(((max.x - min.x) * SAMPLE_RATE as f32) as usize + 1) {
                        let value = (min.y + (max.y - min.y) * (i as f32 / SAMPLE_RATE as f32 - min.x)) + 0.5;
                        let frequency = 440.0 * f32::powf(2.0, (value + 3.0) / 12.0);
                        let amplitude = 0.33;
    
                        *sample = Tone { frequency, amplitude };
                    }
                }
    
                tones_samples.push(ToneSamples::new(samples.into_boxed_slice()));
            }
        }

        PlayerSource::new(tones_samples.into_boxed_slice(), SAMPLE_RATE as u32)
    }
}
impl Default for Timeline {
    fn default() -> Self {
        Self {
            record_system: RecordSystem::default(),
            drawing_system: DrawingSystem::default(),
            tone_system: ToneSystem::default(),
            render_system: RenderSystem,

            view: View::default(),

            playing: false,
            player_timer: Instant::now(),
            player_duration: Duration::ZERO,
            player_bpm: 120.0,
        }
    }
}

struct LinesMesh {
    vao: GLuint,
    base_vbo: GLuint,
    instance_vbo: GLuint,
    num_lines: usize,
}
impl LinesMesh {
    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArraysInstanced(gl::LINES, 0, 2, self.num_lines as GLint);
        }
    }
    pub fn update(&mut self, lines: &[Line]) {
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.instance_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(lines) as GLsizeiptr,
                lines.as_ptr() as *const std::ffi::c_void,
                gl::STATIC_DRAW,
            );
        }

        self.num_lines = lines.len();
    }
}
impl Default for LinesMesh {
    fn default() -> Self {
        let mut vao = 0;
        let mut base_vbo = 0;
        let mut instance_vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut base_vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, base_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of::<[f32; 2]>() as isize,
                [0.0f32, 1.0f32].as_ptr() as *const _,
                gl::STATIC_DRAW
            );
            gl::VertexAttribPointer(0, 1, gl::FLOAT, gl::FALSE, std::mem::size_of::<f32>() as i32, std::ptr::null());
            gl::EnableVertexAttribArray(0);

            gl::GenBuffers(1, &mut instance_vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, instance_vbo);
            gl::VertexAttribPointer(1, 4, gl::FLOAT, gl::FALSE, std::mem::size_of::<[f32; 4]>() as i32, std::ptr::null());
            gl::VertexAttribDivisor(1, 1);
            gl::EnableVertexAttribArray(1);
        }

        Self {
            vao,
            base_vbo,
            instance_vbo,
            num_lines: 0,
        }
    }
}
impl Drop for LinesMesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.base_vbo);
            gl::DeleteBuffers(1, &self.instance_vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}

#[repr(C)]
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

    pub fn last_amplitude(&self) -> f32 {
        self.samples[self.i].amplitude
    }

    fn get_sample(&self) -> f32 {
        f32::sin(self.time)
    }
}
impl Iterator for ToneSamples {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let tone = &self.samples[self.i];
        let sample = self.get_sample() * tone.amplitude;
        
        self.i += 1;
        if self.i >= self.samples.len() {
            self.i = self.samples.len() - 1;
            return None;
        }

        self.time += f32::consts::PI * 2.0 * tone.frequency / 44100.0;
        Some(sample)
    }
}

pub struct PlayerSource {
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
        
        let mut no_more_samples = true;
        for tone_samples in &mut self.tones_samples {
            if let Some(next_sample) = tone_samples.next() {
                no_more_samples = false;

                sample += next_sample;
                accumulated_amplitude += tone_samples.last_amplitude();
            }
        }
        if no_more_samples {
            return None;
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