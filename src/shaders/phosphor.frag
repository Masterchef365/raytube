precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D back_buffer;
uniform float intensity;
uniform float saturation_point;

void main() {
    vec3 color = texture(back_buffer, uv).rgb;

    // Clip the color, modelling the tendency for electrons to stick and then repel others
    color = min(color, vec3(saturation_point));

    out_color = vec4(color * intensity, 1);
}
