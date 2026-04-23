# Hybrid / GPU Rework Plan (WASM Base)

This repository now includes an experimental crate at:

- `rust/hybrid-lab`

The objective is to redesign the core solve data-path for **speed-first local execution**, with memory usage allowed to increase.

## Baseline from upstream benchmark

Source: `README.md` in this repository (section "Comparison").

- WASM Postflop (16 threads, target 0.1%): **45.5 s**, memory **1.25 GB**
- Desktop Postflop (16 threads, target 0.1%): **27.9 s**, memory **1.27 GB**

These are the public baseline points used for directional comparison.

## What changed in this phase

`rust/hybrid-lab` introduces a new architecture prototype:

1. **Tree representation**
   - Legacy layout (`LegacyTree`): scattered per-node vectors.
   - New layout (`FlatTree`): node metadata + contiguous edge list + contiguous slot buffers.

2. **Regret / strategy storage**
   - New aligned host buffers (`AlignedF32Buffer`) for contiguous memory access.
   - Node metadata stores `slot_offset + slot_count` to avoid pointer chasing in hot loops.

3. **CPU/GPU transfer model**
   - `FlatTree` supports optional device mirror buffers.
   - Upload/download accounting is tracked via `TransferStats`.
   - Hybrid and device-only execution modes are implemented around this transfer model.
   - A real GPU compute backend (`wgpu-compute`) now executes the regret-matching kernel on device.

4. **Bench harness**
   - Executable benchmark that compares:
     - `legacy-scattered`
     - `flat-cpu`
     - `hybrid-mirror`
     - `device-only-mirror`

## Run benchmark

```bash
cd rust/hybrid-lab
cargo run --release -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
```

With GPU compute backend enabled:

```bash
cd rust/hybrid-lab
cargo run --release --features wgpu-backend -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
```

Optional knobs:

- `--depth`
- `--branching`
- `--hands`
- `--actions`
- `--iters`
- `--align`

## Integration path into real solver

1. Add a new runtime backend trait inside `postflop-solver`:
   - `LegacyCpu`
   - `FlatCpu`
   - `HybridGpu` (device buffer + overlapped transfers)

2. Build a one-time conversion pass after tree construction:
   - `PostFlopGame -> FlatTreeRuntime`
   - Keep lock-strategy and isomorphic mappings as side tables.

3. Move CFR hot kernels to flat slot loops first (CPU).

4. Add true device compute kernel for regret matching + cf update:
   - implemented in `hybrid-lab` with `wgpu` for flat slot updates
   - next: split into stage kernels (chance aggregation, terminal utility, regret update)
   - then extend to turn/river + chance aggregation parity with full engine

5. Keep feature flags to preserve current path:
   - `legacy-engine` (default until parity)
   - `flat-engine`
   - `hybrid-engine`

## Current status

- Prototype architecture and bench harness are in place.
- GPU kernel prototype is running and benchmarkable in `hybrid-lab`.
- Real solver integration is now wired:
  - flattened runtime metadata is built after allocation,
  - backend switch exists in the real CFR loop (`legacy` / `flat` / `wgpu`),
  - backend preparation is called before each player traversal.
- Important caveat:
  - on current real-spot benchmark (`examples/backend_bench`), `legacy` is still faster than `flat` and `wgpu`,
  - so UI fallback should keep `legacy` as safe default until full-kernel CFR stages are moved to device.
- Next step:
  - move more of the CFR update path (not only regret-matching normalization) to flat/GPU kernels to eliminate host readback bottlenecks.
