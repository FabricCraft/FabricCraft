use fabriccraft_core::{
    develop::DevelopedPanel,
    geometry::Axis,
    skin::{build_skin, suggest_boundary},
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
