// Instanced HDL v0 WGSL. Host CPU packs quads; GPU does not walk opcodes.
// kind: 0 fill (rounded-rect SDF), 1 glyph (nearest atlas), 2 image, 3 diagonal line.
// Matches site/src/lib/shaders/hdlInstanced.ts.

struct Viewport {
  size: vec2f,
  fb: vec2f,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(1) @binding(0) var nearestSamp: sampler;
@group(1) @binding(1) var atlas0: texture_2d<f32>;
@group(1) @binding(2) var atlas1: texture_2d<f32>;
@group(1) @binding(3) var tex0: texture_2d<f32>;
@group(1) @binding(4) var tex1: texture_2d<f32>;
@group(1) @binding(5) var tex2: texture_2d<f32>;

struct VsIn {
  @builtin(vertex_index) vid: u32,
  @location(0) rect: vec4f,
  @location(1) color: vec4f,
  @location(2) uvRect: vec4f,
  @location(3) params: vec4f,
};

struct VsOut {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
  @location(1) uv: vec2f,
  @location(2) local: vec2f,
  @location(3) rect: vec4f,
  @location(4) params: vec4f,
};

fn cornerOf(vid: u32) -> vec2f {
  var c = array<vec2f, 6>(
    vec2f(0.0, 0.0),
    vec2f(1.0, 0.0),
    vec2f(0.0, 1.0),
    vec2f(1.0, 0.0),
    vec2f(1.0, 1.0),
    vec2f(0.0, 1.0),
  );
  return c[vid];
}

fn sdRoundBox(p: vec2f, b: vec2f, r: f32) -> f32 {
  let q = abs(p) - b + vec2f(r, r);
  return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn distToSeg(p: vec2f, a: vec2f, b: vec2f) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let denom = max(dot(ba, ba), 1.0e-8);
  let t = clamp(dot(pa, ba) / denom, 0.0, 1.0);
  return length(pa - ba * t);
}

fn sampleAtlas(id: u32, uv: vec2f) -> f32 {
  if (id == 1u) {
    return textureSampleLevel(atlas1, nearestSamp, uv, 0.0).a;
  }
  return textureSampleLevel(atlas0, nearestSamp, uv, 0.0).a;
}

fn sampleImage(id: u32, uv: vec2f) -> vec4f {
  if (id == 1u) {
    return textureSampleLevel(tex1, nearestSamp, uv, 0.0);
  }
  if (id == 2u) {
    return textureSampleLevel(tex2, nearestSamp, uv, 0.0);
  }
  return textureSampleLevel(tex0, nearestSamp, uv, 0.0);
}

@vertex
fn vs_main(input: VsIn) -> VsOut {
  var out: VsOut;
  let corner = cornerOf(input.vid);
  let kind = input.params.y;
  var px: vec2f;
  if (kind > 2.5) {
    let p0 = input.rect.xy;
    let p1 = input.rect.zw;
    let hw = input.params.x * 0.5 + 1.0;
    let bbMin = min(p0, p1) - vec2f(hw, hw);
    let bbMax = max(p0, p1) + vec2f(hw, hw);
    px = mix(bbMin, bbMax, corner);
  } else {
    px = input.rect.xy + corner * input.rect.zw;
  }
  let fbSize = max(viewport.fb, vec2f(1.0, 1.0));
  let logical = max(viewport.size, vec2f(1.0, 1.0));
  let fbpx = px * (fbSize / logical);
  let ndc = vec2f(
    (fbpx.x / fbSize.x) * 2.0 - 1.0,
    1.0 - (fbpx.y / fbSize.y) * 2.0,
  );
  out.position = vec4f(ndc, 0.0, 1.0);
  out.color = input.color;
  out.uv = mix(input.uvRect.xy, input.uvRect.zw, corner);
  out.local = corner;
  out.rect = input.rect;
  out.params = input.params;
  return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4f {
  let kind = u32(input.params.y + 0.5);
  let res = u32(input.params.z + 0.5);
  let fbSize = max(viewport.fb, vec2f(1.0, 1.0));
  let logicalSize = max(viewport.size, vec2f(1.0, 1.0));
  let scale = fbSize / logicalSize;
  let logicalPx = input.position.xy / scale;

  if (kind == 1u) {
    let cov = sampleAtlas(res, input.uv);
    if (cov < 0.5) {
      discard;
    }
    return vec4f(input.color.rgb, input.color.a * cov);
  }

  if (kind == 2u) {
    return sampleImage(res, input.uv);
  }

  if (kind == 3u) {
    let d = distToSeg(logicalPx, input.rect.xy, input.rect.zw);
    if (d > input.params.x * 0.5) {
      discard;
    }
    return input.color;
  }

  let radius = input.params.x;
  if (radius > 0.0) {
    let origin = input.rect.xy;
    let size = input.rect.zw;
    let half = size * 0.5;
    let center = origin + half;
    let r = min(radius, min(half.x, half.y));
    if (sdRoundBox(logicalPx - center, half, r) > 0.0) {
      discard;
    }
  }
  return input.color;
}
