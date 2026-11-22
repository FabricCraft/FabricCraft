# FabricCraft

**Status: pre-alpha. The CLI can export full-scale test outlines for flat surfaces and pre-cut developable mesh panels.**

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

### Select a surface on a 3D model

Export your CAD model as a triangulated Wavefront OBJ. Run `surfaces` to list the flat patches by number. The list gives each patch's object or group name, normal direction, and the center of its 3D bounds. The included box has six patches, with the top face listed first.

```sh
cargo run --locked -p fabriccraft-cli -- surfaces examples/reference-box.obj mm
cargo run --locked -p fabriccraft-cli -- panel examples/reference-box.obj mm target/box-top.svg --surface 1 --fabric-mm 110 210
```

Replace the model path and use the unit specified by your CAD exporter (`mm`, `cm`, `m` or `in`). Use a new output path. A patch uses edge-connected triangles with shared vertex indices within one OBJ object or group. Export requires a single outline without holes. Surface numbers are repeatable for the same export. Re-exporting can change them.

`--fabric-mm WIDTH LENGTH` is optional and always uses millimetres. Enter the **usable width across the fabric** first and the **available length along the roll** second. The check compares the outline's bounds against that rectangle as drawn and after a 90° turn. The box example fits the specified fabric only after a turn. The SVG records the result and keeps the outline's original orientation. Check the grain direction before turning a panel. This is a one-panel size check using zero seam allowance.

The SVG uses millimetres with a 1 mm page margin around the outline. Print at 100% scale and compare it with a known CAD dimension.

### Unwrap a pre-cut curved panel

`develop` takes one triangulated OBJ patch with a prepared seam. The mesh must have one outer boundary. Where the cut separates two seam banks, give them distinct vertex indices even when they share the same 3D positions. The included eight-sided cylinder opens to approximately 120 × 306.147 mm. Its width around the cylinder is the sum of eight straight mesh edges.

```sh
cargo run --locked -p fabriccraft-cli -- develop examples/cut-cylinder.obj mm target/cylinder-panel.svg --fabric-mm 130 320
```

FabricCraft reports the largest edge and face-area errors and only exports a compatible, non-overlapping outline. If the mesh fails to close, check the cut and whether the shape can flatten without stretching. The optional fabric check uses the same millimetre width and length as the flat workflow.

The prototype accepts up to 8 MiB, 100,000 triangles, and 4,096 boundary edges. Read the [geometry contract](docs/GEOMETRY_AND_FIXTURES.md) and [dependency review](docs/research/PLANAR_DEPENDENCIES.md) for algorithms and tolerances. The five bundled fabric profiles are illustrative metadata with empty measurement fields. [Material library specification](docs/materials/MATERIAL_LIBRARY.md)

For verification results and outstanding checks, read the [initial foundation review](docs/FOUNDATION_REVIEW.md).

## Contributing
Please read [CONTRIBUTING.md](CONTRIBUTING.md), [ARCHITECTURE.md](ARCHITECTURE.md), and [AGENTS.md](AGENTS.md). Open an issue to discuss algorithm changes before large implementations.

## License
FabricCraft contributors offer the project under your choice of the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE). FabricCraft is independent of SolidCraft. See the [third-party notices](THIRD_PARTY_NOTICES.md) for bundled dependencies.

## Project orientation

Start with [PROJECT_INTENT.md](PROJECT_INTENT.md) for the user-facing vision, [ARCHITECTURE.md](ARCHITECTURE.md) for proposed technical boundaries, [DEPENDENCIES.md](DEPENDENCIES.md) for reuse research, and [GOVERNANCE.md](GOVERNANCE.md) for decision-making.

**Using Codex?** Start with [START_HERE_CODEX.md](START_HERE_CODEX.md) and [AGENTS.md](AGENTS.md). The first task is repository verification, not implementing the entire application.
