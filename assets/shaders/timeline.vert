#version 410

layout(location = 0) in vec2 a_Position;
layout(location = 0) out vec2 v_TexCoord;

void main() {
    gl_Position = vec4(a_Position * 2.0 - 1.0, 0.0, 1.0);
    v_TexCoord = a_Position;
}