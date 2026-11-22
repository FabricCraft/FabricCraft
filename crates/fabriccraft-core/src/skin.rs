//! Construct a fabric surface from an ordered boundary in reference space.

use std::collections::BTreeSet;
use std::fmt;

use crate::{
    geometry::{
        Axis, Face, FaceId, GeometryError, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId,
        Vertex, VertexId, face_neighbors,
    },
    planar::{MAX_BOUNDARY_EDGES, MAX_COORDINATE_MM, cross, sub, turn, validate_outline},
};

mod boundary;
pub use boundary::Boundary;

fn side_axis(side: Axis) -> (usize, bool) {
    match side {
        Axis::X => (0, true),
        Axis::Y => (1, true),
        Axis::Z => (2, true),
        Axis::NegativeX => (0, false),
        Axis::NegativeY => (1, false),
        Axis::NegativeZ => (2, false),
    }
}

fn project(p: [f64; 3], axis: usize) -> [f64; 2] {
    [p[(axis + 1) % 3], p[(axis + 2) % 3]]
}

/// Suggest corner indices from the convex footprint seen along the given axis.
/// For coincident projected positions, prefer the most outward 3D point.
pub fn suggest_boundary(points: &[[f64; 3]], side: Axis) -> Result<Vec<usize>, SkinError> {
    check_coordinates(points)?;
    let (depth, positive) = side_axis(side);
    let projected: Vec<_> = points
        .iter()
        .map(|&p| project(p, depth).map(|value| if value == 0. { 0. } else { value }))
        .collect();
    let mut order: Vec<_> = (0..points.len()).collect();
    let height = |i: usize| {
        if positive {
            points[i][depth]
        } else {
            -points[i][depth]
        }
    };
    order.sort_by(|&i, &j| {
        projected[i][0]
            .total_cmp(&projected[j][0])
            .then_with(|| projected[i][1].total_cmp(&projected[j][1]))
            .then_with(|| height(j).total_cmp(&height(i)))
            .then_with(|| i.cmp(&j))
    });
    order.dedup_by(|i, j| projected[*i] == projected[*j]);

    // Andrew's monotone chain. Collinear points on the footprint stay available
    // for manual selection while the suggestion contains the exterior corners.
    let chain = |indices: &mut dyn Iterator<Item = usize>| {
        let mut result: Vec<usize> = Vec::new();
        for i in indices {
            while let [.., a, b] = result.as_slice() {
                if turn(projected[*a], projected[*b], projected[i]) > 0. {
                    break;
                }
                result.pop();
            }
            result.push(i);
        }
        result.pop();
        result
    };
    let mut result = chain(&mut order.iter().copied());
    result.extend(chain(&mut order.iter().rev().copied()));
    if result.len() < 3 {
        return Err(SkinError::Projection);
    }
    if result.len() > MAX_BOUNDARY_EDGES {
        return Err(SkinError::TooManyAnchors);
    }
    if !positive {
        result[1..].reverse();
    }
    Ok(result)
}

/// Triangulate a simple projected perimeter and lift its triangles back to the
/// selected 3D points. Faces can span open space between supports.
pub fn build_skin(anchors: &[[f64; 3]]) -> Result<Surface, SkinError> {
    triangulate_skin(anchors, None)
}

