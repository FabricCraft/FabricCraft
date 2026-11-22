# FabricCraft

**Status: pre-alpha. The desktop app can propose and preview a fabric skin across an imported frame. The CLI exports full-scale outlines for flat and developable panels.**

FabricCraft is a community-driven, open-source project for designing fabric panels on imported 3D models and producing 2D cutting and sewing patterns. It targets upholstery, fitted covers, garments and other textile applications. CAD supplies the read-only reference shape. FabricCraft owns the coverage, seams and fabric panel geometry. The long-term vision combines automatic panel segmentation, material-aware flattening, and gravity-driven cloth draping.

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

### Open the desktop app

On Windows, open `target\release\fabriccraft-gui.exe` after building it with the command below. The app opens a built-in furniture frame and suggests four corners spanning the open middle and the recessed brace.

```sh
cargo build --locked --release -p fabriccraft-cli --features gui --bin fabriccraft-gui
```

Drop in a triangulated OBJ or paste its path and choose the source unit. The viewport uses the imported triangles directly and shows the reference's open edges and creases, with coplanar diagonals hidden. Choose the side to cover and click **Suggest corners** to start from the convex footprint. Use **Pick vertex** or **Pick edge** to refine the boundary in any order. The app inserts supports where they add the least perimeter among valid placements in the coverage view. A picked edge stays together on the outline until you release it or remove one of its endpoints. You can span between separate mesh objects. Drag to orbit, scroll to zoom, or click **Coverage view** to look straight at the boundary.

The teal surface is a geometric estimate spanning the selected boundary at the supports' actual 3D positions. Sag, stretching, and collisions are future work. Open **Pattern** or **Side by side** to see the flattened piece update as you change its supports. Enter a uniform seam allowance in **mm**, **cm**, **m**, **in**, **ft**, or **yd**. Decimal measurements and fractions such as `1/2` and `1 1/4` work. The allowance, fabric width, fabric length, and OBJ units are independent.

**Export full-scale SVG** writes the cut line and, for a positive allowance, the dashed sewing line. The usable fabric size check uses the cut line. The current workflow edits one piece at a time and supports a single outline. If the allowance closes a gap or overlaps the outline, the app asks for a smaller allowance or an adjusted boundary.

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

### Select a surface and set its seam in FabricCraft

Import the complete cylinder as one triangulated OBJ object with shared mesh vertices. `surfaces` lists the two end caps first in the example. Exclude them from fabric coverage, then use `seam-vertices` to find the vertex numbers and connecting edges on the side. The seam joins the two rims and follows existing mesh edges.

```sh
cargo run --locked -p fabriccraft-cli -- surfaces examples/capped-cylinder.obj mm
cargo run --locked -p fabriccraft-cli -- seam-vertices examples/capped-cylinder.obj mm --exclude-surfaces "1,2"
cargo run --locked -p fabriccraft-cli -- develop examples/capped-cylinder.obj mm target/tube-panel.svg --exclude-surfaces "1,2" --seam "12,2" --fabric-mm 130 320
```

Use `--open-tube` in place of `--seam` to let FabricCraft pick the shortest mesh edge route. For a tube with open rims, omit `--exclude-surfaces`. Both operations work on the panel while keeping the CAD export intact. The SVG marks the two sides of the seam as dashed lines, records their measured mismatch, and uses zero seam allowance. The future GUI will let you choose the same coverage and seam paths on the model.

The prototype accepts up to 8 MiB, 100,000 triangles, and 4,096 boundary edges. Read the [geometry contract](docs/GEOMETRY_AND_FIXTURES.md) and [dependency review](docs/research/PLANAR_DEPENDENCIES.md) for algorithms and tolerances. The five bundled fabric profiles are illustrative metadata with empty measurement fields. [Material library specification](docs/materials/MATERIAL_LIBRARY.md)

For verification results and outstanding checks, read the [initial foundation review](docs/FOUNDATION_REVIEW.md).

## Contributing
Please read [CONTRIBUTING.md](CONTRIBUTING.md), [ARCHITECTURE.md](ARCHITECTURE.md), and [AGENTS.md](AGENTS.md). Open an issue to discuss algorithm changes before large implementations.

## License
FabricCraft contributors offer the project under your choice of the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE). FabricCraft is independent of SolidCraft. See the [third-party notices](THIRD_PARTY_NOTICES.md) for bundled dependencies.

## Project orientation

Start with [PROJECT_INTENT.md](PROJECT_INTENT.md) for the user-facing vision, [ARCHITECTURE.md](ARCHITECTURE.md) for proposed technical boundaries, [DEPENDENCIES.md](DEPENDENCIES.md) for reuse research, and [GOVERNANCE.md](GOVERNANCE.md) for decision-making.

**Using Codex?** Start with [START_HERE_CODEX.md](START_HERE_CODEX.md) and [AGENTS.md](AGENTS.md). The first task is repository verification, not implementing the entire application.
