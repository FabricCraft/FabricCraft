use std::collections::{BTreeMap, BTreeSet};

use crate::geometry::{FaceId, Surface, face_neighbors};

use super::{MAX_COORDINATE_MM, PLANAR_TOLERANCE_MM, PanelError, PlanarPanel, Plane, dot};

/// One edge-connected patch of a validated reference mesh. Selecting a patch
/// preserves its source face and vertex IDs.
pub struct PlanarPatch<'a> {
    surface: &'a Surface,
    faces: Vec<usize>,
    center_mm: [f64; 3],
    normal: [f64; 3],
}

impl PlanarPatch<'_> {
    pub fn seed(&self) -> FaceId {
        self.surface.record().faces[self.faces[0]].id
    }

    pub fn triangle_count(&self) -> usize {
        self.faces.len()
    }

    /// Center of the patch's 3D bounding box, in model coordinates.
    pub fn center_mm(&self) -> [f64; 3] {
        self.center_mm
    }

    pub fn normal(&self) -> [f64; 3] {
        self.normal
    }

    /// Validate the patch on its own before producing its closed 2D outline.
    pub fn panel(&self) -> Result<PlanarPanel, PanelError> {
        let original = self.surface.record();
        let faces: Vec<_> = self
            .faces
            .iter()
            .map(|&index| original.faces[index])
            .collect();
        let used: BTreeSet<_> = faces.iter().flat_map(|f| f.vertices).collect();
        let vertices_mm = used
            .into_iter()
            .map(|id| {
                let index = original
                    .vertices_mm
                    .binary_search_by_key(&id, |v| v.id)
                    .unwrap();
                original.vertices_mm[index]
            })
            .collect();
        let patch = Surface::from_record(crate::geometry::SurfaceRecord {
            schema_version: original.schema_version,
            id: original.id,
            provenance: original.provenance.clone(),
            faces,
            vertices_mm,
        })
        .map_err(PanelError::InvalidPatch)?;
        PlanarPanel::from_surface(&patch)
    }
}

/// Group adjacent triangles against a fixed seed plane, in source face order.
/// Each triangle is assigned once. A patch may still have an unsupported
/// outline, which is checked when it is selected for export.
pub fn planar_patches(surface: &Surface) -> Result<Vec<PlanarPatch<'_>>, PanelError> {
    let record = surface.record();
    let positions: BTreeMap<_, _> = record
        .vertices_mm
        .iter()
        .map(|v| (v.id, v.position))
        .collect();
    let triangles: Vec<_> = record
        .faces
        .iter()
        .map(|f| f.vertices.map(|id| positions[&id]))
        .collect();
    if triangles
        .iter()
        .flatten()
        .flatten()
        .any(|value| value.abs() > MAX_COORDINATE_MM)
    {
        return Err(PanelError::CoordinateRange);
    }
    let planes = triangles
        .iter()
        .copied()
        .map(Plane::through)
        .collect::<Option<Vec<_>>>()
        .ok_or(PanelError::ProjectionPrecision)?;
    let neighbors = face_neighbors(&record.faces);

    let mut patches = Vec::new();
    let mut assigned = vec![false; triangles.len()];
    for (seed, &plane) in planes.iter().enumerate() {
        if assigned[seed] {
            continue;
        }
        assigned[seed] = true;
        let mut stack = vec![seed];
        let mut faces = Vec::new();
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        while let Some(face) = stack.pop() {
            faces.push(face);
            for point in triangles[face] {
                for axis in 0..3 {
                    low[axis] = low[axis].min(point[axis]);
                    high[axis] = high[axis].max(point[axis]);
                }
            }
            for (other, _) in neighbors[face].into_iter().flatten() {
                if !assigned[other]
                    && dot(plane.normal, planes[other].normal) > 0.
                    && triangles[other]
                        .iter()
                        .all(|&p| plane.distance(p) <= PLANAR_TOLERANCE_MM)
                {
                    assigned[other] = true;
                    stack.push(other);
                }
            }
        }
        patches.push(PlanarPatch {
            surface,
            faces,
            normal: plane.normal,
            center_mm: std::array::from_fn(|axis| low[axis].midpoint(high[axis])),
        });
    }
    Ok(patches)
}
