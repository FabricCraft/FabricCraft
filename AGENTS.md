# Instructions for AI coding agents

Read README.md, ARCHITECTURE.md, ROADMAP.md and CONTRIBUTING.md before changing code.

- Work on one issue per branch; keep PRs small and reviewable.
- Accuracy, safety and repeatability take precedence over runtime speed.
- Prefer proven permissively licensed open-source libraries; verify exact license and transitive dependencies before adding them.
- Never copy restricted SolidCraft brand, look, theme or UI kit.
- Do not invent upstream SolidCraft APIs: inspect source at a pinned commit, compile an integration spike and record findings.
- Write tests first for geometry algorithms, including degenerate/nonmanifold meshes, units, numeric stability and solver failure.
- Never silently ignore convergence failures, inverted elements, seam mismatches or unsupported materials.
- Distinguish geometric flattening from validated textile simulation.
- Document commands run, results, risks, and unresolved assumptions in every PR.
- Do not create or publish releases or modify licensing without maintainer approval.

For the first session, follow `START_HERE_CODEX.md`. Treat `PROJECT_INTENT.md` as the product scope and `DEPENDENCIES.md` as a research checklist, not evidence that any candidate is integrated.

- Follow `docs/materials/MATERIAL_LIBRARY.md`; never invent mechanical measurements or bundle research-only T2P records.
