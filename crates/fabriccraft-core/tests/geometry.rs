use fabriccraft_core::geometry::{
    Axis, Face, FaceId, GeometryError, ImportFrame, LengthUnit, Provenance, Surface, SurfaceId,
    Vertex, VertexId,
};

fn vertex(id: u64, position: [f64; 3]) -> Vertex {
    Vertex {
        id: VertexId(id),
        position,
    }
}

fn face(id: u64, vertices: [u64; 3]) -> Face {
    Face {
        id: FaceId(id),
        vertices: vertices.map(VertexId),
    }
}

fn provenance() -> Provenance {
    Provenance {
        source: "analytic rectangle".into(),
        unit: LengthUnit::Millimetre,
        frame: ImportFrame::IDENTITY,
    }
}

fn rectangle() -> (Vec<Vertex>, Vec<Face>) {
    (
        vec![
            vertex(10, [0., 0., 0.]),
            vertex(20, [2., 0., 0.]),
            vertex(30, [2., 3., 0.]),
            vertex(40, [0., 3., 0.]),
        ],
        vec![face(4, [10, 20, 30]), face(7, [10, 30, 40])],
    )
}

fn import(vertices: Vec<Vertex>, faces: Vec<Face>) -> Result<Surface, GeometryError> {
    Surface::import(SurfaceId(8), provenance(), vertices, faces)
}

#[test]
fn rectangle_keeps_ids_winding_and_provenance_on_record_round_trip() {
    let (mut vertices, mut faces) = rectangle();
    vertices.reverse();
    faces.reverse();
    let provenance = Provenance {
        unit: LengthUnit::Centimetre,
        frame: ImportFrame {
            axes: [Axis::Y, Axis::X, Axis::Z],
            translation_mm: [1., -2., 5.],
        },
        ..provenance()
    };
    let surface = Surface::import(SurfaceId(8), provenance.clone(), vertices, faces).unwrap();
    let record = surface.into_record();
    assert_eq!(record.schema_version, fabriccraft_core::schema_version());
    assert_eq!(record.id, SurfaceId(8));
    assert_eq!(record.provenance, provenance);
    assert_eq!(record.vertices_mm[1], vertex(20, [1., 18., 5.]));
    assert_eq!(record.vertices_mm[2], vertex(30, [31., 18., 5.]));
    assert_eq!(
        record.faces,
        vec![face(4, [10, 30, 20]), face(7, [10, 40, 30])]
    );
    let mut reordered = record.clone();
    reordered.vertices_mm.rotate_left(1);
    reordered.faces.reverse();
    assert_eq!(Surface::from_record(reordered).unwrap().record(), &record);
}

#[test]
fn rotated_metric_import_agrees_with_native_mm_and_preserves_winding() {
    let (vertices, faces) = rectangle();
    let native = import(vertices.clone(), faces.clone()).unwrap();
    let transformed = vertices
        .into_iter()
        .map(|v| {
            let [x, y, z] = v.position;
            Vertex {
                position: [-y / 1000., x / 1000., z / 1000.],
                ..v
            }
        })
        .collect();
    let source = Provenance {
        unit: LengthUnit::Metre,
        frame: ImportFrame {
            axes: [Axis::Y, Axis::NegativeX, Axis::Z],
            ..ImportFrame::IDENTITY
        },
        ..provenance()
    };
    let result = Surface::import(SurfaceId(8), source, transformed, faces).unwrap();
    assert_eq!(native.record().vertices_mm, result.record().vertices_mm);
    assert_eq!(native.record().faces, result.record().faces);
    assert_eq!(LengthUnit::Inch.millimetres_per_unit(), 25.4);
}

#[test]
fn invalid_coordinates_frame_and_version_have_distinct_diagnostics() {
    let (mut vertices, faces) = rectangle();
    let mut record = import(vertices.clone(), faces.clone())
        .unwrap()
        .into_record();
    record.schema_version += 1;
    assert_eq!(
        Surface::from_record(record).unwrap_err(),
        GeometryError::UnsupportedVersion(2)
    );
    for frame in [
        ImportFrame {
            axes: [Axis::X, Axis::NegativeX, Axis::Z],
            ..ImportFrame::IDENTITY
        },
        ImportFrame {
            translation_mm: [0., f64::NAN, 0.],
            ..ImportFrame::IDENTITY
        },
    ] {
        let result = Surface::import(
            SurfaceId(8),
            Provenance {
                frame,
                ..provenance()
            },
            vertices.clone(),
            faces.clone(),
        );
        assert_eq!(result.unwrap_err(), GeometryError::InvalidFrame);
    }
    for value in [f64::NAN, f64::INFINITY, f64::MAX] {
        vertices[1].position[0] = value;
        let source = Provenance {
            unit: LengthUnit::Metre,
            ..provenance()
        };
        assert_eq!(
            Surface::import(SurfaceId(8), source, vertices.clone(), faces.clone()).unwrap_err(),
            GeometryError::NonFinitePosition(VertexId(20))
        );
    }
}

