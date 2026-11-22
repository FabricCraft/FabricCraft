# Third-party algorithm and dependency policy

No research candidate is an approved dependency until its current repository, revision, license, relevant subcomponents, transitive dependencies, maintenance status, portability, and API have been inspected. Record the findings in `docs/research/` with upstream links and pinned commits.

## Candidates to evaluate

- SolidCraft / SolidCore: optional import of existing CAD reference models, with code and brand licensing boundaries checked against the exact upstream revision.
- geometry-central: surface differential geometry and intrinsic mesh processing; inspect Rust FFI costs.
- libigl: mesh parameterization and deformation; evaluate individual components and optional dependency licenses separately.
- Open-source optimization libraries: constrained nonlinear solvers, sparse linear algebra, automatic differentiation, and robust predicates.
- Rapier Cloth: Rust-based cloth simulation candidate; independently verify availability, API, license, collision support, and stability.
- libuipc / IPC-family solvers: contact-intensive cloth simulation; assess hardware constraints, compilation, numerical behavior, and license.

## Selection rubric

For each candidate record: problem solved; evidence of correct behavior; benchmarks; platform support (Linux/Windows/macOS); Rust interoperability; exact license and required notices; dependency risks; maintenance; integration complexity; fallback; adoption decision.

Prefer a CPU reference path even if an optional GPU accelerator is added. Do not treat an attractive demo as validation of numerical accuracy.

## Fabric properties and data rights
See `docs/materials/MATERIAL_LIBRARY.md` for open exchange format evaluation, T2P research-only licensing, and solver-independent material schemas. Do not treat publicly downloadable research data as redistributable open-source data.
