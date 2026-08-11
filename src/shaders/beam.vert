out float brightness;
uniform sampler2D pos_buffer;

void main() {
    ivec2 size = textureSize(pos_buffer, 0);

    int line_id = gl_VertexID / 2;
    int point_id = gl_VertexID % 2;

    int i_1 = (line_id + 0) % size.x;
    int j_1 = (line_id + 0) / size.x;
    vec2 uv_1 = vec2(i_1, j_1)/vec2(size);
    vec2 pos_1 = texture(pos_buffer, uv_1).xy;

    int i_2 = (line_id + 1) % size.x;
    int j_2 = (line_id + 1) / size.x;
    vec2 uv_2 = vec2(i_2, j_2)/vec2(size);
    vec2 pos_2 = texture(pos_buffer, uv_2).xy;

    float dist = distance(pos_1, pos_2);

    vec2 out_pos = pos_1;
    if (point_id == 1) out_pos = pos_2;

    brightness = 1.0;
    if (dist != 0.0) brightness = 1.0 / dist;

    gl_Position = vec4(pos_1, 0.0, 1.0);
}
