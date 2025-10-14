struct MandelbrotParams {
    center: vec2<f32>,
    zoom: f32,
    max_iterations: u32,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(1)
var<uniform> params: MandelbrotParams;

fn mandelbrot_iterations(c: vec2<f32>, max_iter: u32) -> u32 {
    var z = vec2<f32>(0.0, 0.0);
    var iter = 0u;

    for (var i = 0u; i < max_iter; i = i + 1u) {
        // z = z^2 + c
        let z_real = z.x * z.x - z.y * z.y + c.x;
        let z_imag = 2.0 * z.x * z.y + c.y;
        z = vec2<f32>(z_real, z_imag);

        // Check if |z|^2 > 4 (diverged)
        if (z.x * z.x + z.y * z.y) > 4.0 {
            iter = i;
            break;
        }
    }

    return iter;
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let x = c * (1.0 - abs((h * 6.0) % 2.0 - 1.0));
    let m = v - c;

    var rgb = vec3<f32>(0.0);

    if h < 1.0 / 6.0 {
        rgb = vec3<f32>(c, x, 0.0);
    }
    else if h < 2.0 / 6.0 {
        rgb = vec3<f32>(x, c, 0.0);
    }
    else if h < 3.0 / 6.0 {
        rgb = vec3<f32>(0.0, c, x);
    }
    else if h < 4.0 / 6.0 {
        rgb = vec3<f32>(0.0, x, c);
    }
    else if h < 5.0 / 6.0 {
        rgb = vec3<f32>(x, 0.0, c);
    }
    else {
        rgb = vec3<f32>(c, 0.0, x);
    }

    return rgb + vec3<f32>(m);
}

@compute @workgroup_size(8, 8)
fn compute(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let xy = global_id.xy;
    let dims = textureDimensions(output_texture);

    // Check bounds
    if xy.x >= dims.x || xy.y >= dims.y {
        return;
    }

    // Convert pixel coordinates to complex plane coordinates
    let pixel_pos = vec2<f32>(xy);
    let dims_f = vec2<f32>(dims);

    // Normalize to [-1, 1] and then scale by zoom around center
    let normalized = (pixel_pos / dims_f) * 2.0 - 1.0;
    let aspect_ratio = dims_f.x / dims_f.y;
    let scaled = vec2<f32>(normalized.x * aspect_ratio, normalized.y) / params.zoom;
    let c = scaled + params.center;

    // Calculate Mandelbrot iterations
    let iterations = mandelbrot_iterations(c, params.max_iterations);

    var color = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    if iterations < params.max_iterations {
        // Color based on iteration count using HSV
        let hue = f32(iterations) / f32(params.max_iterations);
        let saturation = 1.0;
        let value = 1.0;
        let rgb = hsv_to_rgb(hue, saturation, value);
        color = vec4<f32>(rgb, 1.0);
    }

    // Write the final color to output texture
    textureStore(output_texture, vec2<i32>(xy), color);
}
