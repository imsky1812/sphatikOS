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

/// Copies a texture to the bound target (used to seed the scene and to
/// downsample the backdrop for blurring).
pub const COPY_FRAGMENT: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
out vec4 o_color;
void main() { o_color = texture(u_tex, v_uv); }
"#;

/// One dual-Kawase-style blur pass: average of four diagonal taps.
pub const KAWASE_FRAGMENT: &str = r#"
in vec2 v_uv;
uniform sampler2D u_tex;
uniform vec2 u_offset;   // texel offset for this pass
out vec4 o_color;
void main() {
    vec4 c = texture(u_tex, v_uv + vec2( u_offset.x,  u_offset.y));
    c += texture(u_tex, v_uv + vec2( u_offset.x, -u_offset.y));
    c += texture(u_tex, v_uv + vec2(-u_offset.x,  u_offset.y));
    c += texture(u_tex, v_uv + vec2(-u_offset.x, -u_offset.y));
    o_color = c * 0.25;
}
"#;

/// A rounded glass panel over a blurred backdrop, with edge refraction, the
/// material gradient fill, inner rim highlights, a specular sheen that follows
/// a light position, and a prism-tinted outer rim (glass v1, WP 2.5). Models
/// the prototype's lens-light `.glass` treatment (reference section 3).
pub const GLASS_VERTEX: &str = r#"
in vec2 a_unit;
uniform vec2 u_canvas;
uniform vec4 u_rect;     // x, y, w, h in points
uniform vec2 u_pad;      // quad padding around the rect, points
out vec2 v_pos;          // position in points
out vec2 v_local;        // 0..1 across the rect
void main() {
    vec2 pad = u_pad;
    vec2 p = u_rect.xy - pad + a_unit * (u_rect.zw + 2.0 * pad);
    v_pos = p;
    v_local = (p - u_rect.xy) / u_rect.zw;
    vec2 ndc = vec2(p.x / u_canvas.x * 2.0 - 1.0, 1.0 - p.y / u_canvas.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
}
"#;

/// Fragment shader for a glass panel (glass v1).
pub const GLASS_FRAGMENT: &str = r#"
in vec2 v_pos;
in vec2 v_local;
uniform sampler2D u_backdrop;   // blurred wallpaper (bottom-up)
uniform vec2 u_canvas;          // points
uniform vec4 u_rect;
uniform float u_radius;
uniform float u_scale;          // pixels per point
uniform vec2 u_light;           // specular spot, 0..1 across the panel
uniform vec4 u_frame;           // fill gradient box in points
uniform vec4 u_geom;            // fill linear from/to in box units
uniform int u_count;
uniform int u_premul;
uniform vec4 u_colors[8];
uniform float u_offsets[8];
out vec4 o_color;

const float REFRACT_WIDTH = 16.0;  // how far in the edge bend reaches, points
const float REFRACT_PX = 6.0;      // peak inward bend, points
const float RING = 1.0;            // prism rim half-width, points

float sd_round_rect(vec2 p, vec2 c, vec2 h, float r) {
    vec2 q = abs(p - c) - h + vec2(r);
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
            if (u_premul == 1) { return mix(premul(u_colors[i - 1]), premul(u_colors[i]), f); }
            return premul(mix(u_colors[i - 1], u_colors[i], f));
        }
    }
    return premul(u_colors[u_count - 1]);
}

vec3 backdrop_at(vec2 pos) {
    vec2 uv = vec2(pos.x / u_canvas.x, 1.0 - pos.y / u_canvas.y);
    return texture(u_backdrop, uv).rgb;
}

// The prototype's ::after rim gradient along the 155-degree diagonal.
vec4 prism(float s) {
    s = clamp(s, 0.0, 1.0);
    if (s < 0.30) return mix(vec4(1.0, 1.0, 1.0, 0.90), vec4(1.0, 1.0, 1.0, 0.12), s / 0.30);
    if (s < 0.55) return mix(vec4(1.0, 1.0, 1.0, 0.12), vec4(1.0, 1.0, 1.0, 0.04), (s - 0.30) / 0.25);
    if (s < 0.80) return mix(vec4(1.0, 1.0, 1.0, 0.04), vec4(0.667, 0.922, 1.0, 0.35), (s - 0.55) / 0.25);
    return mix(vec4(0.667, 0.922, 1.0, 0.35), vec4(1.0, 0.784, 0.922, 0.60), (s - 0.80) / 0.20);
}

