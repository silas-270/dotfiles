#version 300 es
precision mediump float;

in vec2 v_texcoord;
layout(location = 0) out vec4 fragColor;

uniform sampler2D tex;

void main() {
    vec4 pixColor = texture(tex, v_texcoord);
    
    // Reduce blue and green channels to create a warm, blue-light-free tone (Amber/Clay style)
    pixColor[1] *= 0.90; // Green
    pixColor[2] *= 0.75; // Blue
    
    fragColor = pixColor;
}
