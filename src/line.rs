use crate::fragment::Fragment;
use crate::vertex::Vertex;
use crate::color::Color;

pub fn line(a: &Vertex, b: &Vertex) -> Vec<Fragment> {
    let mut fragments = Vec::new();

    let start = a.transformed_position;
    let end = b.transformed_position;

    // Cast with rounding to avoid subpixel jitter when values are near .5
    let mut x0 = start.x.round() as i32;
    let mut y0 = start.y.round() as i32;
    let x1 = end.x.round() as i32;
    let y1 = end.y.round() as i32;

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();

    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut err = if dx > dy { dx / 2 } else { -dy / 2 };

    let denom = if dx >= dy {
        (x1 - start.x as i32) as f32
    } else {
        (y1 - start.y as i32) as f32
    };
    let denom = if denom.abs() < f32::EPSILON { 1.0 } else { denom };

    loop {
        // progress along the dominant axis
        let num = if dx >= dy {
            (x0 - start.x as i32) as f32
        } else {
            (y0 - start.y as i32) as f32
        };
        let t = (num / denom).clamp(0.0, 1.0);
        let z = start.z + (end.z - start.z) * t;

        // color is ignored by the current pipeline; any value is fine
        fragments.push(Fragment::new(x0 as f32, y0 as f32, Color::black(), z));

        if x0 == x1 && y0 == y1 { break; }

        let e2 = err;
        if e2 > -dx { err -= dy; x0 += sx; }
        if e2 <  dy { err += dx; y0 += sy; }
    }

    fragments
}