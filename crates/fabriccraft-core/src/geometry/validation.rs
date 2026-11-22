use std::collections::{BTreeMap, BTreeSet};

use super::{GeometryError, SurfaceRecord, VertexId};

pub(super) fn validate(record: &SurfaceRecord) -> Result<(), GeometryError> {
    if record.provenance.source.trim().is_empty() {
        return Err(GeometryError::MissingSource);
    }
    if record.faces.is_empty() {
        return Err(GeometryError::EmptySurface);
    }
    // Check duplicate IDs before inspecting values with the same sort key.
    for pair in record.vertices_mm.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(GeometryError::DuplicateVertex(pair[0].id));
        }
    }
    for pair in record.faces.windows(2) {
        if pair[0].id == pair[1].id {
            return Err(GeometryError::DuplicateFace(pair[0].id));
        }
    }
    for vertex in &record.vertices_mm {
        if !vertex.position.iter().all(|v| v.is_finite()) {
            return Err(GeometryError::NonFinitePosition(vertex.id));
        }
    }

    let mut triangles = BTreeMap::new();
    let mut edges = BTreeMap::<_, EdgeUse>::new();
    // In a vertex's link, each incident triangle joins its two other vertices.
    let mut links = BTreeMap::<VertexId, BTreeMap<VertexId, Vec<VertexId>>>::new();
    for face in &record.faces {
        let [a, b, c] = face.vertices;
        if a == b || b == c || c == a {
            return Err(GeometryError::RepeatedCorner(face.id));
        }
        let mut corners = face.vertices;
        corners.sort_unstable();
        let mut points = [[0.; 3]; 3];
        for (point, id) in points.iter_mut().zip(corners) {
            let index = record
                .vertices_mm
                .binary_search_by_key(&id, |v| v.id)
                .map_err(|_| GeometryError::MissingVertex {
                    face: face.id,
                    vertex: id,
                })?;
            *point = record.vertices_mm[index].position;
        }
        if is_degenerate(points) {
            return Err(GeometryError::DegenerateFace(face.id));
        }
        if let Some(previous) = triangles.insert(corners, face.id) {
            return Err(GeometryError::DuplicateTriangle(previous, face.id));
        }
        for (v, from, to) in [(a, b, c), (b, c, a), (c, a, b)] {
            let edge = edges.entry([from.min(to), from.max(to)]).or_default();
            edge.count += 1;
            edge.forward += usize::from(from < to);

            let link = links.entry(v).or_default();
            link.entry(from).or_default().push(to);
            link.entry(to).or_default().push(from);
        }
    }

    for (edge, usage) in edges {
        if usage.count > 2 {
            return Err(GeometryError::NonManifoldEdge(edge));
        }
        if usage.count == 2 && usage.forward != 1 {
            return Err(GeometryError::InconsistentWinding(edge));
        }
    }
    for (vertex, link) in links {
        let mut seen = BTreeSet::new();
        let mut pending: Vec<_> = link.keys().next().copied().into_iter().collect();
        while let Some(next) = pending.pop() {
            if seen.insert(next) {
                pending.extend(&link[&next]);
            }
        }
        if seen.len() != link.len() {
            return Err(GeometryError::DisconnectedFan(vertex));
        }
    }
    Ok(())
}

#[derive(Default)]
struct EdgeUse {
    count: usize,
    forward: usize,
}

fn is_degenerate(points: [[f64; 3]; 3]) -> bool {
    fn edges([a, b, c]: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
        [(a, b), (a, c), (b, c)].map(|(p, q)| std::array::from_fn(|i| q[i] - p[i]))
    }
    let mut sides = edges(points);
    // A difference between opposite finite extremes may overflow. Halving all
    // positions in that case preserves the relative-area test.
    if sides.iter().flatten().any(|x| !x.is_finite()) {
        sides = edges(points.map(|p| p.map(|x| x * 0.5)));
    }
    let span = sides.iter().flatten().map(|x| x.abs()).fold(0., f64::max);
    if span == 0. {
        return true;
    }
    let normalized = sides.map(|s| s.map(|x| x / span));
    let longest_squared = normalized
        .iter()
        .map(|side| side.iter().map(|value| value * value).sum::<f64>())
        .fold(0., f64::max);
    let [u, v, _] = normalized;
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    cross[0].hypot(cross[1]).hypot(cross[2]) <= f64::EPSILON * longest_squared
}
