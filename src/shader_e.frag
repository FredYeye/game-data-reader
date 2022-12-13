#version 450 core

//-----

uniform sampler2D tex_sampler;

//-----

in vec2 uv;
in vec4 color;

out vec4 color_out;

//-----

vec3 srgb_gamma_from_linear(vec3 rgb) { // 0-1 sRGB gamma  from  0-1 linear
    bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
    vec3 lower = rgb * vec3(12.92);
    vec3 higher = vec3(1.055) * pow(rgb, vec3(1.0 / 2.4)) - vec3(0.055);
    return mix(higher, lower, vec3(cutoff));
}

vec4 srgba_gamma_from_linear(vec4 rgba) { // 0-1 sRGBA gamma  from  0-1 linear
    return vec4(srgb_gamma_from_linear(rgba.rgb), rgba.a);
}

void main() {
   vec4 texture_in_gamma = srgba_gamma_from_linear(texture2D(tex_sampler, uv));
   color_out = color * texture_in_gamma;
}
