#version 410

layout(location = 0) in float a_PositionY;
layout(location = 0) out float v_TexCoordY;

uniform float u_ViewOffset, u_ViewScale;
uniform float u_Time;

void main() {
    gl_Position = vec4(u_Time, a_PositionY, 0.0, 1.0);
    gl_Position.x -= u_ViewOffset;
    gl_Position.x /= u_ViewScale;
    gl_Position.x = gl_Position.x * 2.0 - 1.0;

    v_TexCoordY = a_PositionY * 0.5 + 0.5;
}