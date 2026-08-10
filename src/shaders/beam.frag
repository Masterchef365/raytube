precision mediump float;
out vec4 out_color;
uniform float intensity;

void main() {
    vec3 color = intensity * vec3(1.0);
    out_color = vec4(color, 1.0);
}
