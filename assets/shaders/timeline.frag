#version 410

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 f_Color;

uniform sampler2D u_CanvasSampler, u_CMajorTemplateSampler;
uniform int u_NumNotes;

void main() {
    f_Color = vec4(1.0);

    f_Color.rgb *= 0.2 + texture2D(u_CMajorTemplateSampler, vec2(0.0, 1.0 - v_TexCoord.y * float(u_NumNotes) / 12.0)).rgb.r * 0.05;
    f_Color.rgb *= mod(v_TexCoord.x * 4.0 * 0.5, 1.0) > 0.5 ? 1.0 : 0.9;

    f_Color.rgb += texture(u_CanvasSampler, vec2(v_TexCoord.x, 1.0 - v_TexCoord.y)).rrr;
}