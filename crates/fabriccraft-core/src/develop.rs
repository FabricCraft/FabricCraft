//! Edge-length-preserving development of a triangulated panel with a prepared cut.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
};

use crate::{
    geometry::{FaceId, GeometryError, Surface, SurfaceRecord, Vertex, VertexId, face_neighbors},
    planar::{MAX_COORDINATE_MM, PanelError, PlanarPanel, cross, dot, norm, sub, turn},
};

const ABSOLUTE_TOLERANCE_MM: f64 = 1e-7;
const RELATIVE_TOLERANCE: f64 = 1e-8;

#[derive(Debug)]
pub struct DevelopedPanel {
    panel: PlanarPanel,
    max_edge_error_mm: f64,
    max_relative_area_error: f64,
}

impl DevelopedPanel {
    /// Develop a connected mesh with a single prepared boundary. Cut banks
    /// use distinct vertex IDs, even where their source positions coincide.
    pub fn from_surface(surface: &Surface) -> Result<Self, DevelopError> {
        let record = surface.record();
        let indices: BTreeMap<_, _> = record
            .vertices_mm
            .iter()
            .enumerate()
            .map(|(index, vertex)| (vertex.id, index))
            .collect();
        let faces: Vec<_> = record
            .faces
            .iter()
            .map(|face| face.vertices.map(|id| indices[&id]))
            .collect();
        let points: Vec<_> = record
            .vertices_mm
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        if faces.iter().flatten().any(|&index| {
            points[index]
                .iter()
                .any(|value| value.abs() > MAX_COORDINATE_MM)
        }) {
            return Err(DevelopError::CoordinateRange);
        }

        let adjacency = face_neighbors(&record.faces);
        let mut mapped = vec![None; points.len()];
        let mut queued = vec![false; faces.len()];
        let [a, b, c] = faces[0];
        let (seed_slot, seed_length) = [(a, b), (b, c), (c, a)]
            .into_iter()
            .enumerate()
            .map(|(slot, (i, j))| (slot, norm(sub(points[j], points[i]))))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        mapped[faces[0][seed_slot]] = Some([0., 0.]);
        mapped[faces[0][(seed_slot + 1) % 3]] = Some([seed_length, 0.]);
        queued[0] = true;

        let mut queue = VecDeque::from([(0, seed_slot)]);
        let mut visited = 0;
        while let Some((index, slot)) = queue.pop_front() {
            let face = record.faces[index].id;
            let mut corners = faces[index];
            corners.rotate_left(slot);
            let [a, b, c] = corners;
            let [p, q, r] = corners.map(|vertex| points[vertex]);
            let position = third_point(
                [p, q, r],
                mapped[a].ok_or(DevelopError::Precision(face))?,
                mapped[b].ok_or(DevelopError::Precision(face))?,
            )
            .ok_or(DevelopError::Precision(face))?;

            if let Some(existing) = mapped[c] {
                let gap_mm = distance_2d(existing, position);
                if gap_mm > distance_tolerance(existing, position) {
                    return Err(DevelopError::ClosureMismatch {
                        face,
                        vertex: record.vertices_mm[c].id,
                        gap_mm,
                    });
                }
            } else {
                mapped[c] = Some(position);
            }

            visited += 1;
            for (neighbor, shared_slot) in adjacency[index].into_iter().flatten() {
                if !queued[neighbor] {
                    queued[neighbor] = true;
                    queue.push_back((neighbor, shared_slot));
                }
            }
        }
        if visited != faces.len() {
            return Err(DevelopError::Disconnected);
        }

        let mut max_edge_error_mm: f64 = 0.;
        let mut max_relative_area_error: f64 = 0.;
        for (index, &[a, b, c]) in faces.iter().enumerate() {
            let face = record.faces[index].id;
            let [Some(u), Some(v), Some(w)] = [a, b, c].map(|id| mapped[id]) else {
                return Err(DevelopError::Precision(face));
            };
            let flat_area = turn(u, v, w);
            let original_area = norm(cross(sub(points[b], points[a]), sub(points[c], points[a])));
            if flat_area <= 0. || original_area <= 0. {
                return Err(DevelopError::Precision(face));
            }
            let area_error = (flat_area / original_area - 1.).abs();
            let mut edge_error_mm: f64 = 0.;
            for (i, j, p, q) in [(a, b, u, v), (b, c, v, w), (c, a, w, u)] {
                let original = norm(sub(points[i], points[j]));
                let error_mm = (distance_2d(p, q) - original).abs();
                if error_mm > distance_tolerance(p, q) || error_mm / original > RELATIVE_TOLERANCE {
                    return Err(DevelopError::Distortion(face));
                }
                edge_error_mm = edge_error_mm.max(error_mm);
            }
            if !area_error.is_finite() || area_error > RELATIVE_TOLERANCE {
                return Err(DevelopError::Distortion(face));
            }
            max_edge_error_mm = max_edge_error_mm.max(edge_error_mm);
            max_relative_area_error = max_relative_area_error.max(area_error);
        }

        let vertices_mm = record
            .vertices_mm
            .iter()
            .zip(mapped)
            .filter_map(|(vertex, position)| {
                position.map(|[x, y]| Vertex {
                    id: vertex.id,
                    position: [x, y, 0.],
                })
            })
            .collect();
        let flattened = Surface::from_record(SurfaceRecord {
            schema_version: record.schema_version,
            id: record.id,
            provenance: record.provenance.clone(),
            vertices_mm,
            faces: record.faces.clone(),
        })
        .map_err(DevelopError::Geometry)?;
        let panel = PlanarPanel::from_surface(&flattened).map_err(DevelopError::Outline)?;
        Ok(Self {
            panel,
            max_edge_error_mm,
            max_relative_area_error,
        })
    }

