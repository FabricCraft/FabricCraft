//! A dimensioned outline for one planar mesh patch.

use std::collections::BTreeMap;
use std::fmt;

use robust::{Coord, orient2d};

use crate::geometry::{FaceId, GeometryError, Surface, VertexId};

mod allowance;
mod patches;

pub use allowance::{AllowanceError, CutPattern};
pub use patches::{PlanarPatch, planar_patches};

const PLANAR_TOLERANCE_MM: f64 = 0.001;
pub(crate) const MAX_COORDINATE_MM: f64 = 1.0e9;
pub(crate) const MAX_BOUNDARY_EDGES: usize = 4096;

#[derive(Debug)]
pub struct PlanarPanel {
    outline: Vec<[f64; 2]>,
    outline_ids: Vec<VertexId>,
    size: [f64; 2],
}

impl PlanarPanel {
    /// Project a consistently wound patch into a right-handed panel frame.
    /// The longest boundary edge fixes the horizontal axis. Mesh vertices must
    /// be within 0.001 mm of the reference plane.
    pub fn from_surface(surface: &Surface) -> Result<Self, PanelError> {
        let record = surface.record();
        let vertices: BTreeMap<_, _> = record
            .vertices_mm
            .iter()
            .map(|v| (v.id, v.position))
            .collect();
        let mut used = BTreeMap::new();
        let mut edges = BTreeMap::new();
        for face in &record.faces {
            let [a, b, c] = face.vertices;
            for (from, to) in [(a, b), (b, c), (c, a)] {
                used.insert(from, vertices[&from]);
                // Surface validation guarantees at most two opposite uses. A
                // second use cancels an interior edge.
                if edges.remove(&(to, from)).is_none() {
                    edges.insert((from, to), ());
                }
            }
        }
        if used.values().flatten().any(|v| v.abs() > MAX_COORDINATE_MM) {
            return Err(PanelError::CoordinateRange);
        }

        let plane = record
            .faces
            .iter()
            .rev()
            .filter_map(|face| Plane::through(face.vertices.map(|id| used[&id])))
            .max_by(|a, b| a.twice_area.total_cmp(&b.twice_area))
            .ok_or(PanelError::ProjectionPrecision)?;
        let normal = plane.normal;
        for (&vertex, &position) in &used {
            let distance_mm = plane.distance(position);
            if distance_mm > PLANAR_TOLERANCE_MM {
                return Err(PanelError::NonPlanar {
                    vertex,
                    distance_mm,
                });
            }
        }

        let boundary = boundary_loop(edges)?;
        let mut horizontal = [0.; 3];
        let mut longest = 0.;
        for i in 0..boundary.len() {
            let edge = sub(
                used[&boundary[(i + 1) % boundary.len()]],
                used[&boundary[i]],
            );
            let perpendicular = dot(edge, normal);
            let in_plane = sub(edge, normal.map(|n| n * perpendicular));
            let length = norm(in_plane);
            if length > longest {
                horizontal = in_plane;
                longest = length;
            }
        }
        if longest == 0. {
            return Err(PanelError::ProjectionPrecision);
        }
        horizontal = horizontal.map(|h| h / longest);
        let vertical = cross(normal, horizontal);
        let origin = used[&boundary[0]];
        let mut projected = BTreeMap::new();
        for (id, point) in used {
            let offset = sub(point, origin);
            projected.insert(id, [dot(offset, horizontal), dot(offset, vertical)]);
        }
        let mut outline: Vec<_> = boundary.iter().map(|id| projected[id]).collect();
        let lower: [f64; 2] =
            std::array::from_fn(|i| outline.iter().map(|p| p[i]).fold(f64::INFINITY, f64::min));
        let size = std::array::from_fn(|i| {
            outline
                .iter()
                .map(|p| p[i])
                .fold(f64::NEG_INFINITY, f64::max)
                - lower[i]
        });
        let translate = |p: &mut [f64; 2]| {
            for (value, origin) in p.iter_mut().zip(lower) {
                *value -= origin;
            }
        };
        outline.iter_mut().for_each(translate);
        projected.values_mut().for_each(translate);
        for face in &record.faces {
            let [a, b, c] = face.vertices.map(|id| projected[&id]);
            if turn(a, b, c) <= 0. {
                return Err(PanelError::FoldedFace(face.id));
            }
        }
        // A simple boundary and positive orientation on every triangle give
        // winding number one inside. Overlapping triangle interiors would
        // contribute more than one.
        validate_outline(&outline)?;
        Ok(Self {
            outline,
            size,
            outline_ids: boundary,
        })
    }

    pub fn outline_mm(&self) -> &[[f64; 2]] {
        &self.outline
    }

    /// IDs from the panel's surface in the same order as the outline points.
    pub fn outline_vertex_ids(&self) -> &[VertexId] {
        &self.outline_ids
    }

    pub fn size_mm(&self) -> [f64; 2] {
        self.size
    }
}

