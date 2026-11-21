# Foundation review, 2026-11-21

The starter contains a core library with one schema-version function and one unit test. The CLI source prints the schema version. The [geometry and fixture proposal](GEOMETRY_AND_FIXTURES.md) defines the next design work.

The foundation changes add `Cargo.lock`, adjust formatting in both Rust source files, and update the GitHub workflow. The README adds local check commands. The contribution guide, security policy and code of conduct clarify reporting routes. The local sampler and its documentation now enforce the public data boundary.

## Local verification

The Windows checks used Rust and Cargo 1.100.0 on `x86_64-pc-windows-gnu`, with Cargo network access disabled.

| Check | Observed result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed. Both crates meet rustfmt’s formatting rules. |
| `cargo build --locked --workspace` | Passed. Both crates built. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed with zero warnings. |
| `cargo test --locked --workspace` | One core unit test passed, zero failed. The CLI has zero unit tests and the library has zero doctests. |
| `cargo run --locked -p fabriccraft-cli` | Exited 0 and printed `FabricCraft pre-alpha (schema 1)`. |
| `cargo metadata --locked --offline --format-version 1` | Resolved two workspace packages. The CLI depends only on the local core. Regenerating the lockfile offline reproduced the same package graph. |
| GitHub Actions | Formatting, build, Clippy, tests and the CLI smoke check passed on the hosted Linux runner. [Foundation CI run](https://github.com/FabricCraft/FabricCraft/actions/runs/44963460568). |
| Demonstration materials | All five JSON profiles parse. Each has illustrative provenance, demonstration status and eight null physical properties. |
| Local sampling helper | Using header-only temporary CSVs, the helper reported all five families absent. It wrote default and custom outputs under the ignored directory and rejected paths outside it, hard-linked output, reuse of the input path, and an invalid header. |

## Publication review

- The license texts, Cargo metadata, contributor terms and example profiles agree on `MIT OR Apache-2.0`. The existing copyright notices were retained. [MIT text](../LICENSE-MIT) and [Apache text](../LICENSE-APACHE).
- The workflow pins checkout to the v7.0.1 commit and disables persisted credentials. This MIT-licensed action checks out sources on a hosted runner. The runner’s rustup installs stable Rust, rustfmt and Clippy. A future stable toolchain can change lint results. [Workflow](../.github/workflows/ci.yml), [upstream checkout release](https://github.com/actions/checkout/releases/tag/v7.0.1) and [license at the pinned commit](https://raw.githubusercontent.com/actions/checkout/3d3c42e5aac5ba805825da76410c181273ba90b1/LICENSE).
- Inspection found no SolidCraft code, interface assets or vendored libraries. The starter arrived without version history. Any future reuse needs a review of the exact upstream revision and its terms.
- The public materials consist of five metadata profiles. Restricted source data and generated samples are outside the public tree. The sampler confines output to the ignored research directory. [Material data policy](materials/MATERIAL_LIBRARY.md).
- Private vulnerability reporting is enabled on the public organization repository. A confidential maintainer contact for conduct reports is still pending. [Code of conduct](../CODE_OF_CONDUCT.md) and [security policy](../SECURITY.md).

## Next three issues

1. **Audit SolidCraft and SolidCore at a pinned revision.** Record candidate crates, exact APIs, license and asset boundaries, and the result of an isolated compile probe.
2. **Implement the canonical mesh and unit schema.** Start with tests for unit conversion, ID round-trips, degenerate faces and nonmanifold input. Use the geometry proposal for the expected diagnostics.
3. **Evaluate permissive flattening libraries.** Compare candidate components on the proposed planar and developable fixtures. Record license obligations, numerical error and build requirements before selecting a dependency.

These are drafts from the [issue backlog](../ISSUES_TO_CREATE.md).
