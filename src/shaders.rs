use nalgebra_glm::{Vec3, Vec4, Mat3};
use crate::vertex::Vertex;
use crate::Uniforms;

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
  // Transform position
  let position = Vec4::new(
    vertex.position.x,
    vertex.position.y,
    vertex.position.z,
    1.0
  );
  
  // Apply model, view, and projection transformations
  let model_view = uniforms.view_matrix * uniforms.model_matrix;
  let mvp = uniforms.projection_matrix * model_view;
  let transformed = mvp * position;

  // Perform perspective division
  let w = transformed.w;
  let ndc_position = Vec3::new(
    transformed.x / w,
    transformed.y / w,
    transformed.z / w
  );
  
  // Convert NDC to screen coordinates
  let screen_position = Vec3::new(
    (ndc_position.x + 1.0) * 600.0,  // Asumiendo viewport width de 1200
    (1.0 - ndc_position.y) * 400.0,  // Asumiendo viewport height de 800
    ndc_position.z
  );

  // Transform normal
  let model_mat3 = Mat3::new(
    uniforms.model_matrix[0], uniforms.model_matrix[1], uniforms.model_matrix[2],
    uniforms.model_matrix[4], uniforms.model_matrix[5], uniforms.model_matrix[6],
    uniforms.model_matrix[8], uniforms.model_matrix[9], uniforms.model_matrix[10]
  );
  let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());

  let transformed_normal = normal_matrix * vertex.normal;

  // Create a new Vertex with transformed attributes
  Vertex {
    position: vertex.position,
    normal: vertex.normal,
    tex_coords: vertex.tex_coords,
    color: vertex.color,
    transformed_position: screen_position,
    transformed_normal,
  }
}