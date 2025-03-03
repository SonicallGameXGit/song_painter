#version 410

layout(location = 0) in float a_Mix;
layout(location = 1) in vec4 a_Transform;

uniform vec2 u_ViewOffset, u_ViewScale;

void main() {
    gl_Position = vec4(mix(a_Transform.xy, a_Transform.zw, a_Mix), 0.0, 1.0);
    gl_Position.xy -= u_ViewOffset;
    gl_Position.xy /= u_ViewScale;
    gl_Position.xy = gl_Position.xy * 2.0 - 1.0;
}