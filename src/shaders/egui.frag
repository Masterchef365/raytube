precision highp float;

out vec4 out_color;
in vec2 uv;

void main() {
    out_color = vec4(0, uv, 1);
}