#[test]
fn invalid_ids_corners_and_faces_are_identified() {
    let (vertices, faces) = rectangle();
    let mut repeated = vertices.clone();
    repeated.push(vertices[0]);
    assert_eq!(
        import(repeated, faces.clone()).unwrap_err(),
        GeometryError::DuplicateVertex(VertexId(10))
    );
    for (triangles, error) in [
        (vec![], GeometryError::EmptySurface),
        (
            vec![faces[0], faces[0]],
            GeometryError::DuplicateFace(FaceId(4)),
        ),
        (
            vec![face(2, [10, 20, 99])],
            GeometryError::MissingVertex {
                face: FaceId(2),
                vertex: VertexId(99),
            },
        ),
        (
            vec![face(2, [10, 10, 20])],
            GeometryError::RepeatedCorner(FaceId(2)),
        ),
        (
            vec![faces[0], face(9, [30, 20, 10])],
            GeometryError::DuplicateTriangle(FaceId(4), FaceId(9)),
        ),
    ] {
        assert_eq!(import(vertices.clone(), triangles).unwrap_err(), error);
    }
}

#[test]
fn degeneracy_is_relative_and_stable_at_extreme_scales() {
    for scale in [1e-250, 1., 1e250] {
        let triangle = vec![face(1, [1, 2, 3])];
        let mut vertices = vec![
            vertex(1, [0.; 3]),
            vertex(2, [scale, 0., 0.]),
            vertex(3, [0., scale, 0.]),
        ];
        assert!(import(vertices.clone(), triangle.clone()).is_ok());
        for height in [0., f64::EPSILON / 8.] {
            vertices[2].position = [scale / 2., scale * height, 0.];
            assert_eq!(
                import(vertices.clone(), triangle.clone()).unwrap_err(),
                GeometryError::DegenerateFace(FaceId(1))
            );
        }
    }
    let large = f64::MAX;
    let vertices = vec![
        vertex(1, [-large, 0., 0.]),
        vertex(2, [large, 0., 0.]),
        vertex(3, [0., large, 0.]),
    ];
    assert!(import(vertices, vec![face(1, [1, 2, 3])]).is_ok());
}

#[test]
fn nonmanifold_edges_winding_conflicts_and_bowties_are_distinguished() {
    let (mut vertices, _) = rectangle();
    vertices.push(vertex(50, [0., 0., 1.]));
    for (mut faces, expected) in [
        (
            vec![
                face(1, [10, 20, 30]),
                face(2, [20, 10, 40]),
                face(3, [10, 20, 50]),
            ],
            GeometryError::NonManifoldEdge([VertexId(10), VertexId(20)]),
        ),
        (
            vec![face(1, [10, 20, 30]), face(2, [10, 20, 40])],
            GeometryError::InconsistentWinding([VertexId(10), VertexId(20)]),
        ),
        (
            vec![face(1, [10, 20, 30]), face(2, [10, 40, 50])],
            GeometryError::DisconnectedFan(VertexId(10)),
        ),
    ] {
        assert_eq!(
            import(vertices.clone(), faces.clone()).unwrap_err(),
            expected
        );
        faces.reverse();
        vertices.reverse();
        assert_eq!(import(vertices.clone(), faces).unwrap_err(), expected);
    }
}

#[test]
fn provenance_needs_a_source_identity_on_import_and_replay() {
    let (vertices, faces) = rectangle();
    let mut record = import(vertices.clone(), faces.clone())
        .unwrap()
        .into_record();
    record.provenance.source = "   ".into();
    assert_eq!(
        Surface::from_record(record).unwrap_err(),
        GeometryError::MissingSource
    );
    let source = Provenance {
        source: String::new(),
        ..provenance()
    };
    assert_eq!(
        Surface::import(SurfaceId(2), source, vertices, faces).unwrap_err(),
        GeometryError::MissingSource
    );
}

#[test]
fn near_flat_triangle_uses_longest_euclidean_edge_in_any_frame() {
    let eps = f64::EPSILON;
    for (b, c) in [
        ([1., 0., 0.], [0.5, 0.75 * eps, 0.]),
        ([1., 1., 0.], [0.5, 0.5 + 1.5 * eps, 0.]),
    ] {
        let vertices = vec![vertex(1, [0.; 3]), vertex(2, b), vertex(3, c)];
        assert_eq!(
            import(vertices, vec![face(1, [1, 2, 3])]).unwrap_err(),
            GeometryError::DegenerateFace(FaceId(1))
        );
    }
}

#[test]
fn consistently_wound_closed_tetrahedron_has_connected_vertex_fans() {
    let verts = vec![
        vertex(1, [0.; 3]),
        vertex(2, [1., 0., 0.]),
        vertex(3, [0., 1., 0.]),
        vertex(4, [0., 0., 1.]),
    ];
    let faces = vec![
        face(1, [1, 3, 2]),
        face(2, [1, 2, 4]),
        face(3, [2, 3, 4]),
        face(4, [3, 1, 4]),
    ];
    assert!(import(verts, faces).is_ok());
}
