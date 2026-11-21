# Codex kickoff — FabricCraft

This is the starting instruction for a Codex session in the repository root. Read `AGENTS.md`, `README.md`, `PROJECT_INTENT.md`, `ARCHITECTURE.md`, `DEPENDENCIES.md`, `ROADMAP.md`, and `CONTRIBUTING.md` before making changes.

## First assignment: repository foundation only

1. Inspect the current repository. Do not assume any file or API exists without checking.
2. Verify the Rust workspace builds, formats, lints, and tests. If tools are unavailable, say so; do not claim success.
3. Review GitHub community files, CI, licenses, and links. Fix concrete defects with minimal changes.
4. Verify that no code or assets are copied from SolidCraft's restricted brand/interface components. Do not add SolidCraft as a dependency yet.
5. Propose (do not implement) the canonical geometry data model and initial benchmark fixtures in a short design document.
6. Produce a final report: files changed, commands and results, remaining risks, and the next three scoped GitHub issues.

## Rules

- No new third-party dependencies without a recorded license and suitability review.
- No automatic publishing, remote repository creation, GitHub organization changes, releases, or mass issue creation without explicit maintainer instruction.
- Keep all work reviewable; prefer a small branch and pull request.
- Never invent numerical validation or claim textile fit is physically accurate without measurements.
- Do not start the full solver, GUI, or cloth simulator in this first task.

## Suggested Codex prompt

> Read START_HERE_CODEX.md and the referenced project documents. Execute only the first repository-foundation assignment. Make minimal corrections, run available checks, and report verified results and remaining issues. Do not create remote GitHub resources or introduce new dependencies without asking.

## Fabric materials follow-up (research, not part of first coding assignment)
Read `docs/materials/MATERIAL_LIBRARY.md`. Preserve the five demonstration-only profiles. For actual T2P examples, use `tools/sample_t2p.py` with a locally obtained CSV only where its research-only license permits; never commit the output. Verify source rights before any public material data import.
