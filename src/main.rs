pub mod engine;
pub mod timeline;
pub mod resources;

use std::{path::PathBuf, time::Instant};

use engine::window::WindowBuilder;
use glfw::Key;
use hound::{WavSpec, WavWriter};
use resources::Resources;
use rfd::FileDialog;
use timeline::Timeline;
use rodio::{OutputStream, Sink};

fn save_to_file(path: &PathBuf, samples: &[i16]) {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    
    let mut writer = match WavWriter::create(path, spec) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to create writer for file at: {}. Error: {}", path.display(), error);
            return;
        },
    };
    for &sample in samples {
        writer.write_sample(sample).unwrap();
    }

    if let Err(error) = writer.finalize() {
        eprintln!("Failed to save file at: {}. Error: {}", path.display(), error);
    }
}

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    let mut window = WindowBuilder::default()
        .with_title("Note painter")
        .with_size(800, 600)
        .with_vsync(false)
        .with_max_fps(200)
        .with_msaa(2)
        .build();
    unsafe {
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::ClearColor(0.1, 0.1, 0.1, 1.0);
        gl::LineWidth(2.0);
    }

    let resources = Resources::default();
    let mut timeline = Timeline::default();

    let mut fps_timer = Instant::now();
    let mut fps_counter = 0u64;

    while window.is_running() {
        window.poll_events();
        if fps_timer.elapsed().as_secs() >= 1 {
            println!("FPS: {}.", fps_counter);
            fps_counter = 0;
            fps_timer = Instant::now();
        }
        fps_counter += 1;

        if window.is_key_pressed(Key::LeftControl) && window.is_key_just_pressed(Key::S) {
            let file_chooser: Option<PathBuf> = FileDialog::new()
                .set_title("Save as WAV File")
                .add_filter("WAV Audio", &["wav"])
                .set_file_name("output.wav")
                .save_file();

            if let Some(path) = file_chooser {
                let player_source = timeline.render_audio();
                
                let mut samples = Vec::new();
                for sample in player_source {
                    samples.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16);
                }
    
                save_to_file(&path, &samples);
            };
        }
        if window.is_key_just_pressed(Key::Space) {
            timeline.play(&sink);
        }

        timeline.update(&window);
        
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT); }
        timeline.draw(&resources);

        window.swap_buffers();
    }
}