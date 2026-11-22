# Canonical geometry and initial benchmarks

**M0 design proposal.** The first interchange object represents an existing target surface supplied as a triangle mesh. Importers produce it and fabric panel tools consume it as a reference. A cover can identify a subset of its faces while keeping the source object's identity.

## Geometry contract

| Record | Proposed contents and invariants |
| --- | --- |
| Surface | Schema version, surface ID, vertices, faces and import provenance. Canonical space is right-handed, Z-up, in millimetres. Store finite double-precision coordinates. |
| Import provenance | Source identity, declared length unit, scale to millimetres, and source-to-canonical frame transform. Scale positions first, then apply the orthogonal axis mapping and translation in millimetres. Reverse face winding when the mapping reverses handedness. Converted coordinates are persisted and the recorded conversion is applied only at import. |
| Vertex and face | Each has an opaque ID scoped to the surface, stable across serialization and reordering. Each vertex carries a position. Each face has three ordered vertex IDs. Winding defines its front side. Keep source entity references when available. A topology-changing edit assigns IDs to new entities and records their source entities. |
| Panel and seam | A panel references source faces. Its 2D vertices carry panel-local IDs, positions in millimetres and source vertex IDs, so both banks of a cut can retain provenance. A seam pairs ordered boundary chains with an explicit endpoint correspondence. Derive adjacency from face edges. |

Validate units, finite coordinates, unique IDs, references, distinct triangle corners, face area and winding. Allow open boundaries. Reject nonmanifold edges and disconnected face fans at a vertex. Return a diagnostic code with the offending entity or input field. Document scale-aware degeneracy tolerances and include their units in benchmark reports.

## First implementation

The core accepts a list of 3D points and triangles in memory. Each triangle names its three points by ID, so reordering the input keeps the same connections. Callers supply the source identity, length unit, axis convention and translation. The library accepts millimetres, centimetres, metres and inches. It converts positions to millimetres and reverses triangle winding when the coordinate change reverses handedness.

`Surface::import` performs that conversion once. `Surface::from_record` checks a saved canonical record directly, which keeps a second import from rescaling its coordinates. Both entry points validate geometry and topology. The implemented frame supports signed axis permutations and translation. General rotations, file-format adapters, per-entity source references and panel/seam records are future work. The [core API](../crates/fabriccraft-core/src/geometry/mod.rs) includes an executable example.

## First headless fixtures

Generate these surfaces from recorded parameters, tessellation rules and seam choices. Each expected result follows from the input geometry and can be computed independently of a flattening solver.

| Fixture | Oracle and expected behavior |
| --- | --- |
| Triangulated rectangle | Compare against its original 2D coordinates after translation and rotation. Preserve edge lengths, signed orientation and area. Re-import in a second length unit and rotated frame, then compare canonical coordinates. Round-trip stable IDs and face winding. |
| Cylinder cut along one generator | Use the lateral faces of a regular prism of circumradius `R`, height `H` and `n` sides. Each developed strip has width `2 R sin(π/n)`. The rectangle spans `n` such widths and height `H`. Check paired seam lengths and independent panel-local IDs on the cut. |
| Cone cut from rim to apex | Use the lateral triangles of a pyramid with a regular base and equal slant edges. The development is a fan of the original triangles. Sum their apex angles for the sector angle and compare each rim edge with its original chord. |
| Triangulated hemisphere | Keep one patch with an interior vertex of positive angle deficit. Measure the distortion required to lay it flat. An isometry request must report the incompatibility. |
| Invalid input variants | Exercise a missing unit, dangling vertex ID, collinear triangle, edge shared by three faces, and two fans joined only at a vertex. Require the matching diagnostic for each case. |

For each run, record the input parameters, unit, tolerances and termination status. For a returned map, report maximum edge length error, paired seam mismatch where applicable, per-face area change, flipped triangles and overlapping triangle interiors. Choose numerical tolerances during fixture review. A future sewn-cushion trial needs its own measurement protocol and observed results before it can support fit claims.
