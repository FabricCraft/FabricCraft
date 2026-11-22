# FabricCraft: project intent and scope

## Mission

Create a free and open-source, community-developed, Rust-first application for designing fabric panels and generating usable 2D cutting and sewing patterns. It should serve upholstery, furniture covers, soft goods, garments, and other fabric-on-geometry applications. When the fabric needs to fit a 3D shape, the user supplies an existing model as a reference.

## Intended user experience

1. Start with a fabric panel or import an existing 3D reference mesh for fitted work. Optional future adapters may import existing CAD models.
2. Select any reference surfaces to cover and choose the desired fit (loose drape, close fit, tensioned covering).
3. Optionally specify seams, darts, pleats, grain direction, attachments, fasteners, material, and construction constraints.
4. Generate candidate panel layouts and optimize them against geometric distortion and manufacturing constraints.
5. Flatten panels into dimensionally specified 2D sewing and cutting patterns with seam allowances, registration marks, labels, and exports.
6. Optionally simulate fabric falling or draping over geometry under gravity, with collisions, pins, friction, and later stitched seams.
7. Ultimately use forward cloth simulation to iteratively improve inverse-designed patterns.

## Core principles

- Numerical accuracy, reproducibility, and physically useful patterns are more important than runtime speed.
- Automation by default, with meaningful user override and transparent diagnostics.
- Reuse mature, legally compatible open-source algorithms rather than rewriting them unnecessarily.
- Rust-first modular architecture, allowing carefully isolated open-source C/C++ numerical backends where justified.
- Independent application that can eventually exchange geometry and workflows with SolidCraft; no dependency on restricted SolidCraft branding or interface assets.
- Both headless and interactive workflows; command-line tests must not depend on a GPU or GUI.
- No proprietary, closed-source, paid-only, or network-locked runtime/build components.

## Explicit limitations

A 3D surface cannot generally be flattened without distortion. Accurate fabric patterns require decisions about seam topology, allowable stretch/shear, ease, assembly, and real material behavior. The project must report infeasibility and error rather than silently generating misleading cut patterns. Cloth visualization is not proof of manufacturability or fit.

## Initial non-goals

- Production-ready clothing sizes or grading.
- Automatic perfect fits for arbitrary surfaces.
- Photorealistic rendering.
- Creating or editing the underlying 3D reference object. Fabric authoring covers panels, patterns, and their assembly.
- A full sewing CAD workbench in the first milestone.
- Direct integration into SolidCraft's native feature timeline before upstream interfaces are verified.
