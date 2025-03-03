use crate::engine::{mesh::{Attribute, Layout, Mesh}, shader::Shader, texture::Texture};

pub struct Resources {
    pub square_mesh: Mesh,
    pub line_mesh: Mesh,

    pub timeline_shader: Shader,
    pub timeline_tone_shader: Shader,
    pub playline_shader: Shader,

    pub cmajor_template_texture: Texture,
}

impl Default for Resources {
    fn default() -> Self {
        let timeline_shader = Shader::new("./assets/shaders/timeline.vert", "./assets/shaders/timeline.frag");
        timeline_shader.bind();
        timeline_shader.set_int("u_CMajorTemplateSampler", 0);

        Self {
            square_mesh: Mesh::basic_square(),
            line_mesh: Mesh::new(&[1.0, -1.0], &Layout::default().next_attribute(Attribute::Float), gl::LINES),

            timeline_shader,
            timeline_tone_shader: Shader::new(
                "./assets/shaders/timeline_tone.vert",
                "./assets/shaders/timeline_tone.frag",
            ),
            playline_shader: Shader::new("./assets/shaders/playline.vert", "./assets/shaders/playline.frag"),

            cmajor_template_texture: Texture::load_from_file("./assets/textures/cmajortemplate.png", gl::NEAREST, gl::REPEAT),
        }
    }
}