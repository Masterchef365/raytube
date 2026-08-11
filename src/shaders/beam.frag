precision mediump float;
in float brightness;
out vec4 out_color;
uniform float intensity;

void main() {
    vec3 color = brightness * intensity * vec3(1.0);
    out_color = vec4(color, 1.0);
}
