precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D back_buffer;
uniform float intensity;
uniform float saturation_point;

void main() {
    float hysteresis = texture(back_buffer, uv).r;

    // Clip the color, modelling the tendency for electrons to stick and then repel others
    hysteresis = clamp(hysteresis, -saturation_point, saturation_point);

    out_color = vec4(vec3(hysteresis * intensity), 1);
}
