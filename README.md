# FabricCraft

**Status: pre-alpha. The CLI can export a test outline for one flat mesh panel.**

FabricCraft is a community-driven, open-source project for designing fabric panels on imported 3D models and producing 2D cutting and sewing patterns. It targets upholstery, fitted covers, garments and other textile applications. Create and edit the reference model in your CAD application, then bring it into FabricCraft to design the fabric. The long-term vision combines automatic panel segmentation, seam constraints, material-aware flattening, and gravity-driven cloth draping, with iterative inverse design.

## Principles
- Accuracy and validated manufacturing output over speed.
- Automatic generation with user control of seams, pleats, darts, grain, attachments and ease.
- Reuse mature open-source numerical methods; do not reimplement unnecessarily.
- Rust-first, with permissively licensed native dependencies where justified.
- Independent project with a future SolidCraft integration adapter; no SolidCraft brand/interface assets.
- All runtime and build dependencies must be open source. License compliance is checked before adoption.

## Planned first deliverable
A headless geometry-to-pattern experiment with a validated planar/cylindrical flattening test suite, distortion metrics, and SVG export. See [ROADMAP.md](ROADMAP.md).

## Run the starter workspace

Use Rust 1.88 or newer with the `rustfmt` and `clippy` components. Run these commands from the directory containing the workspace `Cargo.toml`.

```sh
cargo build --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo run --locked -p fabriccraft-cli
```

### Try a full-scale test panel

From the workspace directory, export the included 200 × 100 mm example to a new file.

```sh
cargo run --locked -p fabriccraft-cli -- panel examples/planar-panel-200x100.obj mm target/example-panel.svg
```

For your own model, export a single flat surface as a triangulated Wavefront OBJ. Use a single connected mesh with one outline and no holes. Replace the input path and provide the CAD export's unit (`mm`, `cm`, `m` or `in`). Use a new output path. FabricCraft checks the geometry and prints the pattern dimensions. The SVG uses millimetres with a 1 mm margin around the outline. Print at 100% scale and check the result against a known CAD dimension. The outline is a geometric test with zero seam allowance.

The prototype accepts up to 8 MiB, 100,000 triangles, and 4,096 boundary edges. Read the [geometry contract](docs/GEOMETRY_AND_FIXTURES.md) and [dependency review](docs/research/PLANAR_DEPENDENCIES.md) for algorithms and tolerances. The five bundled fabric profiles are illustrative metadata with empty measurement fields. [Material library specification](docs/materials/MATERIAL_LIBRARY.md)

For verification results and outstanding checks, read the [initial foundation review](docs/FOUNDATION_REVIEW.md).

## Contributing
Please read [CONTRIBUTING.md](CONTRIBUTING.md), [ARCHITECTURE.md](ARCHITECTURE.md), and [AGENTS.md](AGENTS.md). Open an issue to discuss algorithm changes before large implementations.

## License
FabricCraft contributors offer the project under your choice of the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE). FabricCraft is independent of SolidCraft. See the [third-party notices](THIRD_PARTY_NOTICES.md) for bundled dependencies.

## Project orientation

Start with [PROJECT_INTENT.md](PROJECT_INTENT.md) for the user-facing vision, [ARCHITECTURE.md](ARCHITECTURE.md) for proposed technical boundaries, [DEPENDENCIES.md](DEPENDENCIES.md) for reuse research, and [GOVERNANCE.md](GOVERNANCE.md) for decision-making.

**Using Codex?** Start with [START_HERE_CODEX.md](START_HERE_CODEX.md) and [AGENTS.md](AGENTS.md). The first task is repository verification, not implementing the entire application.
