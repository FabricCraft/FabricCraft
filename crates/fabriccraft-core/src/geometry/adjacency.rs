use std::collections::BTreeMap;

use super::{Face, VertexId};

/// For each directed triangle edge, return the face index and edge slot on
/// the other side. The input belongs to a validated, consistently wound surface.
pub(crate) fn face_neighbors(faces: &[Face]) -> Vec<[Option<(usize, usize)>; 3]> {
    let mut neighbors = vec![[None; 3]; faces.len()];
    let mut edges = BTreeMap::<(VertexId, VertexId), (usize, usize)>::new();
    for (face, triangle) in faces.iter().enumerate() {
        let [a, b, c] = triangle.vertices;
        for (slot, edge) in [(a, b), (b, c), (c, a)].into_iter().enumerate() {
            if let Some((other, other_slot)) = edges.remove(&(edge.1, edge.0)) {
                neighbors[face][slot] = Some((other, other_slot));
                neighbors[other][other_slot] = Some((face, slot));
            } else {
                edges.insert(edge, (face, slot));
            }
        }
    }
    neighbors
}
