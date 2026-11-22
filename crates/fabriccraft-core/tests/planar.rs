use fabriccraft_core::{
    geometry::{
        Face, FaceId, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId, Vertex, VertexId,
    },
    planar::{PanelError, PlanarPanel, planar_patches},
};

fn mesh(points: &[[f64; 3]], triangles: &[[u64; 3]]) -> Surface {
    Surface::import(
        SurfaceId(1),
        Provenance {
            source: "analytic test".into(),
            unit: LengthUnit::Millimetre,
            frame: ImportFrame::IDENTITY,
        },
        points
            .iter()
            .enumerate()
            .map(|(id, &position)| Vertex {
                id: VertexId(id as u64),
                position,
            })
            .collect(),
        triangles
            .iter()
            .enumerate()
            .map(|(id, &corners)| Face {
                id: FaceId(id as u64),
                vertices: corners.map(VertexId),
            })
            .collect(),
    )
    .unwrap()
}

fn close(observed: f64, expected: f64) {
    assert!(
        (observed - expected).abs() < 1e-9,
        "{observed} vs {expected}"
    );
}

#[test]
fn extracts_each_box_side_from_a_closed_mesh() {
    let surface = mesh(
        &[
            [0., 0., 0.],
            [2., 0., 0.],
            [2., 1., 0.],
            [0., 1., 0.],
            [0., 0., 3.],
            [2., 0., 3.],
            [2., 1., 3.],
            [0., 1., 3.],
        ],
        &[
            [4, 5, 6],
            [4, 6, 7],
            [0, 2, 1],
            [0, 3, 2],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    );
    let patches = planar_patches(&surface).unwrap();
    assert_eq!(patches.len(), 6);
    for (index, patch) in patches.iter().enumerate() {
        assert_eq!(patch.seed(), FaceId((index * 2) as u64));
        let panel = patch.panel().unwrap();
        assert_eq!(panel.outline_mm().len(), 4);
    }
    assert_eq!(patches[0].normal(), [0., 0., 1.]);
    assert_eq!(patches[0].center_mm(), [1., 0.5, 3.]);
    assert_eq!(patches[0].panel().unwrap().size_mm(), [2., 1.]);
}

#[test]
fn preserves_islands_and_rejects_holes_in_selected_patches() {
    let surface = mesh(
        &[
            [0., 0., 0.],
            [3., 0., 0.],
            [3., 3., 0.],
            [0., 3., 0.],
            [1., 1., 0.],
            [2., 1., 0.],
            [2., 2., 0.],
            [1., 2., 0.],
            [10., 0., 0.],
            [11., 0., 0.],
            [10., 1., 0.],
        ],
        &[
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
            [8, 9, 10],
        ],
    );
    let patches = planar_patches(&surface).unwrap();
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].triangle_count(), 8);
    assert!(matches!(
        patches[0].panel(),
        Err(PanelError::SingleBoundaryRequired)
    ));
    assert_eq!(patches[1].triangle_count(), 1);
    assert!(patches[1].panel().is_ok());
}

#[test]
fn a_patch_cannot_keep_growing_as_the_plane_drifts() {
    let surface = mesh(
        &[
            [0., 0., 0.],
            [0., 1., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [2., 0., 0.00075],
            [2., 1., 0.00075],
            [3., 0., 0.0015],
            [3., 1., 0.0015],
        ],
        &[
            [0, 2, 3],
            [0, 3, 1],
            [2, 4, 5],
            [2, 5, 3],
            [4, 6, 7],
            [4, 7, 5],
        ],
    );
    let patches = planar_patches(&surface).unwrap();
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].triangle_count(), 4);
    assert_eq!(patches[1].triangle_count(), 2);
}

#[test]
fn inclined_rectangle_retains_dimensions_and_excludes_unused_points() {
    let surface = mesh(
        &[
            [10., 20., 30.],
            [210., 20., 30.],
            [210., 80., 110.],
            [10., 80., 110.],
            [1e20; 3],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    );
    let panel = PlanarPanel::from_surface(&surface).unwrap();
    let [width, height] = panel.size_mm();
    close(width, 200.);
    close(height, 100.);
    assert_eq!(panel.outline_mm().len(), 4);
}

#[test]
fn rejects_nonplanar_mesh_with_physical_tolerance() {
    let surface = mesh(
        &[
            [0., 0., 0.],
            [200., 0., 0.],
            [200., 100., 0.],
            [0., 100., 0.01],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    );
    assert!(matches!(
        PlanarPanel::from_surface(&surface),
        Err(PanelError::NonPlanar { .. })
    ));
}

#[test]
fn rejects_disconnected_islands() {
    let surface = mesh(
        &[
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [2., 1., 0.],
        ],
        &[[0, 1, 2], [3, 4, 5]],
    );
    assert!(matches!(
        PlanarPanel::from_surface(&surface),
        Err(PanelError::SingleBoundaryRequired)
    ));
}

#[test]
fn rejects_a_hole() {
    let surface = mesh(
        &[
            [0., 0., 0.],
            [3., 0., 0.],
            [3., 3., 0.],
            [0., 3., 0.],
            [1., 1., 0.],
            [2., 1., 0.],
            [2., 2., 0.],
            [1., 2., 0.],
        ],
        &[
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ],
    );
    assert!(matches!(
        PlanarPanel::from_surface(&surface),
        Err(PanelError::SingleBoundaryRequired)
    ));
}

#[test]
fn rejects_a_fold_with_consistent_mesh_winding() {
    let surface = mesh(
        &[[0., 0., 0.], [3., 0., 0.], [0., 3., 0.], [1., 1., 0.]],
        &[[0, 1, 2], [0, 2, 3]],
    );
    assert!(matches!(
        PlanarPanel::from_surface(&surface),
        Err(PanelError::FoldedFace(_))
    ));
}

#[test]
fn rejects_crossing_outline_even_when_all_triangles_keep_orientation() {
    let mut points = vec![[0.; 3]];
    for index in 0..5 {
        let angle = f64::from(index) * std::f64::consts::TAU * 0.4;
        points.push([angle.cos(), angle.sin(), 0.]);
    }
    let surface = mesh(
        &points,
        &[[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 5], [0, 5, 1]],
    );
    assert!(matches!(
        PlanarPanel::from_surface(&surface),
        Err(PanelError::IntersectingOutline)
    ));
}
