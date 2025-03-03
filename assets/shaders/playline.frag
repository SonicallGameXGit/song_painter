#version 410
#define PI 3.1415926536

layout(location = 0) in float v_TexCoordY;
layout(location = 0) out vec4 f_Color;

uniform float u_Time;

void main() {
    float multiplier = sin(u_Time * PI * 0.5) * 0.5 + 0.5;
    multiplier = multiplier * 0.7 + 0.3;
    f_Color = mix(vec4(0.16, 1.0, 0.1, 0.2), vec4(0.4, 1.0, 0.16, 1.0), v_TexCoordY * multiplier);
}