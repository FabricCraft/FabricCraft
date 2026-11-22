# FabricCraft architecture (proposal v0.1)

## Boundaries
1. **Reference geometry import** accepts meshes exported by the user's CAD application and converts them to canonical triangular surfaces with stable provenance IDs, units and transforms. Future B-rep adapters may import CAD files directly into the same reference representation.
2. **Fabric and material domain** owns the editable panels and their assembly. Imported reference surfaces are read-only inputs for fit, panel placement or collisions. The fabric domain tracks warp/weft directions, thickness, mass per area, stretch/shear/bending parameters, ease and attachments.
3. **Panel optimization**: hard seam constraints, candidate segmentation, multi-objective scoring and deterministic diagnostics.
4. **Flattening**: conformal initialization, strain-aware nonlinear optimization, grain constraints, boundary matching, distortion reports. Reject invalid or self-intersecting output.
5. **Pattern production**: sewing lines vs cut lines, allowances, darts, notches, registration, units, SVG/DXF.
6. **Simulation API**: rest-shape mesh, topology, gravity, material laws, collision, friction, pinning, stitching, solver state and convergence reports. Interchangeable open-source backends.
7. **Document/command API**: versioned schema, deterministic command execution, dependency tracking, undo, CLI, tests.
8. **UI**: independent egui/wgpu interface; do not copy restricted SolidCraft brand/UI components.
9. **Optional SolidCraft adapter** exchanges reference geometry. Pin an upstream revision, probe its APIs and licenses before reuse, and keep the core workspace independent.

A future adjustable mannequin will use user-supplied body measurements to generate the same reference mesh representation. Keep the measurement definitions, units, and mannequin model version with that reference so the fabric workflow can reproduce it.

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
