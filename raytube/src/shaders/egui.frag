precision highp float;

out vec4 out_color;
in vec2 uv;

uniform sampler2D phosphor;
uniform vec3 phosphor_color;
uniform float barrel;

void main() {
    vec2 xy = uv * 2.0 - 1.0;
    float r = length(xy);
    r = min(r, 1.0);
    xy *= 1.0 - barrel * sqrt(1.0 - r * r);

    vec2 uv2 = (xy + 1.0) / 2.0;

    vec3 inten = texture(phosphor, uv2).rgb;

    out_color = vec4(inten * phosphor_color, 1.0);
}

