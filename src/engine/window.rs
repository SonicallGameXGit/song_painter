use std::time::{Duration, Instant};

use glfw::{self, Context};
use gl;
use spin_sleep::SpinSleeper;

pub struct Window {
    pub glfw: glfw::Glfw,
    pub handle: glfw::PWindow,

    events: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
    keys: [u64; glfw::ffi::KEY_LAST as usize + 1],
    mouse_buttons: [u64; glfw::ffi::MOUSE_BUTTON_LAST as usize + 1],

    current_frame: u64,

    frame_duration: Duration,
    last_time: Instant,
    sleeper: spin_sleep::SpinSleeper,

    width: u32,
    height: u32,

    last_width: u32,
    last_height: u32,

    aspect: f32,

    mouse_x: f32,
    mouse_y: f32,

    last_mouse_x: f32,
    last_mouse_y: f32,

    mouse_dx: f32,
    mouse_dy: f32,

    frame_time: Instant,
    delta_time: Duration,
}

impl Window {
    pub fn make_current(&mut self) {
        self.handle.make_current();
    }

    pub fn is_running(&self) -> bool {
        !self.handle.should_close()
    }

    pub fn poll_events(&mut self) {
        self.delta_time = self.frame_time.elapsed();
        self.frame_time = Instant::now();

        let elapsed = self.last_time.elapsed();
        if elapsed < self.frame_duration {
            self.sleeper.sleep(self.frame_duration - elapsed);
        }

        self.last_time = Instant::now();

        self.glfw.poll_events();
        self.current_frame += 1;

        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                glfw::WindowEvent::FramebufferSize(width, height) => {
                    self.last_width = self.width;
                    self.last_height = self.height;

                    self.width = width as u32;
                    self.height = height as u32;

                    self.aspect = width as f32 / height as f32;
                    unsafe { gl::Viewport(0, 0, width, height); }
                }
                glfw::WindowEvent::Key(key, _, action, _) => {
                    match action {
                        glfw::Action::Press => {
                            self.keys[key as usize] = self.current_frame;
                        }
                        glfw::Action::Release => {
                            self.keys[key as usize] = 0;
                        }
                        _ => {}
                    }
                }
                glfw::WindowEvent::MouseButton(button, action, _) => {
                    match action {
                        glfw::Action::Press => {
                            self.mouse_buttons[button as usize] = self.current_frame;
                        }
                        glfw::Action::Release => {
                            self.mouse_buttons[button as usize] = 0;
                        }
                        _ => {}
                    }
                }
                
                _ => {}
            }
        }

        let cursor_pos = self.handle.get_cursor_pos();

        self.mouse_x = cursor_pos.0 as f32;
        self.mouse_y = cursor_pos.1 as f32;

        self.mouse_dx = self.mouse_x - self.last_mouse_x;
        self.mouse_dy = self.mouse_y - self.last_mouse_y;

        self.last_mouse_x = self.mouse_x;
        self.last_mouse_y = self.mouse_y;
    }

    pub fn swap_buffers(&mut self) {
        self.handle.swap_buffers();
    }

    pub fn grab_mouse(&mut self) {
        self.handle.set_cursor_mode(glfw::CursorMode::Disabled);
    }
    pub fn release_mouse(&mut self) {
        self.handle.set_cursor_mode(glfw::CursorMode::Normal);

        let cursor_pos = self.handle.get_cursor_pos();

        self.mouse_x = cursor_pos.0 as f32;
        self.mouse_y = cursor_pos.1 as f32;

        self.last_mouse_x = self.mouse_x;
        self.last_mouse_y = self.mouse_y;
    }
    pub fn toggle_mouse(&mut self) {
        if self.is_mouse_grabbed() {
            self.release_mouse();
        } else {
            self.grab_mouse();
        }
    }

    pub const fn is_key_pressed(&self, key: glfw::Key) -> bool {
        self.keys[key as usize] > 0
    }
    pub const fn is_key_just_pressed(&self, key: glfw::Key) -> bool {
        self.keys[key as usize] == self.current_frame
    }

    pub const fn is_mouse_button_pressed(&self, button: glfw::MouseButton) -> bool {
        self.mouse_buttons[button as usize] > 0
    }
    pub const fn is_mouse_button_just_pressed(&self, button: glfw::MouseButton) -> bool {
        self.mouse_buttons[button as usize] == self.current_frame
    }

    pub const fn get_mouse_x(&self) -> f32 {
        self.mouse_x
    }
    pub const fn get_mouse_y(&self) -> f32 {
        self.mouse_y
    }

    pub const fn get_last_mouse_x(&self) -> f32 {
        self.mouse_x - self.mouse_dx
    }
    pub const fn get_last_mouse_y(&self) -> f32 {
        self.mouse_y - self.mouse_dy
    }

    pub const fn get_mouse_dx(&self) -> f32 {
        self.mouse_dx
    }
    pub const fn get_mouse_dy(&self) -> f32 {
        self.mouse_dy
    }

    pub const fn get_width(&self) -> u32 {
        self.width
    }
    pub const fn get_height(&self) -> u32 {
        self.height
    }

    pub const fn get_last_width(&self) -> u32 {
        self.last_width
    }
    pub const fn get_last_height(&self) -> u32 {
        self.last_height
    }

    pub const fn get_aspect(&self) -> f32 {
        self.aspect
    }

    pub const fn get_delta(&self) -> Duration {
        self.delta_time
    }
    pub const fn get_delta_secs(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    pub fn is_mouse_grabbed(&self) -> bool {
        self.handle.get_cursor_mode() == glfw::CursorMode::Disabled
    }

    pub fn close(&mut self) {
        self.handle.set_should_close(true);
    }
}

