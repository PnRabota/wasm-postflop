pub mod aligned;
pub mod backend;
pub mod bench;
pub mod flat;
pub mod tree;
#[cfg(feature = "wgpu-backend")]
pub mod wgpu_backend;

pub use backend::{CpuFlatBackend, HybridMirrorBackend, SolverBackend};
pub use bench::{run_bench_suite, BenchConfig, BenchResult};
pub use flat::{FlatTree, TransferStats};
pub use tree::{build_synthetic_tree, LegacyTree};
#[cfg(feature = "wgpu-backend")]
pub use wgpu_backend::WgpuComputeBackend;
