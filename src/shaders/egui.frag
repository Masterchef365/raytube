precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D phosphor;

void main() {
    vec3 color = texture(phosphor, uv).rgb;
    out_color = vec4(color, 1);
}

