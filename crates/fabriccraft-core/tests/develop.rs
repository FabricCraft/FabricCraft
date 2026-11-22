use std::f64::consts::TAU;

use fabriccraft_core::{
    develop::{DevelopError, DevelopedPanel},
    geometry::{
        Face, FaceId, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId, Vertex, VertexId,
    },
    tube::{TubeDevelopment, TubeError},
};

const SIDES: usize = 8;
const RADIUS: f64 = 50.;
const HEIGHT: f64 = 120.;

fn ring(index: usize, height: f64) -> [f64; 3] {
    let angle = TAU * (index % SIDES) as f64 / SIDES as f64;
    [RADIUS * angle.cos(), RADIUS * angle.sin(), height]
}

fn mesh(points: Vec<[f64; 3]>, faces: Vec<[usize; 3]>) -> Surface {
    Surface::import(
        SurfaceId(1),
        Provenance {
            source: "analytic development fixture".into(),
            unit: LengthUnit::Millimetre,
            frame: ImportFrame::IDENTITY,
        },
        points
            .into_iter()
            .enumerate()
            .map(|(i, position)| Vertex {
                id: VertexId(i as u64),
                position,
            })
            .collect(),
        faces
            .into_iter()
            .enumerate()
            .map(|(i, ids)| Face {
                id: FaceId(i as u64),
                vertices: ids.map(|id| VertexId(id as u64)),
            })
            .collect(),
    )
    .unwrap()
}

fn cylinder(cut: bool) -> Surface {
    let count = if cut { SIDES + 1 } else { SIDES };
    let points = (0..count)
        .flat_map(|i| [ring(i, 0.), ring(i, HEIGHT)])
        .collect();
    let faces = (0..SIDES)
        .flat_map(|i| {
            let a = 2 * i;
            let b = 2 * ((i + 1) % count);
            [[a, b, b + 1], [a, b + 1, a + 1]]
        })
        .collect();
    mesh(points, faces)
}

fn fan(cut: bool) -> Surface {
    let count = if cut { SIDES + 1 } else { SIDES };
    let points = std::iter::once([0., 0., HEIGHT])
        .chain((0..count).map(|i| ring(i, 0.)))
        .collect();
    let faces = (0..SIDES)
        .map(|i| [0, i + 1, (i + 1) % count + 1])
        .collect();
    mesh(points, faces)
}

fn close(observed: f64, expected: f64) {
    assert!(
        (observed - expected).abs() < 1e-8,
        "{observed} vs {expected}"
    );
}

#[test]
fn develops_the_cut_cylinder_at_its_faceted_dimensions() {
    let development = DevelopedPanel::from_surface(&cylinder(true)).unwrap();
    let mut dims = development.panel().size_mm();
    dims.sort_by(f64::total_cmp);
    close(dims[0], HEIGHT);
    close(
        dims[1],
        2. * RADIUS * (TAU / (2. * SIDES as f64)).sin() * SIDES as f64,
    );
    assert!(development.max_edge_error_mm() < 1e-9);
    assert!(development.max_relative_area_error() < 1e-10);
}

#[test]
fn develops_a_cone_cut_from_apex_to_rim() {
    let development = DevelopedPanel::from_surface(&fan(true)).unwrap();
    let outline = development.panel().outline_mm();
    assert_eq!(outline.len(), SIDES + 2);
    let slant = RADIUS.hypot(HEIGHT);
    let chord = 2. * RADIUS * (TAU / (2. * SIDES as f64)).sin();
    let mut lengths: Vec<_> = outline
        .iter()
        .zip(outline.iter().cycle().skip(1))
        .map(|(a, b)| (b[0] - a[0]).hypot(b[1] - a[1]))
        .collect();
    lengths.sort_by(f64::total_cmp);
    for length in &lengths[..SIDES] {
        close(*length, chord);
    }
    for length in &lengths[SIDES..] {
        close(*length, slant);
    }
}

#[test]
fn rejects_missing_cut_or_incompatible_curvature() {
    for surface in [cylinder(false), fan(false)] {
        let error = DevelopedPanel::from_surface(&surface).unwrap_err();
        assert!(
            matches!(error, DevelopError::ClosureMismatch { .. }),
            "{error:?}"
        );
    }
}

#[test]
fn rejects_disconnected_triangles() {
    let surface = mesh(
        vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [2., 0., 0.],
            [3., 0., 0.],
            [2., 1., 0.],
        ],
        vec![[0, 1, 2], [3, 4, 5]],
    );
    assert!(matches!(
        DevelopedPanel::from_surface(&surface),
        Err(DevelopError::Disconnected)
    ));
}

