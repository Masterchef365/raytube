precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D phosphor;
uniform vec3 phosphor_color;

void main() {
    vec3 inten = texture(phosphor, uv).rgb;
    out_color = vec4(inten * phosphor_color, 1.0);
}

