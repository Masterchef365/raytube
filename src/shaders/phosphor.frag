precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D back_buffer;
uniform float intensity;

void main() {
    vec3 color = texture(back_buffer, uv).rgb;
    out_color = vec4(color * intensity, 1);
}