    pub fn panel(&self) -> &PlanarPanel {
        &self.panel
    }

    pub fn into_panel(self) -> PlanarPanel {
        self.panel
    }

    pub fn max_edge_error_mm(&self) -> f64 {
        self.max_edge_error_mm
    }

    pub fn max_relative_area_error(&self) -> f64 {
        self.max_relative_area_error
    }
}

fn third_point([a, b, c]: [[f64; 3]; 3], p: [f64; 2], q: [f64; 2]) -> Option<[f64; 2]> {
    let edge = sub(b, a);
    let length = norm(edge);
    let flat_length = distance_2d(p, q);
    if length == 0. || flat_length == 0. {
        return None;
    }
    let axis = edge.map(|value| value / length);
    let delta = sub(c, a);
    let along = dot(delta, axis);
    let perpendicular = norm(cross(axis, delta));
    let [x, y] = std::array::from_fn(|i| (q[i] - p[i]) / flat_length);
    // A consistently wound triangle lies to the left of its directed edge.
    let corner = [
        p[0] + along * x - perpendicular * y,
        p[1] + along * y + perpendicular * x,
    ];
    (perpendicular > 0. && corner.iter().all(|value| value.is_finite())).then_some(corner)
}

pub(crate) fn distance_2d(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

pub(crate) fn distance_tolerance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let scale = a.into_iter().chain(b).map(f64::abs).fold(0., f64::max);
    ABSOLUTE_TOLERANCE_MM + scale * (128. * f64::EPSILON)
}

#[derive(Debug)]
pub enum DevelopError {
    CoordinateRange,
    Precision(FaceId),
    Disconnected,
    ClosureMismatch {
        face: FaceId,
        vertex: VertexId,
        gap_mm: f64,
    },
    Distortion(FaceId),
    Geometry(GeometryError),
    Outline(PanelError),
}

impl fmt::Display for DevelopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateRange => write!(
                f,
                "panel coordinates must lie within ±{MAX_COORDINATE_MM} mm"
            ),
            Self::Precision(face) => write!(
                f,
                "triangle {} collapses or exceeds the precision of the development",
                face.0
            ),
            Self::Disconnected => write!(f, "develop one edge-connected mesh patch"),
            Self::ClosureMismatch {
                face,
                vertex,
                gap_mm,
            } => write!(
                f,
                "development will not close at triangle {}, vertex {} ({gap_mm:.6e} mm gap). Check the seam cut and intrinsic curvature",
                face.0, vertex.0
            ),
            Self::Distortion(face) => write!(
                f,
                "triangle {} exceeds the development's edge or area tolerance",
                face.0
            ),
            Self::Geometry(error) => write!(f, "{error}"),
            Self::Outline(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DevelopError {}
