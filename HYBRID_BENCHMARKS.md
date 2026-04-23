# Hybrid Lab Benchmarks

Date: April 23, 2026

Command (CPU + hybrid mirror backends):

```bash
cd rust/hybrid-lab
cargo run --release -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
```

Output:

| Backend | Time (ms) | Slot updates/s | Upload (MB) | Download (MB) |
| --- | ---: | ---: | ---: | ---: |
| `legacy-scattered` | 339.31 | 283,211,296 | 0.00 | 0.00 |
| `flat-cpu` | 159.96 | 600,734,850 | 0.00 | 0.00 |
| `hybrid-mirror` | 181.48 | 529,522,864 | 274.93 | 278.60 |
| `device-only-mirror` | 185.11 | 519,119,118 | 139.30 | 3.67 |

Command (with real GPU compute kernel):

```bash
cd rust/hybrid-lab
cargo run --release --features wgpu-backend -- --depth 5 --branching 3 --hands 220 --actions 4 --iters 300 --align 64
```

Output:

| Backend | Time (ms) | Slot updates/s | Upload (MB) | Download (MB) |
| --- | ---: | ---: | ---: | ---: |
| `legacy-scattered` | 335.26 | 286,629,170 | 0.00 | 0.00 |
| `flat-cpu` | 160.12 | 600,147,234 | 0.00 | 0.00 |
| `hybrid-mirror` | 182.03 | 527,905,488 | 274.93 | 278.60 |
| `device-only-mirror` | 186.60 | 514,991,397 | 139.30 | 3.67 |
| `wgpu-compute` | 48.65 | 1,975,353,307 | 4.29 | 3.67 |

## Quick read

- The flat contiguous layout remains clearly faster than the scattered legacy layout.
- `wgpu-compute` now runs a true device kernel and reaches about **3.3x** the throughput of `flat-cpu` on this machine.
- Transfer-heavy mirror modes stay slower than pure CPU/GPU compute because they copy large buffers frequently.

## Relation to public benchmark tables

The public benchmark in `README.md` compares full-solver runtime across applications (WASM/Desktop/Pio/GTO+).
The `hybrid-lab` benchmark is a **kernel/data-path benchmark** and should be used as:

- early signal for architecture changes,
- regression guard while integrating flat/hybrid storage into the real solver loop.

## Real solver benchmark (postflop-solver backend switch)

Date: April 23, 2026

Command:

```bash
cd ../postflop-solver-upstream
cargo run --release --example backend_bench --no-default-features --features custom-alloc,rayon -- --iters 200 --target-pct 0.5
```

Output:

| Backend | Exploitability | Time (ms) |
| --- | ---: | ---: |
| `legacy` | 0.9110 | 253.41 |
| `flat` | 0.9110 | 1998.38 |

With WGPU feature:

```bash
cd ../postflop-solver-upstream
cargo run --release --example backend_bench --no-default-features --features custom-alloc,rayon,wgpu-backend -- --iters 200 --target-pct 0.5
```

| Backend | Exploitability | Time (ms) |
| --- | ---: | ---: |
| `legacy` | 0.9110 | 361.17 |
| `flat` | 0.9110 | 1936.71 |
| `wgpu` | 0.8364 | 3858.97 |

Quick read:

- The backend switch is now integrated in the real solver loop.
- The current WGPU path is functional, but not yet faster on this complete solve path.
- Main bottleneck remains host-side traversal and readback; more CFR stages must move to flat/GPU kernels before enabling WGPU as default.
