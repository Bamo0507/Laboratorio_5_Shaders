use nalgebra_glm::{Vec3, Vec4, Mat3};
use crate::vertex::Vertex;
use crate::Uniforms;

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    // ---- World position (Model) ----
    let p = Vec4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let world = uniforms.model_matrix * p;

    // ---- View & Projection ----
    let view  = uniforms.view_matrix * world;
    let clip  = uniforms.proj_matrix * view;

    // Perspective divide → NDC
    let w = clip.w.max(1e-6); // avoid div by zero
    let ndc = Vec3::new(clip.x / w, clip.y / w, clip.z / w); // x,y,z in [-1,1]

    // Viewport mapping to pixels
    let (sw, sh) = uniforms.screen_size;
    let sx = (ndc.x * 0.5 + 0.5) * sw;          // [0, sw]
    let sy = (1.0 - (ndc.y * 0.5 + 0.5)) * sh;  // flip Y → [0, sh]
    let sz = ndc.z;                              // depth in [-1,1] (near=-1 is closer)

    // Normal: only model rotation/scale (keep in same space as your light vectors)
    let m = &uniforms.model_matrix;
    let model_mat3 = Mat3::new(
        m[(0,0)], m[(0,1)], m[(0,2)],
        m[(1,0)], m[(1,1)], m[(1,2)],
        m[(2,0)], m[(2,1)], m[(2,2)],
    );
    let mut n = model_mat3 * vertex.normal;
    if n.magnitude() > 0.0 { n = n.normalize(); }

    Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: vertex.color,
        transformed_position: Vec3::new(sx, sy, sz),
        transformed_normal: n,
    }
}