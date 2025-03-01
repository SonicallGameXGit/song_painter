#version 410

layout(location = 0) in float v_TexCoordY;
layout(location = 0) out vec4 f_Color;

void main() {
    f_Color = mix(vec4(0.16, 1.0, 0.1, 0.2), vec4(0.4, 1.0, 0.16, 1.0), v_TexCoordY);
    // f_Color = mix(vec4(1.0, 0.0, 0.0, 0.0), vec4(0.0, 1.0, 0.0, 1.0), v_TexCoordY);
}