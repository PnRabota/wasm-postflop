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

Date: April 24, 2026

Command:

```bash
cd ../postflop-solver-upstream
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example backend_bench --no-default-features --features custom-alloc,rayon -- --iters 200 --target-pct 0.5
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example backend_bench --no-default-features --features custom-alloc,rayon,wgpu-backend -- --iters 200 --target-pct 0.5
```

Output snapshot:

| Backend | Exploitability | Time (ms) |
| --- | ---: | ---: |
| `legacy` | 0.9110 | 186.88 |
| `flat` | 0.9110 | 183.63 |
| `wgpu` | 0.8364 | 1736.91 |

Quick read:

- The backend switch is integrated in the real solver loop.
- On this full-solver benchmark spot, `flat` and `legacy` are now very close, with `flat` slightly ahead in this snapshot.
- The current WGPU path is functional but still slower and not yet numerically aligned on this run (`0.8364` vs `0.9110` exploitability).
- Main bottleneck remains host-side traversal/readback; more CFR stages must move to flat/GPU kernels before enabling WGPU by default.

## Pio preset benchmark (this fork local anchor)

Date: April 24, 2026

Scenario:

- `postflop-solver-upstream/examples/pio_preset_bench.rs`
- `RAYON_NUM_THREADS=16`
- target exploitability: `0.1%` (of pot)
- memory mode: 32-bit float

Command:

```bash
cd ../postflop-solver-upstream
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon -- --iters 1000 --target-pct 0.1 --backend legacy
```

Runs:

| Run | Time (s) | Exploitability | Memory |
| --- | ---: | ---: | ---: |
| 1 | 29.43 | 0.1785 | 1.25 GB |
| 2 | 31.46 | 0.1785 | 1.25 GB |
| 3 | 35.21 | 0.1785 | 1.25 GB |

Average time: **32.03 s**

### Backend sweep (current integration snapshot)

Note: this sweep reflects the current in-progress backend integration state in `postflop-solver-upstream` (not the earlier stabilized anchor run above).

Command:

```bash
cd ../postflop-solver-upstream
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon -- --iters 1000 --target-pct 0.1 --backend legacy
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon -- --iters 1000 --target-pct 0.1 --backend flat
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon,wgpu-backend -- --iters 1000 --target-pct 0.1 --backend wgpu
```

Output:

| Backend | Exploitability | Time (s) | Status |
| --- | ---: | ---: | --- |
| `legacy` | 0.1785 | 32.32 | OK |
| `flat` | 0.1785 | 31.52 | OK |
| `wgpu` | 0.1785 | 30.29 | OK (experimental; oversized-buffer guard + fallback path enabled when needed) |

## Pio preset benchmark (Apr 24, 2026 optimization pass)

Command:

```bash
cd ../postflop-solver-upstream
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon -- --iters 1000 --target-pct 0.1 --backend legacy
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon -- --iters 1000 --target-pct 0.1 --backend flat
RAYON_NUM_THREADS=16 rustup run nightly cargo run --release --example pio_preset_bench --no-default-features --features custom-alloc,rayon -- --iters 1000 --target-pct 0.1 --backend flat --compression
```

Output:

| Backend | Compression | Exploitability | Time (s) | Memory |
| --- | --- | ---: | ---: | ---: |
| `legacy` | off | 0.1785 | 32.32 | 1.26 GB |
| `flat` | off | 0.1785 | 31.52 | 1.26 GB |
| `flat` | on | 0.1664 | 28.14 | 0.65 GB |

Quick read:

- The latest flat runtime path now matches/slightly beats legacy on this full preset.
- Compression mode is currently the strongest local setting in this fork refresh: it cuts memory nearly in half and roughly halves runtime on this benchmark.