fn boundary_loop(edges: BTreeMap<(VertexId, VertexId), ()>) -> Result<Vec<VertexId>, PanelError> {
    let count = edges.len();
    if count > MAX_BOUNDARY_EDGES {
        return Err(PanelError::BoundaryTooLarge);
    }
    let mut successors: BTreeMap<_, _> = edges.into_keys().collect();
    if count < 3 || successors.len() != count {
        return Err(PanelError::SingleBoundaryRequired);
    }
    let start = *successors.first_key_value().unwrap().0;
    let mut boundary = vec![start];
    let mut vertex = start;
    while let Some(next) = successors.remove(&vertex) {
        if next == start && successors.is_empty() {
            return Ok(boundary);
        }
        if next == start {
            break;
        }
        boundary.push(next);
        vertex = next;
    }
    Err(PanelError::SingleBoundaryRequired)
}

pub(crate) fn validate_outline(points: &[[f64; 2]]) -> Result<(), PanelError> {
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        let a = points[i];
        let b = points[j];
        let c = points[(j + 1) % points.len()];
        if a == b || overlaps_at_join(a, b, c) {
            return Err(PanelError::IntersectingOutline);
        }
        for k in (i + 2)..points.len() {
            let l = (k + 1) % points.len();
            if l != i && intersects(a, b, points[k], points[l]) {
                return Err(PanelError::IntersectingOutline);
            }
        }
    }
    Ok(())
}

pub(crate) fn overlaps_at_join(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    turn(a, b, c) == 0. && (between(a, b, c) || between(c, a, b))
}

pub(crate) fn intersects(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    // A bounding box check avoids unnecessary robust predicates on distant edges.
    if (0..2).any(|i| a[i].max(b[i]) < c[i].min(d[i]) || c[i].max(d[i]) < a[i].min(b[i])) {
        return false;
    }
    let [c_side, d_side, a_side, b_side] =
        [turn(a, b, c), turn(a, b, d), turn(c, d, a), turn(c, d, b)];
    (c_side == 0. && between(c, a, b))
        || (d_side == 0. && between(d, a, b))
        || (a_side == 0. && between(a, c, d))
        || (b_side == 0. && between(b, c, d))
        || (opposite(c_side, d_side) && opposite(a_side, b_side))
}

fn opposite(a: f64, b: f64) -> bool {
    (a > 0. && b < 0.) || (a < 0. && b > 0.)
}

fn between(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> bool {
    (0..2).all(|i| p[i] >= a[i].min(b[i]) && p[i] <= a[i].max(b[i]))
}

pub(crate) fn turn(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let [a, b, c] = [a, b, c].map(|[x, y]| Coord { x, y });
    orient2d(a, b, c)
}

pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

pub(crate) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn norm([x, y, z]: [f64; 3]) -> f64 {
    x.hypot(y).hypot(z)
}

#[derive(Clone, Copy)]
struct Plane {
    origin: [f64; 3],
    normal: [f64; 3],
    twice_area: f64,
}

impl Plane {
    fn through([a, b, c]: [[f64; 3]; 3]) -> Option<Self> {
        let axis = cross(sub(b, a), sub(c, a));
        let twice_area = norm(axis);
        (twice_area > 0.).then(|| Self {
            origin: a,
            normal: axis.map(|value| value / twice_area),
            twice_area,
        })
    }

    fn distance(self, point: [f64; 3]) -> f64 {
        dot(sub(point, self.origin), self.normal).abs()
    }
}

#[derive(Debug)]
pub enum PanelError {
    NonPlanar { vertex: VertexId, distance_mm: f64 },
    SingleBoundaryRequired,
    BoundaryTooLarge,
    CoordinateRange,
    ProjectionPrecision,
    InvalidPatch(GeometryError),
    FoldedFace(FaceId),
    IntersectingOutline,
}

impl fmt::Display for PanelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPlanar {
                vertex,
                distance_mm,
            } => write!(
                f,
                "export a flat panel: vertex {} is {distance_mm:.6} mm from the reference plane (limit {PLANAR_TOLERANCE_MM} mm)",
                vertex.0
            ),
            Self::SingleBoundaryRequired => write!(
                f,
                "export one connected surface with a single outline and no holes"
            ),
            Self::BoundaryTooLarge => {
                write!(f, "outline exceeds the {MAX_BOUNDARY_EDGES}-edge limit")
            }
            Self::CoordinateRange => write!(
                f,
                "panel coordinates must lie within ±{MAX_COORDINATE_MM} mm"
            ),
            Self::ProjectionPrecision => write!(
                f,
                "panel is too small to project at the available precision"
            ),
            Self::InvalidPatch(error) => write!(f, "{error}"),
            Self::FoldedFace(face) => write!(
                f,
                "triangle {} folds over or collapses in the plane",
                face.0
            ),
            Self::IntersectingOutline => write!(f, "projected outline intersects itself"),
        }
    }
}

impl std::error::Error for PanelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidPatch(error) => Some(error),
            _ => None,
        }
    }
}
