precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D phosphor;

void main() {
    vec3 inten = texture(phosphor, uv).rgb;
    const vec3 phosphor_color = vec3(0.1, 1.0, 0.2);
    out_color = vec4(inten * phosphor_color, 1.0);
}

