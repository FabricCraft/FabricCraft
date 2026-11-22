use std::collections::BTreeSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read("box.json")?;
    let body: solidcore_record::brep::Brep = serde_json::from_slice(&source)?;
    body.validate()?;
    let spec = solidcore_tess::MeshSpec::chord(0.01);
    let diagonal = 29.0_f64.sqrt();
    let (plan, stats) = solidcore_tess::mesh(&body, &spec, diagonal, None)?;
    let (mesh, refused) = plan.welded()?;
    assert!(refused.is_empty() && stats.unmeshed.is_empty() && stats.lenient.is_empty());
    assert!(stats.over_tolerance.is_empty() && stats.edges_capped.is_empty());
    assert!(stats.unsampled_edges.is_empty() && stats.unshaped.is_empty());
    assert!(!mesh.triangles.is_empty());
    assert_eq!(mesh.triangles.len(), mesh.tri_face.len());
    let faces = mesh.tri_face.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(faces, BTreeSet::from([0, 1, 2, 3, 4, 5]));
    let mut area = 0.0;
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
        let u = [b.x - a.x, b.y - a.y, b.z - a.z];
        let v = [c.x - a.x, c.y - a.y, c.z - a.z];
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let twice = cross.iter().map(|n| n * n).sum::<f64>().sqrt();
        assert!(twice.is_finite() && twice > 0.0);
        area += twice * 0.5;
    }
    assert!((area - 52.0).abs() <= 1e-9);
    assert!(matches!(
        solidcore_tess::mesh(&body, &solidcore_tess::MeshSpec::chord(0.0), diagonal, None),
        Err(solidcore_tess::TessError::Invalid(_))
    ));
    println!(
        "vertices={} triangles={} faces={} area_mm2={area:.12} zero_chord=Invalid",
        mesh.positions.len(),
        mesh.triangles.len(),
        faces.len()
    );
    Ok(())
}