pub struct WindowBuilder {
    width: u32,
    height: u32,
    title: String,
    vsync: bool,
    max_fps: u32,
    msaa: u32,
}

impl WindowBuilder {
    pub const NO_MAX_FPS: u32 = 0;
    
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;

        self
    }
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = String::from(title);
        self
    }
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }
    pub fn with_max_fps(mut self, max_fps: u32) -> Self {
        self.max_fps = max_fps;
        self
    }
    pub fn with_msaa(mut self, msaa_quality: u32) -> Self {
        self.msaa = msaa_quality;
        self
    }

    pub fn build(&self) -> Window {
        let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

        glfw.window_hint(glfw::WindowHint::ContextVersion(4, 6));
        glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Compat));

        if self.msaa > 0 {
            glfw.window_hint(glfw::WindowHint::Samples(Some(self.msaa)));
        }
    
        let (mut handle, events) = glfw.create_window(
            self.width, self.height,
            self.title.as_str(),
            glfw::WindowMode::Windowed
        ).expect("Failed to create GLFW window.");

        handle.make_current();

        let args: Vec<String> = std::env::args().collect();
        if !args.contains(&"--raw-input-off".to_string()) {
            println!("Using raw mouse motion.");
            handle.set_raw_mouse_motion(true);
        }

        handle.set_key_polling(true);
        handle.set_mouse_button_polling(true);
        handle.set_framebuffer_size_polling(true);

        glfw.set_swap_interval(if self.vsync { glfw::SwapInterval::Sync(1) } else { glfw::SwapInterval::None });

        let framebuffer_size: (i32, i32) = handle.get_framebuffer_size();
        gl::load_with(|procname| handle.get_proc_address(procname));
        
        unsafe { gl::Viewport(0, 0, framebuffer_size.0, framebuffer_size.1); }
        if self.msaa > 0 {
            unsafe { gl::Enable(gl::MULTISAMPLE); }
        }

        Window {
            glfw,
            handle,
            events,

            keys: [0; glfw::ffi::KEY_LAST as usize + 1],
            mouse_buttons: [0; glfw::ffi::MOUSE_BUTTON_LAST as usize + 1],

            current_frame: 0,

            frame_duration: if self.max_fps == Self::NO_MAX_FPS { Duration::ZERO } else { Duration::from_secs_f32(1.0 / self.max_fps as f32) },
            last_time: Instant::now(),
            sleeper: SpinSleeper::default(),

            width: framebuffer_size.0 as u32,
            height: framebuffer_size.1 as u32,

            last_width: framebuffer_size.0 as u32,
            last_height: framebuffer_size.1 as u32,

            aspect: framebuffer_size.0 as f32 / framebuffer_size.1 as f32,

            mouse_x: 0.0,
            mouse_y: 0.0,

            last_mouse_x: 0.0,
            last_mouse_y: 0.0,

            mouse_dx: 0.0,
            mouse_dy: 0.0,

            frame_time: Instant::now(),
            delta_time: Duration::ZERO,
        }
    }
}
impl Default for WindowBuilder {
    fn default() -> Self {
        Self {
            width: 1920 / 2,
            height: 1080 / 2,
            title: String::from("Untitled"),
            vsync: true,
            max_fps: 0,
            msaa: 0,
        }
    }
}