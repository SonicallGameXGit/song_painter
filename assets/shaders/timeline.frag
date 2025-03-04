#version 410

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 f_Color;

uniform sampler2D u_CMajorTemplateSampler;

uniform vec2 u_ViewOffset;
uniform vec2 u_ViewScale;
uniform float u_BPM;

void main() {
    const float c_NumOctaveNotes = 12.0;

    vec2 world_texcoord = v_TexCoord * u_ViewScale;
    f_Color = vec4(1.0);
    f_Color.rgb *= 0.2 + texture2D(u_CMajorTemplateSampler, vec2(0.0, (world_texcoord.y + u_ViewOffset.y) / c_NumOctaveNotes)).rgb.r * 0.05;

    float barFields = mod((world_texcoord.x + u_ViewOffset.x) * 0.5, 1.0) > 0.5 ? 1.0 : 0.9;
    barFields = mix(barFields, 1.0, clamp(u_ViewScale.x / 16.0 - 12.0, 0.0, 1.0));

    f_Color.rgb *= barFields;

    float estriperLine = mod((world_texcoord.x + u_ViewOffset.x) * 4.0, 1.0) <= 0.01 * u_ViewScale.x ? 0.8 : 1.0;
    estriperLine = mix(estriperLine, 1.0, clamp(u_ViewScale.x / 8.0 - 3.0, 0.0, 1.0));

    f_Color.rgb *= estriperLine;
}