#[test]
fn develops_a_planar_fan_with_a_consistent_interior_loop() {
    let surface = mesh(
        vec![
            [0., 0., 0.],
            [30., 0., 0.],
            [30., 20., 0.],
            [0., 20., 0.],
            [15., 10., 0.],
        ],
        vec![[0, 1, 4], [1, 2, 4], [2, 3, 4], [3, 0, 4]],
    );
    let development = DevelopedPanel::from_surface(&surface).unwrap();
    assert!(development.max_edge_error_mm() < 1e-9);
    assert!(development.max_relative_area_error() < 1e-10);
}

#[test]
fn rejects_crossing_outline_even_when_triangle_lengths_match() {
    let mut points = vec![[0.; 3]];
    for i in 0..5 {
        let angle = TAU * f64::from(i) * 0.4;
        points.push([angle.cos(), angle.sin(), 0.]);
    }
    let surface = mesh(
        points,
        vec![[0, 1, 2], [0, 2, 3], [0, 3, 4], [0, 4, 5], [0, 5, 1]],
    );
    let error = DevelopedPanel::from_surface(&surface).unwrap_err();
    assert!(matches!(
        error,
        DevelopError::Outline(fabriccraft_core::planar::PanelError::IntersectingOutline)
    ));
}

#[test]
fn opens_a_tube_inside_fabriccraft_and_verifies_both_banks() {
    let cylinder = cylinder(false);
    let automatic = TubeDevelopment::from_surface(&cylinder, None).unwrap();
    assert_eq!(automatic.source_path(), &[VertexId(0), VertexId(1)]);
    close(automatic.source_length_mm(), HEIGHT);
    assert!(automatic.max_paired_edge_error_mm() < 1e-9);
    let [left, right] = automatic.banks_mm();
    close(
        (left[0][0] - left[1][0]).hypot(left[0][1] - left[1][1]),
        HEIGHT,
    );
    close(
        (right[0][0] - right[1][0]).hypot(right[0][1] - right[1][1]),
        HEIGHT,
    );
    assert_ne!(left, right);

    let selected = [VertexId(4), VertexId(5)];
    let manual = TubeDevelopment::from_surface(&cylinder, Some(&selected)).unwrap();
    assert_eq!(manual.source_path(), &selected);
    close(manual.source_length_mm(), HEIGHT);
    let mut dims = manual.development().panel().size_mm();
    dims.sort_by(f64::total_cmp);
    close(dims[0], HEIGHT);
    close(
        dims[1],
        2. * RADIUS * (TAU / (2. * SIDES as f64)).sin() * SIDES as f64,
    );
    assert!(matches!(
        DevelopedPanel::from_surface(&cylinder),
        Err(DevelopError::ClosureMismatch { .. })
    ));
}

#[test]
fn validates_picked_seam_edges_and_tube_topology() {
    let annulus = cylinder(false);
    for path in [
        vec![VertexId(0)],
        vec![VertexId(0), VertexId(2)],
        vec![VertexId(0), VertexId(7)],
    ] {
        assert!(matches!(
            TubeDevelopment::from_surface(&annulus, Some(&path)),
            Err(TubeError::InvalidPath)
        ));
    }
    assert!(matches!(
        TubeDevelopment::from_surface(&cylinder(true), None),
        Err(TubeError::NotATube)
    ));
}

#[test]
fn opens_a_shortest_seam_with_an_interior_vertex() {
    let points: Vec<_> = (0..SIDES)
        .flat_map(|i| {
            let top = if i == 3 { HEIGHT } else { HEIGHT + 10. };
            [ring(i, 0.), ring(i, top / 2.), ring(i, top)]
        })
        .collect();
    let triangles = (0..SIDES)
        .flat_map(|i| {
            let a = 3 * i;
            let b = 3 * ((i + 1) % SIDES);
            [
                [a, b, b + 1],
                [a, b + 1, a + 1],
                [a + 1, b + 1, b + 2],
                [a + 1, b + 2, a + 2],
            ]
        })
        .collect();
    let result = TubeDevelopment::from_surface(&mesh(points, triangles), None).unwrap();
    assert_eq!(
        result.source_path(),
        &[VertexId(9), VertexId(10), VertexId(11)]
    );
    close(result.source_length_mm(), HEIGHT);
    assert_eq!(result.banks_mm().map(|bank| bank.len()), [3, 3]);
}
