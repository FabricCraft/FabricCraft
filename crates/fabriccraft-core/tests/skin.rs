use fabriccraft_core::{
    develop::DevelopedPanel,
    geometry::Axis,
    planar::CutPattern,
    skin::{Boundary, build_skin, suggest_boundary},
};

#[test]
fn bridges_a_frame_with_no_reference_face_in_the_middle() {
    let anchors = [
        [0., 0., 5.],
        [200., 0., 5.],
        [200., 100., 5.],
        [0., 100., 5.],
    ];
    let skin = build_skin(&anchors).unwrap();
    assert_eq!(skin.record().faces.len(), 2);
    assert_eq!(
        skin.record()
            .vertices_mm
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>(),
        anchors
    );
    let [width, height] = DevelopedPanel::from_surface(&skin)
        .unwrap()
        .panel()
        .size_mm();
    assert!((width - 200.).abs() < 1e-9 && (height - 100.).abs() < 1e-9);
}

#[test]
fn suggests_outward_corners_and_skips_recessed_supports() {
    let points = [
        [0., 0., 0.],
        [0., 0., 10.],
        [200., 0., 10.],
        [200., 100., 8.],
        [0., 100., 9.],
        [100., 50., 0.],
        [100., 0., 10.],
    ];
    assert_eq!(suggest_boundary(&points, Axis::Z).unwrap(), [1, 2, 3, 4]);
    assert_eq!(
        suggest_boundary(&points, Axis::NegativeZ).unwrap(),
        [0, 4, 3, 2]
    );
}

