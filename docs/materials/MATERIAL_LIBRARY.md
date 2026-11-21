# FabricCraft material library — specification and licensing

## Objective
Maintain a solver-independent, unit-explicit, provenance-aware catalog of fabric behavior. Support measured, estimated and demonstration-only profiles; never imply generic profiles are validated for physical pattern manufacture.

## Required material schema (version 0.1 proposal)
- `schema_version`, `id`, `display_name`, `category`, `construction`, `composition`, `source`, `status` (`measured`, `estimated`, `demonstration`), `license`, `measurement_conditions`.
- Physical measurements with explicit units and uncertainty: `areal_density_kg_m2`, `thickness_m`, warp/weft tensile stress–strain curves, shear response, directional bending response, friction, damping, hysteresis and buckling if known.
- `null` means *not measured*; do not silently replace with zero or invented numbers.
- Each source measurement includes method, sample direction, sample count, temperature/humidity if available, and provenance URL/DOI.
- Distinguish physical properties from solver-specific calibration coefficients. Do not convert CLO3D stiffness fields to SI moduli by unit-label substitution; conversion requires documented constitutive interpretation and tests.
- Material schema and solver adapters must be separately versioned. Invalid units and missing required data must fail clearly.

## Sources to evaluate
1. Open material exchange schema: https://github.com/virtualtextiles/fabrics-drape-data (MIT; inspect and preserve attribution).
2. T2P: https://huggingface.co/datasets/image2garment/T2P (1,382 records; `research-only`). **Do not commit, bundle, redistribute, relicense, or use as a public app's default bundled material catalog without explicit permission.** For permitted research, the local-only script `tools/sample_t2p.py` selects five actual records for comparison; resulting CSV remains Git-ignored. Check current terms and access requirements before use.
3. Peer-reviewed KES/FAST fabric measurements and other openly licensed records: independently verify license per dataset, not merely open access of a paper.

## Starter example profiles
`materials/examples/` contains five *demonstration-only metadata profiles*, NOT copied T2P rows, and NOT measured stiffness data. They are suitable for exercising UI, schema validation, and material selection but not predictive draping.

## Codex tasks
1. Verify each upstream source and current license, and document an evidence table.
2. Implement typed Rust material structures, JSON serialization, schema validation, and tests for units, missing measurements and provenance.
3. Implement a local research-only T2P CSV importer behind an explicit user action; never include its output in releases, CI artifacts or example data.
4. Research licensing/permission to distribute a small representative T2P sample; until permission is granted, keep it local.
5. Define an open contribution protocol for measured materials, including rights confirmation and measurement methodology.
6. Add adapters for selected simulation solvers only after validating their constitutive models and parameter mappings.

## Representative local T2P selection
Keep the source CSV outside the checkout or under `materials/local-research/`. The helper writes its output under that ignored directory even when invoked from another working directory. A custom `--out` path must resolve inside the same directory.

Select one record each for: denim, jersey, chiffon, satin and twill, when available. These are *actual rows from the user's separately obtained dataset*, not generic invented substitutes. Record source CSV, row ID and missing-field flags. If a family is absent, report it rather than substituting without disclosure.
