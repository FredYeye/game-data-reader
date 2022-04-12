#version 450 core

//-----

uniform sampler2D tex_sampler;

//-----

in vec2 uv;
in vec4 color;

out vec4 color_out;

//-----

void main()
{
   color_out = color * texture(tex_sampler, uv);
}
