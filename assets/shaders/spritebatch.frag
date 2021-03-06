#version 450

layout(location=0) in vec4 in_color;
layout(location=1) in vec2 in_uv;

layout(location=0) out vec4 out_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;

void main() {
    vec4 col = texture(sampler2D(t_diffuse, s_diffuse), in_uv);
    out_color = col * in_color;

    if (out_color.a == 0) {
        discard;
    }
}