#[test]
fn respects_concavity_and_all_manual_boundary_anchors() {
    let anchors = [
        [0., 0., 0.],
        [10., 0., 0.],
        [10., 10., 0.],
        [3., 4., 0.],
        [0., 10., 0.],
    ];
    let skin = build_skin(&anchors).unwrap();
    assert_eq!(skin.record().faces.len(), 3);
    let area: f64 = skin
        .record()
        .faces
        .iter()
        .map(|face| {
            let [a, b, c] = face.vertices.map(|id| anchors[id.0 as usize]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / 2.
        })
        .sum();
    assert_eq!(area, 70.);
}

#[test]
fn keeps_3d_support_heights_and_rejects_self_crossing_boundaries() {
    let warped = [
        [0., 0., 10.],
        [10., 0., 20.],
        [10., 10., 18.],
        [0., 10., 15.],
    ];
    let surface = build_skin(&warped).unwrap();
    assert_eq!(surface.record().vertices_mm[1].position[2], 20.);
    let flat = warped.map(|[x, y, _]| [x, y, 0.]);
    assert!(build_skin(&[flat[0], flat[2], flat[1], flat[3]]).is_err());
    assert!(build_skin(&[warped[0], warped[0], warped[1], warped[2]]).is_err());
}

const SUPPORTS: [[f64; 3]; 7] = [
    [0., 0., 0.],
    [100., 0., 0.],
    [100., 60., 0.],
    [0., 60., 0.],
    [35., 10., 0.],
    [65., 10., 0.],
    [0., 0., 20.],
];

#[test]
fn shuffled_corner_picks_produce_the_same_rectangle() {
    for a in 0..4 {
        for b in (0..4).filter(|&b| b != a) {
            for c in (0..4).filter(|&c| c != a && c != b) {
                let mut boundary = Boundary::new(Axis::Z);
                for id in [a, b, c, 6 - a - b - c] {
                    boundary = boundary.toggle_point(&SUPPORTS, id).unwrap();
                }
                let panel =
                    DevelopedPanel::from_surface(&boundary.skin(&SUPPORTS).unwrap()).unwrap();
                let mut size = panel.panel().size_mm();
                size.sort_by(f64::total_cmp);
                assert!((size[0] - 60.).abs() < 1e-8 && (size[1] - 100.).abs() < 1e-8);
            }
        }
    }
}

#[test]
fn an_inset_edge_reshapes_the_outline_and_stays_together() {
    let boundary = Boundary::suggest(&SUPPORTS[..6], Axis::Z).unwrap();
    let boundary = boundary.toggle_edge(&SUPPORTS, [4, 5]).unwrap();
    assert_eq!(boundary.edges(), &[[4, 5]]);
    assert_eq!(boundary.points(), &[0, 4, 5, 1, 2, 3]);
    let panel = DevelopedPanel::from_surface(&boundary.skin(&SUPPORTS).unwrap()).unwrap();
    assert_eq!(panel.panel().outline_mm().len(), 6);

    let boundary = boundary.toggle_point(&SUPPORTS, 4).unwrap();
    assert!(boundary.edges().is_empty());
    assert_eq!(boundary.points().len(), 5);
}

#[test]
fn connects_existing_points_and_refuses_coincident_or_conflicting_picks() {
    let boundary = Boundary::suggest(&SUPPORTS[..6], Axis::Z).unwrap();
    assert!(boundary.toggle_point(&SUPPORTS, 6).is_err());
    assert!(boundary.toggle_edge(&SUPPORTS, [0, 6]).is_err());
    let [a, b, c, d]: [usize; 4] = boundary.points().try_into().unwrap();
    let boundary = boundary.toggle_edge(&SUPPORTS, [a, b]).unwrap();
    let boundary = boundary.toggle_edge(&SUPPORTS, [b, c]).unwrap();
    assert!(boundary.toggle_edge(&SUPPORTS, [b, d]).is_err());
    assert_eq!(boundary.edges().len(), 2);
    assert_eq!(boundary.points(), &[a, b, c, d]);

    // An already selected point can move next to the clicked edge's endpoint
    // when doing so gives a simple outline and preserves the selected edges.
    let boundary = boundary.toggle_point(&SUPPORTS, 4).unwrap();
    let boundary = boundary.toggle_edge(&SUPPORTS, [4, 2]).unwrap();
    assert!(boundary.edges().contains(&[2, 4]));
    boundary.skin(&SUPPORTS).unwrap();
}

#[test]
fn orders_vertical_coverage_in_its_own_plane() {
    let points = SUPPORTS.map(|[x, y, z]| [z, x, y]);
    let mut boundary = Boundary::new(Axis::NegativeX);
    for index in [0, 2, 1, 3] {
        boundary = boundary.toggle_point(&points, index).unwrap();
    }
    assert_eq!(boundary.skin(&points).unwrap().record().faces.len(), 2);
    assert!(boundary.toggle_point(&points, 6).is_err());
}

#[test]
fn seam_allowance_offsets_the_sewing_line_outward_in_millimetres() {
    let surface = build_skin(&SUPPORTS[..4]).unwrap();
    let panel = DevelopedPanel::from_surface(&surface).unwrap();
    let cut = CutPattern::new(panel.panel(), 12.7).unwrap();
    let [w, h] = cut.size_mm();
    assert!((w - 125.4).abs() < 1e-8 && (h - 85.4).abs() < 1e-8);
    let sewing = cut.sewing_line_mm();
    for axis in 0..2 {
        let low = sewing.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min);
        assert!((low - 12.7).abs() < 1e-8);
    }
    let zero = CutPattern::new(panel.panel(), 0.).unwrap();
    assert_eq!(zero.cut_line_mm(), panel.panel().outline_mm());
    assert!(CutPattern::new(panel.panel(), -1.).is_err());
}

#[test]
fn seam_allowance_respects_a_notch_until_its_cut_edges_meet() {
    let surface = build_skin(&[
        [0., 0., 0.],
        [100., 0., 0.],
        [100., 100., 0.],
        [55., 100., 0.],
        [55., 25., 0.],
        [45., 25., 0.],
        [45., 100., 0.],
        [0., 100., 0.],
    ])
    .unwrap();
    let panel = DevelopedPanel::from_surface(&surface).unwrap();
    assert!(CutPattern::new(panel.panel(), 4.).is_ok());
    assert!(CutPattern::new(panel.panel(), 5.).is_err());
    assert!(CutPattern::new(panel.panel(), 6.).is_err());
}
