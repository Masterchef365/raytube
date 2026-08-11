in vec3 pos_brightness;
out float brightness;

void main() {
    brightness = pos_brightness.z;
    gl_Position = vec4(pos_brightness.xy, 0.0, 1.0);
}
