# Roadmap

Milestones are acceptance-gated, not date commitments.

- **M0 Foundation:** compile Rust workspace; upstream compatibility/license matrix; canonical geometry schema; CI; first geometry import smoke test.
- **M1 Flattening:** planar and developable surfaces; strain and distortion diagnostics; deterministic SVG; regression tests.
- **M2 Automatic panels:** curvature analysis, candidate seams, constrained segmentation, score comparisons.
- **M3 Manufacturing:** allowances, notches, grain, edge pairing, DXF, physical sewn-cushion validation.
- **M4 Basic drape:** gravity, rigid obstacle collision, fixed points, friction, reproducible convergence tests.
- **M5 Material-aware optimization:** measured warp/weft and shear properties, inverse-design iteration.
- **M6 Advanced assembly:** darts, pleats, gathering, sewn seams, self-contact.
- **M7 SolidCraft integration:** supported interchange/plugin contract and compatibility tests; native workbench only if upstream supports it.

Each milestone requires documented tests and reproducible examples before completion.
