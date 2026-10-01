//! GLSL sources. Bodies are written in the GLSL ES 3.00 subset that desktop
//! GLSL 3.30 also accepts, so one source runs on Freedreno (GLES 3.2), on
//! Windows (desktop GL) and on Mesa llvmpipe in CI (ADR 0008).

/// Header for OpenGL ES 3 contexts.
pub const HEADER_ES: &str = "#version 300 es\nprecision highp float;\nprecision highp int;\n";

/// Header for desktop OpenGL 3.3+ core contexts.
pub const HEADER_DESKTOP: &str = "#version 330 core\n";

/// Expands a unit quad to a shape's bounds (plus one point for
/// antialiasing) and passes the position in points to the fragment shader.
pub const SHAPE_VERTEX: &str = r#"
in vec2 a_unit;
uniform vec2 u_canvas;   // target size in points
uniform vec4 u_rect;     // x, y, w, h in points, top-left origin
out vec2 v_pos;          // position in points
void main() {
    vec2 pad = vec2(1.0);
    vec2 p = u_rect.xy - pad + a_unit * (u_rect.zw + 2.0 * pad);
    v_pos = p;
    vec2 ndc = vec2(p.x / u_canvas.x * 2.0 - 1.0, 1.0 - p.y / u_canvas.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
}
"#;

/// Fills a rounded rectangle with a solid colour or a gradient. Output is
/// premultiplied; coverage comes from the rectangle's signed distance.
pub const SHAPE_FRAGMENT: &str = r#"
in vec2 v_pos;
uniform vec4 u_rect;
uniform float u_radius;
uniform float u_scale;       // pixels per point, for 1 px antialiasing
uniform int u_kind;          // 0 solid, 1 linear, 2 radial
uniform vec4 u_frame;        // gradient box in points
uniform vec4 u_geom;         // linear: x1 y1 x2 y2; radial: cx cy rx ry (box units)
uniform int u_count;
uniform int u_premul;        // 1: CSS (premultiplied) interpolation
uniform vec4 u_colors[8];
uniform float u_offsets[8];
uniform float u_opacity;
out vec4 o_color;

float sd_round_rect(vec2 p, vec2 center, vec2 half_size, float r) {
    vec2 q = abs(p - center) - half_size + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

vec4 premul(vec4 c) { return vec4(c.rgb * c.a, c.a); }

vec4 ramp(float t) {
    t = clamp(t, 0.0, 1.0);
    if (t <= u_offsets[0]) { return premul(u_colors[0]); }
    for (int i = 1; i < 8; i++) {
        if (i >= u_count) { break; }
        if (t <= u_offsets[i]) {
            float span = max(u_offsets[i] - u_offsets[i - 1], 1e-6);
            float f = (t - u_offsets[i - 1]) / span;
            if (u_premul == 1) {
                return mix(premul(u_colors[i - 1]), premul(u_colors[i]), f);
            }
            return premul(mix(u_colors[i - 1], u_colors[i], f));
        }
    }
    return premul(u_colors[u_count - 1]);
}

void main() {
    float r = min(u_radius, 0.5 * min(u_rect.z, u_rect.w));
    float d = sd_round_rect(v_pos, u_rect.xy + 0.5 * u_rect.zw, 0.5 * u_rect.zw, r);
    float coverage = clamp(0.5 - d * u_scale, 0.0, 1.0);
    vec4 color;
    if (u_kind == 0) {
        color = premul(u_colors[0]);
    } else {
        vec2 u = (v_pos - u_frame.xy) / u_frame.zw;
        float t;
        if (u_kind == 1) {
            vec2 a = u_geom.xy;
            vec2 ab = u_geom.zw - a;
            t = dot(u - a, ab) / max(dot(ab, ab), 1e-12);
        } else {
            t = length((u - u_geom.xy) / u_geom.zw);
        }
        color = ramp(t);
    }
    o_color = color * (coverage * u_opacity);
}
"#;

/// Vertex shader for tessellated meshes: point coordinates to clip space.
pub const MESH_VERTEX: &str = r#"
in vec2 a_pos;
uniform vec2 u_canvas;
out vec2 v_pos;
void main() {
    v_pos = a_pos;
    vec2 ndc = vec2(a_pos.x / u_canvas.x * 2.0 - 1.0, 1.0 - a_pos.y / u_canvas.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
}
"#;

/// Fragment shader for meshes: the same gradient ramp as the shape shader,
/// fully covering each triangle (no SDF), premultiplied, times u_opacity.
pub const MESH_FRAGMENT: &str = r#"
in vec2 v_pos;
uniform int u_kind;          // 0 solid, 1 linear, 2 radial
uniform vec4 u_frame;        // gradient box in points
uniform vec4 u_geom;         // linear: x1 y1 x2 y2; radial: cx cy rx ry (box units)
uniform int u_count;
uniform vec4 u_colors[8];
uniform float u_offsets[8];
uniform float u_opacity;
out vec4 o_color;

vec4 premul(vec4 c) { return vec4(c.rgb * c.a, c.a); }

vec4 ramp(float t) {
    t = clamp(t, 0.0, 1.0);
    if (t <= u_offsets[0]) { return premul(u_colors[0]); }
    for (int i = 1; i < 8; i++) {
        if (i >= u_count) { break; }
        if (t <= u_offsets[i]) {
            float span = max(u_offsets[i] - u_offsets[i - 1], 1e-6);
            float f = (t - u_offsets[i - 1]) / span;
            return premul(mix(u_colors[i - 1], u_colors[i], f));
        }
    }
    return premul(u_colors[u_count - 1]);
}

void main() {
    vec4 color;
    if (u_kind == 0) {
        color = premul(u_colors[0]);
    } else {
        vec2 u = (v_pos - u_frame.xy) / u_frame.zw;
        float t;
        if (u_kind == 1) {
            vec2 a = u_geom.xy;
            vec2 ab = u_geom.zw - a;
            t = dot(u - a, ab) / max(dot(ab, ab), 1e-12);
        } else {
            t = length((u - u_geom.xy) / u_geom.zw);
        }
        color = ramp(t);
    }
    o_color = color * u_opacity;
}
"#;

/// Vertex shader for full-screen passes (blur, composite, downsample).
pub const QUAD_VERTEX: &str = r#"
in vec2 a_unit;
out vec2 v_uv;
void main() {
    v_uv = a_unit;
    gl_Position = vec4(a_unit * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// Separable Gaussian blur of a premultiplied texture.
pub const BLUR_FRAGMENT: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
uniform vec2 u_dir;      // one texel step along the blur axis
uniform float u_sigma;   // in texels
uniform int u_radius;    // taps each side
out vec4 o_color;

const int RMAX = 72;
void main() {
    float inv = 1.0 / (2.0 * u_sigma * u_sigma);
    vec4 sum = vec4(0.0);
    float wsum = 0.0;
    for (int i = -RMAX; i <= RMAX; i++) {
        if (i < -u_radius || i > u_radius) { continue; }
        float w = exp(-float(i * i) * inv);
        sum += texture(u_tex, v_uv + u_dir * float(i)) * w;
        wsum += w;
    }
    o_color = sum / max(wsum, 1e-6);
}
"#;

/// Samples a premultiplied texture and scales it by an opacity, for
/// compositing a layer (blend mode is set with GL state).
pub const COMPOSITE_FRAGMENT: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
uniform float u_opacity;
out vec4 o_color;
void main() {
    o_color = texture(u_tex, v_uv) * u_opacity;
}
"#;