fn triangulate_skin(anchors: &[[f64; 3]], projection: Option<usize>) -> Result<Surface, SkinError> {
    if anchors.len() < 3 {
        return Err(SkinError::TooFewAnchors);
    }
    if anchors.len() > MAX_BOUNDARY_EDGES {
        return Err(SkinError::TooManyAnchors);
    }
    check_coordinates(anchors)?;
    let offsets: Vec<_> = anchors
        .iter()
        .map(|&point| sub(point, anchors[0]))
        .collect();
    let mut normal = [0.; 3];
    for (&a, &b) in offsets.iter().zip(offsets.iter().cycle().skip(1)) {
        for (sum, part) in normal.iter_mut().zip(cross(a, b)) {
            *sum += part;
        }
    }
    let [x, y, z] = normal.map(f64::abs);
    let axis = projection.unwrap_or(if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    });
    if normal[axis] == 0. {
        return Err(SkinError::Projection);
    }
    let projected: Vec<_> = offsets.iter().map(|&p| project(p, axis)).collect();
    validate_outline(&projected).map_err(|_| SkinError::Projection)?;

    let mut indices: Vec<usize> = Vec::new();
    earcut::Earcut::new().earcut(projected.iter().copied(), &[], &mut indices);
    if indices.len() != 3 * (anchors.len() - 2) {
        return Err(SkinError::Triangulation);
    }
    let mut faces = Vec::with_capacity(anchors.len() - 2);
    for (id, &[a, b, c]) in indices.as_chunks::<3>().0.iter().enumerate() {
        if [a, b, c].iter().any(|&i| i >= anchors.len()) {
            return Err(SkinError::Triangulation);
        }
        let sign = turn(projected[a], projected[b], projected[c]);
        if sign == 0. {
            return Err(SkinError::Triangulation);
        }
        let corners = if sign.is_sign_positive() == normal[axis].is_sign_positive() {
            [a, b, c]
        } else {
            [a, c, b]
        };
        faces.push(Face {
            id: FaceId(id as u64),
            vertices: corners.map(|i| VertexId(i as u64)),
        });
    }
    let vertices = anchors
        .iter()
        .enumerate()
        .map(|(i, &position)| Vertex {
            id: VertexId(i as u64),
            position,
        })
        .collect();
    let skin = Surface::import(
        SurfaceId(1),
        Provenance {
            source: "FabricCraft fabric boundary".into(),
            unit: LengthUnit::Millimetre,
            frame: ImportFrame::IDENTITY,
        },
        vertices,
        faces,
    )
    .map_err(SkinError::Geometry)?;
    let faces = &skin.record().faces;
    let boundary: BTreeSet<_> = face_neighbors(faces)
        .into_iter()
        .zip(faces)
        .flat_map(|(neighbors, face)| {
            neighbors
                .into_iter()
                .enumerate()
                .filter_map(|(slot, next)| {
                    next.is_none()
                        .then_some((face.vertices[slot], face.vertices[(slot + 1) % 3]))
                })
        })
        .collect();
    let expected: BTreeSet<_> = (0..anchors.len())
        .map(|i| {
            (
                VertexId(i as u64),
                VertexId(((i + 1) % anchors.len()) as u64),
            )
        })
        .collect();
    if boundary != expected {
        return Err(SkinError::Triangulation);
    }
    Ok(skin)
}

fn check_coordinates(points: &[[f64; 3]]) -> Result<(), SkinError> {
    if points
        .iter()
        .flatten()
        .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_MM)
    {
        return Err(SkinError::CoordinateRange);
    }
    Ok(())
}

#[derive(Debug)]
pub enum SkinError {
    TooFewAnchors,
    TooManyAnchors,
    CoordinateRange,
    Projection,
    BoundaryEdit,
    Triangulation,
    Geometry(GeometryError),
}

impl fmt::Display for SkinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewAnchors => write!(
                f,
                "Choose at least three points around the fabric boundary."
            ),
            Self::TooManyAnchors => write!(
                f,
                "The fabric boundary can contain up to {MAX_BOUNDARY_EDGES} points."
            ),
            Self::CoordinateRange => write!(
                f,
                "The fabric coordinates must be finite and within ±{MAX_COORDINATE_MM} mm."
            ),
            Self::Projection => write!(
                f,
                "Choose a boundary that projects to a single outline without repeated or crossing points."
            ),
            Self::BoundaryEdit => write!(
                f,
                "This pick would cross the outline, overlap a point in the coverage view, or break a selected edge. Try another support or release an edge."
            ),
            Self::Triangulation => write!(
                f,
                "The triangulation could not preserve every selected boundary segment. Try adjusting the selection."
            ),
            Self::Geometry(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SkinError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Geometry(error) => Some(error),
            _ => None,
        }
    }
}
