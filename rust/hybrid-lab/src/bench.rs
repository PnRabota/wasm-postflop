use crate::backend::{
    kernel_step, CpuFlatBackend, DeviceOnlyBackend, HybridMirrorBackend, SolverBackend,
};
use crate::flat::{FlatTree, TransferStats};
use crate::tree::{build_synthetic_tree, LegacyTree};
#[cfg(feature = "wgpu-backend")]
use crate::wgpu_backend::WgpuComputeBackend;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct BenchConfig {
    pub depth: usize,
    pub branching: usize,
    pub hand_count: usize,
    pub action_count: usize,
    pub iterations: u32,
    pub alignment: usize,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            depth: 5,
            branching: 3,
            hand_count: 220,
            action_count: 4,
            iterations: 300,
            alignment: 64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchResult {
    pub name: &'static str,
    pub elapsed: Duration,
    pub node_count: usize,
    pub slot_count: usize,
    pub iterations: u32,
    pub transfer: TransferStats,
    pub note: Option<String>,
}

impl BenchResult {
    pub fn slot_updates_per_sec(&self) -> f64 {
        if self.elapsed.is_zero() {
            return 0.0;
        }
        let total_updates = (self.slot_count as f64) * (self.iterations as f64);
        total_updates / self.elapsed.as_secs_f64().max(1e-9)
    }

    pub fn skipped(name: &'static str, config: BenchConfig, note: String) -> Self {
        Self {
            name,
            elapsed: Duration::ZERO,
            node_count: 0,
            slot_count: 0,
            iterations: config.iterations,
            transfer: TransferStats::default(),
            note: Some(note),
        }
    }
}

pub fn run_bench_suite(config: BenchConfig) -> Vec<BenchResult> {
    let legacy = build_synthetic_tree(
        config.depth,
        config.branching,
        config.hand_count,
        config.action_count,
    );

    let mut results = Vec::new();
    results.push(run_legacy_bench(config, legacy.clone()));

    let mut flat_cpu = FlatTree::from_legacy(&legacy, config.alignment);
    results.push(run_flat_bench(config, &mut flat_cpu, CpuFlatBackend));

    let mut flat_hybrid = FlatTree::from_legacy(&legacy, config.alignment);
    results.push(run_flat_bench(
        config,
        &mut flat_hybrid,
        HybridMirrorBackend::new(4, 4),
    ));

    let mut flat_device = FlatTree::from_legacy(&legacy, config.alignment);
    results.push(run_flat_bench(
        config,
        &mut flat_device,
        DeviceOnlyBackend::new(8),
    ));

    #[cfg(feature = "wgpu-backend")]
    {
        let mut flat_gpu = FlatTree::from_legacy(&legacy, config.alignment);
        match WgpuComputeBackend::try_new(&flat_gpu) {
            Ok(backend) => results.push(run_flat_bench(config, &mut flat_gpu, backend)),
            Err(err) => results.push(BenchResult::skipped("wgpu-compute", config, err)),
        }
    }

    results
}

fn run_legacy_bench(config: BenchConfig, mut legacy: LegacyTree) -> BenchResult {
    let started = Instant::now();
    for iteration in 0..config.iterations {
        legacy_step(&mut legacy, iteration);
    }
    BenchResult {
        name: "legacy-scattered",
        elapsed: started.elapsed(),
        node_count: legacy.node_count(),
        slot_count: legacy.total_slots(),
        iterations: config.iterations,
        transfer: TransferStats::default(),
        note: None,
    }
}

fn run_flat_bench<B: SolverBackend>(
    config: BenchConfig,
    tree: &mut FlatTree,
    mut backend: B,
) -> BenchResult {
    let started = Instant::now();
    for iteration in 0..config.iterations {
        backend.run_iteration(tree, iteration);
    }
    backend.finish(tree);

    BenchResult {
        name: backend.name(),
        elapsed: started.elapsed(),
        node_count: tree.nodes.len(),
        slot_count: tree.total_slots(),
        iterations: config.iterations,
        transfer: backend.transfer_stats(tree),
        note: None,
    }
}

fn legacy_step(tree: &mut LegacyTree, iteration: u32) {
    let iter_bias = (iteration as f32) * 0.00017;
    for node in &mut tree.nodes {
        let actions = node.action_count as usize;
        let hands = node.hand_count as usize;
        if actions == 0 || hands == 0 {
            continue;
        }

        let slots = actions * hands;
        if node.regrets.len() != slots {
            continue;
        }

        for hand in 0..hands {
            let mut positive_sum = 0.0f32;
            for action in 0..actions {
                let idx = action * hands + hand;
                let value = node.regrets[idx].max(0.0);
                node.strategy[idx] = value;
                positive_sum += value;
            }

            if positive_sum <= 1e-8 {
                let uniform = 1.0 / actions as f32;
                for action in 0..actions {
                    let idx = action * hands + hand;
                    node.strategy[idx] = uniform;
                }
            } else {
                let inv = 1.0 / positive_sum;
                for action in 0..actions {
                    let idx = action * hands + hand;
                    node.strategy[idx] *= inv;
                }
            }
        }

        for idx in 0..slots {
            let cf = node.cfvalues[idx];
            let strat = node.strategy[idx];
            let delta = cf - iter_bias;
            node.regrets[idx] = (node.regrets[idx] + delta).clamp(-5000.0, 5000.0);
            node.cfvalues[idx] = (0.84 * cf) + (0.16 * strat);
        }
    }
}

#[allow(dead_code)]
fn _sanity_compare_one_step(legacy: &mut LegacyTree, flat: &mut FlatTree, iteration: u32) {
    legacy_step(legacy, iteration);
    flat.with_host_buffers(|nodes, regrets, strategy, cfvalues| {
        kernel_step(nodes, regrets, strategy, cfvalues, iteration);
    });
}