void main() {
    vec2 c = u_rect.xy + 0.5 * u_rect.zw;
    vec2 h = 0.5 * u_rect.zw;
    float r = min(u_radius, min(h.x, h.y));
    float d = sd_round_rect(v_pos, c, h, r);     // negative inside
    float aa = 1.0 / u_scale;
    float coverage = clamp(0.5 - d * u_scale, 0.0, 1.0);
    if (coverage <= 0.0) { discard; }

    // Outward normal of the rounded rect (finite differences).
    vec2 e = vec2(1.0, 0.0);
    vec2 n = normalize(vec2(
        sd_round_rect(v_pos + e.xy, c, h, r) - sd_round_rect(v_pos - e.xy, c, h, r),
        sd_round_rect(v_pos + e.yx, c, h, r) - sd_round_rect(v_pos - e.yx, c, h, r)
    ) + 1e-6);

    // Edge refraction: bend the backdrop inward near the rim, with a small
    // red/blue split for dispersion.
    float inside = max(-d, 0.0);
    float edge = 1.0 - clamp(inside / REFRACT_WIDTH, 0.0, 1.0);
    edge *= edge;
    vec2 bend = -n * edge * REFRACT_PX;
    vec3 backdrop;
    backdrop.r = backdrop_at(v_pos + bend * 1.08).r;
    backdrop.g = backdrop_at(v_pos + bend).g;
    backdrop.b = backdrop_at(v_pos + bend * 0.92).b;

    // Material gradient fill over the blurred, refracted backdrop.
    vec2 u = (v_pos - u_frame.xy) / u_frame.zw;
    vec2 ab = u_geom.zw - u_geom.xy;
    float tg = dot(u - u_geom.xy, ab) / max(dot(ab, ab), 1e-12);
    vec4 fill = ramp(tg);
    vec3 rgb = fill.rgb + backdrop * (1.0 - fill.a);

    // Inner glows (the prototype's inset box-shadows): a soft band inside the
    // top edge and a fainter one inside the bottom. Kept gentle so the glass
    // stays translucent rather than plastic.
    float top = v_pos.y - u_rect.y;
    float bottom = (u_rect.y + u_rect.w) - v_pos.y;
    rgb += 0.24 * (1.0 - smoothstep(0.0, 12.0, top));
    rgb += 0.12 * (1.0 - smoothstep(0.0, 14.0, bottom));

    // Specular sheen: a soft radial spot at the light plus a faint diagonal
    // band, both subtle.
    float spot = 1.0 - smoothstep(0.0, 0.65, length(v_local - u_light));
    rgb += 0.13 * spot;
    float band = dot(v_local, normalize(vec2(0.9, 0.42)));
    rgb += 0.05 * (1.0 - smoothstep(0.04, 0.24, abs(band - 0.42)));

    // Crisp top hairline (inset 0 1px 0): a thin bright line along the top edge.
    rgb += 0.40 * (1.0 - smoothstep(0.0, 1.6, top));

    // Prism rim: a thin 155-degree tinted ring just inside the edge, softened.
    float ring = (1.0 - smoothstep(0.0, RING + aa, abs(d))) * 0.6;
    float s = dot(v_local, normalize(vec2(0.42, 0.9)));
    vec4 rim = prism(s);
    rgb = mix(rgb, rgb * (1.0 - rim.a) + rim.rgb * rim.a, ring);

    o_color = vec4(min(rgb, vec3(1.0)), 1.0) * coverage;
}
"#;

/// A soft drop shadow for a floating glass panel (the prototype's outer
/// box-shadow). Drawn before the panel, offset downward.
pub const SHADOW_FRAGMENT: &str = r#"
in vec2 v_pos;
in vec2 v_local;
uniform vec4 u_rect;
uniform float u_radius;
uniform float u_scale;
uniform float u_offset;    // downward shift, points
uniform float u_spread;    // feather, points
uniform float u_alpha;     // peak opacity
out vec4 o_color;
float sd_round_rect(vec2 p, vec2 c, vec2 h, float r) {
    vec2 q = abs(p - c) - h + vec2(r);
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}
void main() {
    vec2 c = u_rect.xy + 0.5 * u_rect.zw + vec2(0.0, u_offset);
    // The prototype's shadow is spread inward (-20 px), so it is narrower than
    // the panel; inset the shape a little.
    vec2 h = max(0.5 * u_rect.zw - vec2(10.0), vec2(1.0));
    float r = min(u_radius, min(h.x, h.y));
    float d = sd_round_rect(v_pos, c, h, r);
    float a = u_alpha * (1.0 - smoothstep(0.0, u_spread, max(d, 0.0)));
    o_color = vec4(0.0157, 0.0118, 0.0706, 1.0) * a;  // rgba(4,3,18,.7)-ish, premultiplied
}
"#;

/// Flat per-vertex-colour shader for the debug overlay (pixel coordinates).
pub const OVERLAY_VERTEX: &str = r#"
in vec2 a_pos;      // pixels, top-left origin
in vec4 a_color;    // premultiplied
uniform vec2 u_size;
out vec4 v_color;
void main() {
    v_color = a_color;
    vec2 ndc = vec2(a_pos.x / u_size.x * 2.0 - 1.0, 1.0 - a_pos.y / u_size.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
}
"#;

/// Fragment for the overlay: the interpolated premultiplied colour.
pub const OVERLAY_FRAGMENT: &str = r#"
in vec4 v_color;
out vec4 o_color;
void main() { o_color = v_color; }
"#;
