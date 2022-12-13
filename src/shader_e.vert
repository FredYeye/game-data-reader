#version 450 core

//-----

layout (location = 0) in vec2 vertices;
layout (location = 1) in vec2 uv_in;
layout (location = 2) in vec4 color_in;

layout (location = 3) uniform vec2 size = vec2(1024.0, 768.0);

out vec2 uv;
out vec4 color;

//-----

void main() {
    gl_Position = vec4(
        2.0 * vertices.x / size.x - 1.0,
        1.0 - 2.0 * vertices.y / size.y,
        0.0,
        1.0
    );

    uv = uv_in;
    color = color_in / 255.0;
}
