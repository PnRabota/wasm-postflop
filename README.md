# WASM Postflop

> [!IMPORTANT]
> **As of October 2023, I have started developing a poker solver as a business and have decided to suspend development of this open-source project. See [this issue] for more information.**

[this issue]: https://github.com/b-inary/postflop-solver/issues/46

---

**WASM Postflop** is a free, open-source GTO solver for Texas hold'em poker that works on web browsers.

Website: https://wasm-postflop.pages.dev/

**Related repositories**
- Desktop application: https://github.com/b-inary/desktop-postflop
- Solver engine: https://github.com/b-inary/postflop-solver

![Image](image.png)

## Fork update (April 2026)

This fork includes a substantial engine + UX refresh on top of upstream:

- **Refreshed interface**:
  dark/light theme support, cleaner navigation, and improved premium-style layout.
- **Interactive combo inspector**:
  when hovering a hand in the range matrix, the right panel now shows combos in a **mosaic/tile view** (instead of only a flat list/table style), with strategy mix and EV context.
- **Backend selection plumbing**:
  solver runtime now supports backend selection labels (`legacy`, `flat`, experimental `wgpu`) and reports backend/runtime-node status in the run panel.
- **IAB/webview-safe worker fallback**:
  if multithreaded WASM cannot initialize (e.g. missing `SharedArrayBuffer` / non-isolated context / webview limitations), the app automatically falls back to **single-thread** solver so solving still works.
- **Experimental hybrid/GPU lab**:
  new `rust/hybrid-lab` crate for speed-first architecture research (flat memory layout, transfer model, optional WGPU kernel, benchmark harness).

For roadmap and benchmark notes from this fork:

- [`HYBRID_ENGINE_ROADMAP.md`](HYBRID_ENGINE_ROADMAP.md)
- [`HYBRID_BENCHMARKS.md`](HYBRID_BENCHMARKS.md)

## Why WASM Postflop?

The GTO (Game Theory Optimal) solver has become an indispensable tool for poker research.
However, unfortunately, there is a high barrier to trying out the GTO solver: the need to purchase expensive commercial software.
This project aims to overcome this situation by developing a free, open-source GTO solver.

Please note that this project does not intend to *replace* commercial GTO solvers.
They are great software, and it is not easy to create a new one that can compete with them.
This project intends to make the GTO solver more easily accessible to a broader audience.

### Features

- **Free to use**.
  The most important feature.
  Anyone can try out the solver for free!

- **Open source**.
  The implementation of the GTO solver is complex and is not easy to write down accurately.
  By making the program open source, we make it possible for anyone to examine the implementation.

- **Works on web browsers**.
  This feature brings several advantages.
  First, it allows for the solver to be easily accessible.
  Second, it naturally makes the solver a cross-platform application.
  Finally, it sandboxes the solver execution, so users do not have to worry about security.

- **Sufficiently fast**.
  Slow solvers are not wanted.
  By using WebAssembly, we have reduced the performance penalty of being a web application.
  We also supported multithreading and used a state-of-the-art algorithm ([Discounted CFR]).

[Discounted CFR]: https://arxiv.org/abs/1809.04040

## Comparison

### Local benchmark update (April 23, 2026)

[Desktop Postflop]: https://github.com/b-inary/desktop-postflop
[PioSOLVER Free]: https://www.piosolver.com/
[GTO+]: https://www.gtoplus.com/
[TexasSolver]: https://github.com/bupticybee/TexasSolver

Machine used for this update:

- CPU: Apple M4 (10 CPU cores)
- GPU: Apple M4 integrated GPU (8 cores, Metal 4)
- RAM: 16 GB

#### Hybrid data-path benchmark (fork internals)

Command:

```sh
./scripts/bench-hybrid.sh
USE_WGPU=1 ./scripts/bench-hybrid.sh
```

| Backend | Time (ms) | Slot updates/s | Upload (MB) | Download (MB) |
| --- | ---: | ---: | ---: | ---: |
| `legacy-scattered` (origin-like layout) | 335.26 | 286,629,170 | 0.00 | 0.00 |
| `flat-cpu` | 160.12 | 600,147,234 | 0.00 | 0.00 |
| `hybrid-mirror` | 182.03 | 527,905,488 | 274.93 | 278.60 |
| `device-only-mirror` | 186.60 | 514,991,397 | 139.30 | 3.67 |
| `wgpu-compute` | 48.65 | 1,975,353,307 | 4.29 | 3.67 |

#### Real solver benchmark (backend switch in full CFR loop)

Command:

```sh
cd ../postflop-solver-upstream
RAYON_NUM_THREADS=16 cargo run --release --no-default-features --features rayon,wgpu-backend --example pio_preset_bench -- --backend legacy --iters 200 --target-pct 0.1
RAYON_NUM_THREADS=16 cargo run --release --no-default-features --features rayon,wgpu-backend --example pio_preset_bench -- --backend flat --iters 200 --target-pct 0.1
RAYON_NUM_THREADS=16 cargo run --release --no-default-features --features rayon,wgpu-backend --example pio_preset_bench -- --backend flat --compression --iters 200 --target-pct 0.1
RAYON_NUM_THREADS=16 cargo run --release --no-default-features --features rayon,wgpu-backend --example pio_preset_bench -- --backend wgpu --iters 200 --target-pct 0.1
```

| Backend | Exploitability | Time |
| --- | ---: | ---: |
| `legacy` (origin path) | 0.1785 | 32.76 s |
| `flat` | 0.1785 | 32.75 s |
| `flat` + compression | 0.1664 | 30.94 s |
| `wgpu` (chunked runtime) | 0.1666 | 197.81 s |

> Note: on this full-solver spot, `flat` and `legacy` are now close, with `flat` slightly ahead in this snapshot.
> The current GPU path is now chunked and honors binding-size limits (no panic on oversized buffers), but remains transfer-bound and slower than CPU on this preset.

### External solver comparison (historical upstream reference)

The table below combines fresh local runs for this fork with the last published upstream cross-solver references (same 3betpotFAST methodology) to keep continuity against commercial and open-source tools.

| Solver | Time (Target 0.1%, 16 threads) | Memory |
| :--- | ---: | ---: |
| This fork (Apr 24, 2026 local run, `flat` + compression) | **30.9 s** | **0.65 GB** |
| This fork (Apr 24, 2026 local run, `flat`, uncompressed) | 32.8 s | 1.26 GB |
| WASM Postflop (upstream reference) | 45.5 s | 1.25 GB |
| Desktop Postflop (v0.2.1) | 27.9 s | 1.27 GB |
| PioSOLVER Free (2.0.8, 6-thread cap) | 60.1 s (6 threads) | 1.41 GB |
| GTO+ (v1.5.0) | 41.7 s | 705 MB |
| TexasSolver (v0.2.0) | 182.6 s | 2.84 GB |

`This fork` row details (Apr 24, 2026):
- `RAYON_NUM_THREADS=16`
- benchmark: `examples/pio_preset_bench` (preset matching `solve_pio_preset_normal`)
- `flat + compression`: `30.94 s`, exploitability `0.1664`, memory `0.65 GB`
- `flat` uncompressed: `32.75 s`, exploitability `0.1785`, memory `1.26 GB`
- `legacy` uncompressed: `32.76 s`, exploitability `0.1785`, memory `1.26 GB`
- `wgpu` chunked: `197.81 s`, exploitability `0.1666`, memory `1.26 GB`

## Build

```sh
$ # prerequisites
$ rustup install nightly
$ rustup +nightly component add rust-src
$ rustup target add wasm32-unknown-unknown
$ cargo install wasm-pack
$ npm install

$ # build
$ npm run wasm
$ npm run build

$ # serve
$ npm run serve

$ # lint/format
$ npm run lint
$ npm run format
```

## Runtime behavior in this fork

- **Default safe path**: CPU-oriented path remains the reliability baseline.
- **Experimental path**: `wgpu` backend wiring exists but is still experimental for end-to-end solve performance.
- **Threading fallback**:
  if multithread init fails, runtime falls back to single-thread backend automatically.
- **Localhost serving**:
  `server.js` sets `COOP/COEP` headers required by threaded WASM contexts.

## Experimental hybrid/GPU path

An experimental speed-first data path prototype is available in:

- `rust/hybrid-lab`

It includes:

- flat tree representation,
- aligned contiguous regret/strategy/cfvalue buffers,
- host/device mirror transfer model,
- benchmark harness for legacy vs flat vs hybrid-like execution modes,
- optional `wgpu` compute backend with a real on-device regret-matching kernel.

See `HYBRID_ENGINE_ROADMAP.md` for details.

### Run hybrid benchmark

```sh
$ ./scripts/bench-hybrid.sh
```

With WGPU kernel path enabled:

```sh
$ USE_WGPU=1 ./scripts/bench-hybrid.sh
```

## License

Copyright (C) 2022 Wataru Inariba

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program.  If not, see <https://www.gnu.org/licenses/>.
