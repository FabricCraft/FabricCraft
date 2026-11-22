# Dependencies for the planar panel experiment

| Package | Scope and license | Reason for use |
| --- | --- | --- |
| `tobj` 4.0.5, `a54fea9d6ee5138bb95de0d2906e4e0323fe3524` | MIT. Default features disabled and `use_f64` enabled. The manifest has zero enabled transitive dependencies. | Parse Wavefront geometry using double precision. The CLI checks the input size and requires triangles. It supplies no material files to the parser and uses position indices independently of texture and normal indices. [Pinned source](https://github.com/Twinklebear/tobj/tree/a54fea9d6ee5138bb95de0d2906e4e0323fe3524) |
| `robust` 1.2.0, `654f34cb8cdb3ae21bf59ef3472f92652942cd74` | MIT or Apache-2.0. FabricCraft redistributes under the MIT option. The crate has zero runtime dependencies. | Reuse the adaptive orientation predicate for triangle winding and boundary intersections. [Pinned source](https://github.com/georust/robust/tree/654f34cb8cdb3ae21bf59ef3472f92652942cd74) |

The downloaded crates matched the published crates.io checksums. `cargo tree --locked` resolves only these two registry packages. The GUI, native-code bridges and curved flattening libraries are outside this experiment. Third-party license notices accompany any distributed build. [Third-party notices](../../THIRD_PARTY_NOTICES.md)

The flat fixture has a closed-form projection. A 3D orthonormal basis preserves lengths in its reference plane. The longest boundary edge sets the horizontal direction. The largest-face plane sets the normal. The projection accepts a maximum departure of 0.001 mm and requires consistent projected triangle orientation and a simple outline. The millimetre tolerance is for checking mesh coplanarity, independent of textile measurements.
