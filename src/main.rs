pub mod engine;
pub mod paint;

use core::f32;
use std::time::{Duration, Instant};

use engine::{mesh::{Attribute, Layout, Mesh}, shader::Shader, texture::Texture, window::WindowBuilder};
use glfw::{Key, MouseButton};
use nalgebra::{Point2, Vector2};
use paint::Canvas;
use rodio::{OutputStream, Sink, Source};

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

    pub fn next(&mut self) -> f32 {
        let tone = &self.samples[self.i];
        let sample = f32::sin(self.time) * tone.amplitude;
        
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

struct PlayerSource {
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

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut window = WindowBuilder::default()
        .with_title("Note painter")
        .with_size(800, 600)
        .with_vsync(false)
        .build();
    unsafe {
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::ClearColor(0.1, 0.1, 0.1, 1.0);
        gl::LineWidth(2.0);
    }

    const NUM_NOTES: usize = 16;

    let timeline_shader = Shader::new("./assets/shaders/timeline.vert", "./assets/shaders/timeline.frag");
    timeline_shader.bind();
    timeline_shader.set_int("u_CanvasSampler", 0);
    timeline_shader.set_int("u_CMajorTemplateSampler", 1);
    timeline_shader.set_int("u_NumNotes", NUM_NOTES as i32);

    let playline_shader = Shader::new("./assets/shaders/playline.vert", "./assets/shaders/playline.frag");

    let cmajortemplate = Texture::load_from_file("./assets/textures/cmajortemplate.png", gl::NEAREST, gl::REPEAT);
    let square = Mesh::basic_square();
    let line = Mesh::new::<f32>(&[1.0, -1.0], &Layout::default().next_attribute(Attribute::Float), gl::LINES);

    let mut canvas = Canvas::new(800, 600);

    let mut last_window_width = window.get_width();
    let mut last_window_height = window.get_height();
    
    let mut fps_timer = Instant::now();
    let mut fps_counter = 0;

    let mut sink_timer = Instant::now();

    while window.is_running() {
        window.poll_events();

        fps_counter += 1;
        if fps_timer.elapsed().as_secs_f32() >= 1.0 {
            println!("FPS: {}.", fps_counter);

            fps_timer = Instant::now();
            fps_counter = 0;
        }

        if window.get_width() != last_window_width || window.get_height() != last_window_height {
            canvas.resize(window.get_width() as usize, window.get_height() as usize);
        }

        if window.is_mouse_button_just_pressed(MouseButton::Left) {
            canvas.new_record();
        }

        let mx = window.get_mouse_x();
        let my = window.get_mouse_y();
        let lmx = window.get_last_mouse_x();
        let lmy = window.get_last_mouse_y();
        if window.is_mouse_button_pressed(MouseButton::Left) && (mx != lmx || my != lmy) {
            let size = Vector2::new(window.get_width() as f32, window.get_height() as f32);
            canvas.line(
                Point2::new(lmx / size.x, lmy / size.y),
                Point2::new(mx / size.x, my / size.y),
            );
        }

        if window.is_key_pressed(Key::LeftControl) {
            if window.is_key_just_pressed(Key::Z) {
                canvas.undo();
            }
            if window.is_key_just_pressed(Key::Y) {
                canvas.redo();
            }
        }
        if window.is_key_just_pressed(Key::Space) {
            sink.stop();

            const SAMPLE_RATE: usize = 44100;
            let mut tones_samples = Vec::new();
            for lines in canvas.get_lines() {
                let mut samples = vec![Tone { frequency: 0.0, amplitude: 0.0 }; SAMPLE_RATE];
                for line in lines {
                    let min = if line.start.x < line.end.x { line.start } else { line.end };
                    let max = if line.start.x > line.end.x { line.start } else { line.end };
                    
                    for (i, sample) in samples
                            .iter_mut()
                            .enumerate()
                            .skip((min.x * SAMPLE_RATE as f32) as usize)
                            .take(((max.x - min.x) * SAMPLE_RATE as f32) as usize + 1) {
                        let value = (1.0 - min.y + (max.y - min.y) * (i as f32 / SAMPLE_RATE as f32 - min.x)) * NUM_NOTES as f32 + 0.5 ;
                        let frequency = 440.0 * f32::powf(2.0, (value + 3.0) / 12.0);
                        let amplitude = 0.33;

                        *sample = Tone { frequency, amplitude };
                    }
                }

                tones_samples.push(ToneSamples::new(samples.into_boxed_slice()));
            }

            sink.append(PlayerSource::new(tones_samples.into_boxed_slice(), SAMPLE_RATE as u32));
            sink_timer = Instant::now();
        }

        canvas.update();
        
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT); }
        timeline_shader.bind();
        timeline_shader.set_int("u_NumNotes", NUM_NOTES as i32);

        unsafe {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, canvas.get_texture());

            cmajortemplate.bind(1);
        }
        square.draw();

        if !sink.empty() {
            playline_shader.bind();
            playline_shader.set_float("u_Time", sink_timer.elapsed().as_secs_f32());

            line.draw();
        }

        window.swap_buffers();

        last_window_width = window.get_width();
        last_window_height = window.get_height();
    }
}