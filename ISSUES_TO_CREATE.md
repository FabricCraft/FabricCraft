# Initial GitHub issues (publish after repository creation)

## 1. [M0] Audit SolidCraft reusable crates and license boundaries
Labels: research, integration, M0
Inspect pinned upstream revision; build matrix of exact crates/APIs, dependencies, restricted assets and version drift. Acceptance: reproducible report with compile probes, commit SHA and license evidence.

## 2. [M0] Define canonical geometry and unit schema
Labels: architecture, geometry, M0
Define triangle mesh, stable IDs, transforms, millimeter conversion and error model. Acceptance: round-trip tests and degenerate input handling.

## 3. [M0] Survey permissive flattening and solver libraries
Labels: research, numerical, M0
Evaluate geometry-central, libigl components, optimization libraries, Rust bindings, licenses and benchmark fixtures. Acceptance: comparison with explicit reasons for adoption/rejection.

## 4. [M0] Add dependency license auditing to CI
Labels: infrastructure, good first issue, M0
Adopt a permissively licensed auditing tool, review allowlist and flag unknown/restricted licenses. Acceptance: CI fails for an intentionally disallowed test dependency.

## 5. [M1] Implement planar flattening with analytic validation
Labels: geometry, numerical, M1
Given a triangulated plane, recover a 2D map up to rigid transform. Acceptance: area/edge preservation within documented numerical tolerances and failure diagnostics.

## 6. [M1] Add cylinder/cone development test fixtures
Labels: geometry, testing, M1
Generate analytic triangulated developable surfaces and expected 2D reference patterns. Acceptance: deterministic fixtures, seam selection, error reports.

## 7. [M1] Implement distortion diagnostics and visualization schema
Labels: numerical, M1
Define strain, edge mismatch, inverted triangle and area change metrics. Acceptance: analytic cases, machine-readable report, tests.

## 8. [M1] Export unit-correct SVG pattern boundaries
Labels: io, good first issue, M1
Write closed 2D polylines with physical units and labels. Acceptance: round-trip dimensional checks and test fixture.

## 9. [M2] Research automatic seam placement objectives
Labels: research, optimization, M2
Document hard constraints, curvature heuristics, seam length, grain, panel count, manufacturability and comparison methodology. Acceptance: benchmark designs and solver API proposal.

## 10. [M4] Benchmark open-source draping solvers
Labels: research, simulation, M4
Evaluate pinned versions of Rapier Cloth, libuipc and other candidates: gravity, rigid collisions, self-contact, pins, convergence, material laws, licensing and portability. Acceptance: reproducible benchmark report and recommended initial backend.

## 11. [M0] Build minimal SolidCraft adapter spike
Labels: integration, M0
Compile against pinned upstream APIs without copying restricted UI assets. Acceptance: one verified mesh exchange operation, documented limitations and CI strategy.

## 12. [Community] Add first physical fabric validation protocol
Labels: documentation, testing, good first issue
Specify measurement procedure for a sewn rectangular cushion, reference geometry, seam tolerance and material properties. Acceptance: reproducible data sheet and pass/fail metrics.

## Material database research issue
**Title:** Design fabric material schema and evaluate open mechanical-property datasets

**Acceptance:** verify licenses for T2P and fabrics-drape-data; define physical units, provenance, uncertainty and missing-data handling; validate five demonstration profiles; document private research-only selection of five T2P records; no restricted rows committed; propose a measured-material contribution protocol.
