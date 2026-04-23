# Hybrid Lab Benchmarks

Date: April 22, 2026

Command (CPU + hybrid mirror backends):

```bash
cd rust/hybrid-lab
cargo run --release -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
```

Output:

| Backend | Time (ms) | Slot updates/s | Upload (MB) | Download (MB) |
| --- | ---: | ---: | ---: | ---: |
| `legacy-scattered` | 875.74 | 109,731,731 | 0.00 | 0.00 |
| `flat-cpu` | 377.49 | 254,567,370 | 0.00 | 0.00 |
| `hybrid-mirror` | 497.23 | 193,262,594 | 274.93 | 278.60 |
| `device-only-mirror` | 429.30 | 223,844,726 | 139.30 | 3.67 |

Command (with real GPU compute kernel):

```bash
cd rust/hybrid-lab
cargo run --release --features wgpu-backend -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
```

Output:

| Backend | Time (ms) | Slot updates/s | Upload (MB) | Download (MB) |
| --- | ---: | ---: | ---: | ---: |
| `legacy-scattered` | 892.12 | 107,716,395 | 0.00 | 0.00 |
| `flat-cpu` | 375.88 | 255,654,700 | 0.00 | 0.00 |
| `hybrid-mirror` | 497.45 | 193,175,861 | 274.93 | 278.60 |
| `device-only-mirror` | 430.09 | 223,430,357 | 139.30 | 3.67 |
| `wgpu-compute` | 94.27 | 1,019,391,522 | 4.29 | 3.67 |

## Quick read

- The flat contiguous layout remains clearly faster than the scattered legacy layout.
- `wgpu-compute` now runs a true device kernel and reaches about **4x** the throughput of `flat-cpu` on this machine.
- Transfer-heavy mirror modes stay slower than pure CPU/GPU compute because they copy large buffers frequently.

## Relation to public benchmark tables

The public benchmark in `README.md` compares full-solver runtime across applications (WASM/Desktop/Pio/GTO+).
The `hybrid-lab` benchmark is a **kernel/data-path benchmark** and should be used as:

- early signal for architecture changes,
- regression guard while integrating flat/hybrid storage into the real solver loop.

## Real solver benchmark (postflop-solver backend switch)

Date: April 22, 2026

Command:

```bash
cd ../postflop-solver-upstream
cargo run --release --example backend_bench --no-default-features --features custom-alloc,rayon -- --iters 200 --target-pct 0.5
```

Output:

| Backend | Exploitability | Time (ms) |
| --- | ---: | ---: |
| `legacy` | 0.9110 | 319.98 |
| `flat` | 0.9110 | 778.56 |

With WGPU feature:

```bash
cd ../postflop-solver-upstream
cargo run --release --example backend_bench --no-default-features --features custom-alloc,rayon,wgpu-backend -- --iters 200 --target-pct 0.5
```

| Backend | Exploitability | Time (ms) |
| --- | ---: | ---: |
| `legacy` | 0.9110 | 173.28 |
| `flat` | 0.9110 | 451.73 |
| `wgpu` | 0.8364 | 2406.13 |

Quick read:

- The backend switch is now integrated in the real solver loop.
- The current WGPU path is functional, but not yet faster on this complete solve path.
- Main bottleneck remains host-side traversal and readback; more CFR stages must move to flat/GPU kernels before enabling WGPU as default.
