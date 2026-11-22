# FabricCraft architecture (proposal v0.1)

## Boundaries
1. **Reference geometry import** accepts meshes exported by the user's CAD application and converts them to canonical triangular surfaces with stable provenance IDs, units and transforms. Future B-rep adapters may import CAD files directly into the same read-only reference representation.
2. **Fabric and material domain** owns the editable coverage surface, panel boundaries, seams, darts, allowances and assembly. A fabric surface can follow a reference face or interpolate across support vertices and edges. Imported reference surfaces are read-only inputs for fit, panel placement or collisions. The fabric domain tracks warp/weft directions, thickness, mass per area, stretch/shear/bending parameters, ease and attachments.
3. **Panel optimization** owns seam paths on the reference mesh, candidate segmentation, multi-objective scoring and deterministic diagnostics. Users set seams in FabricCraft. The CLI accepts source vertex paths, and a future GUI can select the same vertices and edges visually. Cuts create separate panel topology while retaining the source model.
4. **Flattening**: conformal initialization, strain-aware nonlinear optimization, grain constraints, boundary matching, distortion reports. Reject invalid or self-intersecting output.
5. **Pattern production**: sewing lines vs cut lines, allowances, darts, notches, registration, units, SVG/DXF.
6. **Simulation API**: rest-shape mesh, topology, gravity, material laws, collision, friction, pinning, stitching, solver state and convergence reports. Interchangeable open-source backends.
7. **Document/command API**: versioned schema, deterministic command execution, dependency tracking, undo, CLI, tests.
8. **UI** is independent. The eframe/egui desktop app uses the OpenGL backend to display imported OBJ triangles with open edges and creases. Support picks edit a separate fabric boundary checked in the selected coverage projection. The skin triangulation and geometric development produce the sewing outline, and a checked uniform offset produces the cut line for preview, stock checks, and export. The GUI and CLI share reference import and geometric development.
9. **Optional SolidCraft adapter** supplies reference geometry and source revision information to the fabric workflow. Pin an upstream revision, probe its APIs and licenses before reuse, and keep the core workspace independent.

A future adjustable mannequin will use user-supplied body measurements to generate the same reference mesh representation. Keep the measurement definitions, units, and mannequin model version with that reference so the fabric workflow can reproduce it.

Future source updates will bring a new CAD export or revision into FabricCraft. Store coverage and seams with their reference revision. Regenerate the affected patterns when the source faces and edges can be matched reliably. Ask the user to relink a selection when a change deletes a reference or produces several plausible matches. OBJ vertex numbers identify a particular export. Persistent CAD IDs will need a verified source interface.

## Accuracy rules
- Define units and coordinate conventions at every boundary.
- Test analytic cases: plane, cylinder, cone, sphere/hemisphere, sewn cushion.
- Quantify area change, edge length mismatch, stretch/shear energy, inverted triangles and self-intersections.
- Report solver residuals, convergence status, and unmet constraints; never label unvalidated output as physically accurate.
- Physical textile calibration and cut/sew measurements are required for material-aware claims.

## Research candidates (not yet approved dependencies)
SolidCore (vendored in SolidCraft), geometry-central, libigl (component-specific license review), open-source nonlinear solvers, Rapier Cloth, libuipc. Evaluate exact revision, transitive licenses, build portability, numerical stability and Rust FFI before selection. GPU acceleration is optional; CPU reference implementations should remain available.

## Open questions
- Source geometry ownership and stable feature references.
- Mesh quality and remeshing strategies.
- Material parameter identification.
- Cloth collision fidelity vs reproducibility.
- Native SolidCraft feature timeline integration (requires upstream API support).

## Fabric material data boundary
Introduce a solver-independent material domain crate and versioned data schema per `docs/materials/MATERIAL_LIBRARY.md`. Keep measured physical properties, illustrative profiles, and backend-specific solver calibration distinct. No T2P research-only records in public releases.
