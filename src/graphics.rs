use macroquad::prelude::*;
use macroquad::miniquad::{Comparison, PipelineParams};

pub fn hash2(x: u32, y: u32) -> f32 {
    let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) as f32) / (u32::MAX as f32)
}

pub fn make_tex(w: u16, h: u16, f: impl Fn(u16, u16) -> [u8; 4]) -> Texture2D {
    let mut bytes = Vec::with_capacity((w as usize) * (h as usize) * 4);
    for y in 0..h { for x in 0..w { bytes.extend_from_slice(&f(x, y)); } }
    let tex = Texture2D::from_rgba8(w, h, &bytes);
    tex.set_filter(FilterMode::Linear);
    tex
}

pub fn make_sphere_mesh(radius: f32, rings: u16, sectors: u16, tex: Option<&Texture2D>, col: Color) -> Mesh {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        for s in 0..=sectors {
            let u = s as f32 / sectors as f32;
            let theta = u * std::f32::consts::TAU;
            let x = phi.sin() * theta.cos() * radius;
            let y = phi.cos() * radius;
            let z = phi.sin() * theta.sin() * radius;
            vertices.push(Vertex::new(x, y, z, u, v, col));
        }
    }
    for r in 0..rings {
        for s in 0..sectors {
            let a = r * (sectors + 1) + s;
            let b = a + sectors + 1;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Mesh { vertices, indices, texture: tex.cloned() }
}
pub fn create_horror_material() -> Material {
    let vs = "#version 100
    attribute vec3 position; attribute vec2 texcoord; attribute vec4 color0;
    varying lowp vec2 uv; varying lowp vec4 color; varying highp vec3 world_pos;
    uniform mat4 Model; uniform mat4 Projection;
    void main() {
        vec4 wp = Model * vec4(position, 1.0);
        world_pos = wp.xyz; gl_Position = Projection * wp;
        color = color0 / 255.0; uv = texcoord;
    }";

    let fs = "#version 100
    precision highp float;
    varying lowp vec2 uv; varying lowp vec4 color; varying highp vec3 world_pos;
    uniform sampler2D Texture; uniform vec3 u_cam_pos; uniform vec3 u_cam_dir;
    uniform float u_on; uniform float u_bright; uniform float u_time;
    uniform vec3 u_l0; uniform vec3 u_l1; uniform vec3 u_l2; uniform vec3 u_l3;

    vec3 safe_norm(vec3 v) {
        float l = length(v);
        return l > 0.0001 ? v / l : vec3(0.0, 1.0, 0.0);
    }

    float hash3(vec3 p) {
        p = fract(p * 0.3183099 + 0.1);
        p *= 17.0;
        return fract(p.x * p.y * p.z * (p.x + p.y + p.z));
    }

    vec3 aces(vec3 x) {
        x = max(x, vec3(0.0));
        return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
    }

    vec3 add_pt(vec3 lp, vec3 wp, vec3 N, vec3 col) {
        vec3 dvec = lp - wp;
        float d = length(dvec);
        float nd = mix(0.55, 1.0, max(dot(N, safe_norm(dvec)), 0.0));
        return col * nd * (2.8 / (1.0 + d * d * 0.16));
    }

    void main() {
        vec4 tex = texture2D(Texture, uv) * color;
        vec3 to_f = world_pos - u_cam_pos;
        float dist = length(to_f);
        vec3 V = safe_norm(-to_f);

        float n1 = sin(world_pos.x * 2.3 + world_pos.z * 1.9 + sin(world_pos.z * 0.8) * 1.5);
        float n2 = cos(world_pos.x * 5.7 - world_pos.z * 4.9 + world_pos.y * 3.1);
        vec3 N = safe_norm(vec3((n1 * 0.06 + n2 * 0.04), 0.98, (n2 * 0.06 - n1 * 0.04)));

        float puddle = smoothstep(0.45, 0.78, sin(world_pos.x * 0.35) * cos(world_pos.z * 0.35) + n1 * 0.25) * (1.0 - smoothstep(0.2, 1.6, world_pos.y));

        vec3 L = safe_norm(u_cam_pos - world_pos);
        float spot = dot(safe_norm(u_cam_dir), -L);
        float core = pow(clamp((spot - 0.915) / 0.085, 0.0001, 1.0), 1.8) * 1.75;
        float spill = pow(clamp((spot - 0.64) / 0.36, 0.0001, 1.0), 2.4) * 0.98;
        float beam = (core + spill) * u_on;

        vec3 H = safe_norm(L + V);
        float fresnel = pow(max(1.0 - max(dot(N, V), 0.0), 0.0001), 3.5);
        float gloss = mix(0.12, 0.95, puddle);
        float spec = pow(max(dot(N, H), 0.0001), mix(44.0, 96.0, puddle)) * gloss * (0.2 + fresnel * 0.8) * spill * u_on;
        float atten = 1.0 / (1.0 + 0.036 * dist + 0.013 * dist * dist);

        vec3 moon_dir = safe_norm(vec3(-0.45, 0.78, -0.42));
        vec3 moon_H = safe_norm(moon_dir + V);
        float moon_spec = pow(max(dot(N, moon_H), 0.0001), 64.0) * puddle * 0.42;
        float rim = pow(max(1.0 - max(dot(N, V), 0.0), 0.0001), 3.0) * 0.045;
        vec3 ambient = vec3(0.036, 0.052, 0.082) + vec3(0.05, 0.07, 0.11) * rim;

        vec3 pt_light = add_pt(u_l0, world_pos, N, vec3(1.15, 0.76, 0.36))
                      + add_pt(u_l1, world_pos, N, vec3(1.15, 0.76, 0.36))
                      + add_pt(u_l2, world_pos, N, vec3(1.15, 0.76, 0.36))
                      + add_pt(u_l3, world_pos, N, vec3(1.15, 0.76, 0.36));

        vec3 beam_tint = mix(vec3(0.88, 0.90, 0.86), vec3(1.06, 0.99, 0.88), clamp(core, 0.0, 1.0));
        vec3 base_albedo = mix(tex.rgb, tex.rgb * 0.55, puddle * 0.65);
        vec3 lit = (base_albedo * (ambient + beam_tint * beam * atten + pt_light) + vec3(0.90, 0.95, 1.0) * spec * atten + vec3(0.45, 0.58, 0.82) * moon_spec) * u_bright;

        float valley_mist = clamp((1.4 - world_pos.y) * 0.022, 0.0, 0.32);
        float fog = clamp(1.0 - exp(-dist * 0.125) + valley_mist, 0.0, 1.0);
        vec3 fog_col = vec3(0.024, 0.035, 0.058) * u_bright;
        vec3 final_col = aces(mix(lit, fog_col, fog));
        if (!(final_col.r >= 0.0 && final_col.g >= 0.0 && final_col.b >= 0.0)) { final_col = fog_col; }

        float grain = (hash3(vec3(gl_FragCoord.xy, fract(u_time) * 100.0)) - 0.5) * 0.018;
        gl_FragColor = vec4(clamp(final_col + grain, 0.0, 1.0), tex.a);
    }";

    load_material(
        ShaderSource::Glsl { vertex: vs, fragment: fs },
        MaterialParams {
            pipeline_params: PipelineParams { depth_write: true, depth_test: Comparison::LessOrEqual, cull_face: macroquad::miniquad::CullFace::Nothing, ..Default::default() },
            uniforms: vec![
                UniformDesc::new("u_cam_pos", UniformType::Float3),
                UniformDesc::new("u_cam_dir", UniformType::Float3),
                UniformDesc::new("u_on", UniformType::Float1),
                UniformDesc::new("u_bright", UniformType::Float1),
                UniformDesc::new("u_time", UniformType::Float1),
                UniformDesc::new("u_l0", UniformType::Float3),
                UniformDesc::new("u_l1", UniformType::Float3),
                UniformDesc::new("u_l2", UniformType::Float3),
                UniformDesc::new("u_l3", UniformType::Float3),
            ],
            ..Default::default()
        },
    ).unwrap()
}
