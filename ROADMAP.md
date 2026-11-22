# Roadmap

Milestones are acceptance-gated, not date commitments.

- **M0 Foundation:** compile Rust workspace; upstream compatibility/license matrix; canonical reference mesh schema; CI; first smoke test with an existing mesh.
- **M1 Flattening:** planar and developable surfaces; strain and distortion diagnostics; deterministic SVG; regression tests.
- **M2 Automatic panels:** curvature analysis, candidate seams, constrained segmentation, score comparisons.
- **M3 Manufacturing:** allowances, notches, grain, edge pairing, layout within usable fabric width and length, DXF, physical sewn-cushion validation.
- **M4 Basic drape:** gravity, rigid obstacle collision, fixed points, friction, reproducible convergence tests.
- **M5 Material-aware optimization:** measured warp/weft and shear properties, inverse-design iteration.
- **M6 Advanced assembly:** darts, pleats, gathering, sewn seams, self-contact.
- **M7 SolidCraft integration:** optional interchange for existing reference models, with a supported contract, source revision updates, and compatibility tests.

Each milestone requires documented tests and reproducible examples before completion.

## Future garment capability

Add an adjustable human mannequin as a reference for garment design. Accept user-entered body measurements with explicit units and documented measurement landmarks. Report which measurements were supplied and which dimensions come from the mannequin model. Verify that the generated reference reproduces supported measurements within documented tolerances before using it for fitting.
