#version 450 core

//-----

layout (location = 0) in vec2 vertices;
layout (location = 1) in vec2 uv_in;
layout (location = 2) in vec4 color_in;

out vec2 uv;
out vec4 color;

//-----

vec3 linear_from_srgb(vec3 srgb) // 0-1 linear from 0-255 sRGB
{
    bvec3 cutoff = lessThan(srgb, vec3(10.31475));
    vec3 lower = srgb / vec3(3294.6);
    vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
    return mix(higher, lower, cutoff);
}

vec4 linear_from_srgba(vec4 srgba)
{
    return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
}

void main()
{
    gl_Position = vec4(
        2.0 * vertices.x / 1024.0 - 1.0,
        1.0 - 2.0 * vertices.y / 768.0,
        0.0,
        1.0
    );

    uv = uv_in;
    color = linear_from_srgba(color_in);
}
