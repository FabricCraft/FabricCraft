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

`Surface::import` performs that conversion once. `Surface::from_record` checks a saved canonical record directly, which keeps a second import from rescaling its coordinates. Both entry points validate geometry and topology. The implemented frame supports signed axis permutations and translation. General rotations, additional file formats, per-entity source references and panel/seam records are future work. The [core API](../crates/fabriccraft-core/src/geometry/mod.rs) includes an executable example.

The `surfaces` command groups edge-connected triangles within 0.001 mm of a fixed seed face's plane, provided their normals point to the same side of that plane. A fixed seed prevents small changes from accumulating across a curved mesh. The exporter validates the selected patch separately for planarity, orientation and a simple closed boundary. OBJ groups are treated as separate surfaces and vertices are matched by index within each group.

The `develop` command builds a planar development by placing each triangle on the other side of the shared edge from its already placed neighbor. It computes the third point from its intrinsic distance along and perpendicular to that edge. A previously placed vertex must agree with the new estimate within the numerical closure tolerance. After placement, every triangle must retain positive orientation. Relative errors in edge length and face area must each be at most `1e-8`. The absolute edge and closure threshold is `1e-7 mm` plus a double-precision roundoff term of 128 machine epsilons times the largest absolute developed coordinate of the points being compared. The same closed-outline checks used for planar panels reject crossings. [Developability of Triangle Meshes](https://www.cs.cmu.edu/~kmcrane/Projects/DiscreteDevelopable/paper.pdf)

For an uncapped tube, FabricCraft checks that the validated surface is connected, has two boundary loops and has Euler characteristic zero. The explicit seam is a simple path of mesh edges that meets each rim once. The automatic option uses Dijkstra's algorithm with 3D edge lengths as its costs. FabricCraft splits the mesh corners across the chosen path and preserves source face IDs. It then develops the cut mesh, compares corresponding edges on the two seam sides, and exports them with the outline. The eight-sided [uncut cylinder](../examples/uncut-cylinder.obj) is a fixture for this workflow. [Dijkstra's original paper](https://doi.org/10.1007/BF01386390)

For a complete [capped cylinder](../examples/capped-cylinder.obj), excluding the two cap patches inside FabricCraft produces the same lateral coverage. Selecting faces preserves source vertex and face IDs, and the seam can use the original imported vertex numbers. The filtered surface and cut are used for the pattern, with the original file retained as the reference.

## First headless fixtures

Generate these surfaces from recorded parameters, tessellation rules and seam choices. Each expected result follows from the input geometry and can be computed independently of a flattening solver.

| Fixture | Oracle and expected behavior |
| --- | --- |
| Triangulated rectangle | Compare against its original 2D coordinates after translation and rotation. Preserve edge lengths, signed orientation and area. Re-import in a second length unit and rotated frame, then compare canonical coordinates. Round-trip stable IDs and face winding. |
| Closed box | Identify six patches in a single mesh and export each patch at one-to-one scale. The [simple box](../examples/reference-box.obj) has a 200 × 100 mm top face. An OBJ converted from the [SolidCore probe](research/SOLIDCRAFT.md) produced six correct outlines across 108 triangles. |
| Cylinder cut along one generator | Use the lateral faces of a regular prism of circumradius `R`, height `H` and `n` sides. Each developed strip has width `2 R sin(π/n)`. The rectangle spans `n` such widths and height `H`. The [example OBJ](../examples/cut-cylinder.obj) uses `R = 50 mm`, `H = 120 mm` and `n = 8`. Check paired seam lengths and independent panel-local IDs on the cut. |
| Cone cut from rim to apex | Use the lateral triangles of a pyramid with a regular base and equal slant edges. The development is a fan of the original triangles. Sum their apex angles for the sector angle and compare each rim edge with its original chord. |
| Triangulated hemisphere | Keep one patch with an interior vertex of positive angle deficit. Measure the distortion required to lay it flat. An isometry request must report the incompatibility. |
| Invalid input variants | Exercise a missing unit, dangling vertex ID, collinear triangle, edge shared by three faces, and two fans joined only at a vertex. Require the matching diagnostic for each case. |

For each run, record the input parameters, unit, tolerances and termination status. For a returned map, report maximum edge length error, paired seam mismatch where applicable, per-face area change, flipped triangles and overlapping triangle interiors. Choose numerical tolerances during fixture review. A future sewn-cushion trial needs its own measurement protocol and observed results before it can support fit claims.
