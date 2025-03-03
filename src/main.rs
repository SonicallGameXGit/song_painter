pub mod engine;
pub mod timeline;
pub mod resources;

use std::time::Instant;

use engine::window::WindowBuilder;
use glfw::Key;
use resources::Resources;
use timeline::Timeline;
use rodio::{OutputStream, Sink};

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

        if window.is_key_just_pressed(Key::Space) {
            timeline.play(&sink);
        }

        timeline.update(&window);
        
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT); }
        timeline.draw(&resources);

        window.swap_buffers();
    }
}