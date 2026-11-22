# Reproduce the isolated research probes

Run these experiments in a separate scratch directory. FabricCraft's Cargo workspace has no build-time dependency on them. The upstream revisions are listed in the [flattening report](../FLATTENING_CANDIDATES.md) and [reference-import audit](../SOLIDCRAFT.md). Use the pinned checkouts available locally before building.

## Flattening

The C++ program generates all four test meshes mathematically. It calls one upstream API at a time and prints raw edge error, normalized edge and area errors, orientation counts, and seam mismatch.

Set `EIGEN`, `GC`, `IGL` and `OPENABF` to the corresponding checked-out roots. Set `PROBES` to the absolute path of this directory. Fetch geometry-central's pinned submodule with `git -C "$GC" submodule update --init --recursive`. Configure its static build with the same compiler as the probe. The commands below use shell syntax compatible with Bash and paths for the Windows GNU run. On Linux, the native output names can omit `.exe`.

```sh
cmake -S "$GC" -B gc-build -G Ninja \
  -DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
  -DSUITESPARSE=OFF -DGC_ALWAYS_DOWNLOAD_EIGEN=OFF \
  -DGC_EIGEN_LOCATION="$EIGEN" -DCMAKE_CXX_FLAGS=-DEIGEN_MPL2_ONLY
cmake --build gc-build

clang++ -std=c++17 -O2 -DEIGEN_MPL2_ONLY -DGC -I"$EIGEN" -I"$GC/include" \
  "$PROBES/flattening.cpp" gc-build/src/libgeometry-central.a -o bff.exe
clang++ -std=c++17 -O2 -DEIGEN_MPL2_ONLY -DIGL -I"$EIGEN" -I"$IGL/include" \
  "$PROBES/flattening.cpp" -o lscm.exe
clang++ -std=c++17 -O1 -DEIGEN_MPL2_ONLY -DPROBE_OPENABF -I"$EIGEN" \
  -I"$OPENABF/single_include" "$PROBES/flattening.cpp" -o openabf.exe

./bff.exe
./lscm.exe
./openabf.exe
```

## SolidCore tessellation

Create a fresh Cargo binary package in the scratch directory and use `solidcore_mesh.rs` as its `src/main.rs`. Add these entries under `[dependencies]`, replacing the paths with those in the pinned checkout.

```toml
solidcore-record = { path = "../solidcore/crates/solidcore-record" }
solidcore-tess = { path = "../solidcore/crates/solidcore-tess" }
serde_json = { version = "1", features = ["float_roundtrip"] }
```

Decompress the checked-out `crates/solidcore-tess/tests/data/box_2x3x4.json.gz` into `box.json` at the scratch package root. Generate and retain the scratch `Cargo.lock` for repeat runs. Record dependency versions for any new comparison since registry resolution can change.

```sh
cargo generate-lockfile
cargo fetch --locked
cargo run --locked --offline
```
