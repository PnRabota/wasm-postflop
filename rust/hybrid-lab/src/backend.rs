use crate::flat::{FlatTree, TransferStats};

pub trait SolverBackend {
    fn name(&self) -> &'static str;
    fn run_iteration(&mut self, tree: &mut FlatTree, iteration: u32);
    fn finish(&mut self, _tree: &mut FlatTree) {}
    fn transfer_stats(&self, tree: &FlatTree) -> TransferStats {
        tree.transfer_stats()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuFlatBackend;

impl SolverBackend for CpuFlatBackend {
    fn name(&self) -> &'static str {
        "flat-cpu"
    }

    fn run_iteration(&mut self, tree: &mut FlatTree, iteration: u32) {
        tree.with_host_buffers(|nodes, regrets, strategy, cfvalues| {
            kernel_step(nodes, regrets, strategy, cfvalues, iteration);
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HybridMirrorBackend {
    pub upload_interval: u32,
    pub download_interval: u32,
}

impl HybridMirrorBackend {
    pub fn new(upload_interval: u32, download_interval: u32) -> Self {
        Self {
            upload_interval: upload_interval.max(1),
            download_interval: download_interval.max(1),
        }
    }
}

impl SolverBackend for HybridMirrorBackend {
    fn name(&self) -> &'static str {
        "hybrid-mirror"
    }

    fn run_iteration(&mut self, tree: &mut FlatTree, iteration: u32) {
        tree.enable_device_mirror();
        if iteration % self.upload_interval == 0 {
            tree.upload_to_device();
        }
        let _ = tree.with_device_buffers(|nodes, regrets, strategy, cfvalues| {
            kernel_step(nodes, regrets, strategy, cfvalues, iteration);
        });
        if iteration % self.download_interval == 0 {
            tree.download_from_device();
        }
    }

    fn finish(&mut self, tree: &mut FlatTree) {
        tree.download_from_device();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceOnlyBackend {
    pub upload_every: u32,
}

impl DeviceOnlyBackend {
    pub fn new(upload_every: u32) -> Self {
        Self {
            upload_every: upload_every.max(1),
        }
    }
}

impl SolverBackend for DeviceOnlyBackend {
    fn name(&self) -> &'static str {
        "device-only-mirror"
    }

    fn run_iteration(&mut self, tree: &mut FlatTree, iteration: u32) {
        tree.enable_device_mirror();
        if iteration % self.upload_every == 0 {
            tree.upload_to_device();
        }
        let _ = tree.with_device_buffers(|nodes, regrets, strategy, cfvalues| {
            kernel_step(nodes, regrets, strategy, cfvalues, iteration);
        });
    }

    fn finish(&mut self, tree: &mut FlatTree) {
        tree.download_from_device();
    }
}

pub(crate) fn kernel_step(
    nodes: &[crate::flat::NodeMeta],
    regrets: &mut [f32],
    strategy: &mut [f32],
    cfvalues: &mut [f32],
    iteration: u32,
) {
    let iter_bias = (iteration as f32) * 0.00017;
    for node in nodes {
        let actions = node.action_count as usize;
        let hands = node.hand_count as usize;
        if actions == 0 || hands == 0 {
            continue;
        }

        let start = node.slot_offset as usize;
        let slot_count = node.slot_count as usize;
        let expected = actions * hands;
        if slot_count != expected {
            continue;
        }

        for hand in 0..hands {
            let mut positive_sum = 0.0f32;
            for action in 0..actions {
                let idx = start + action * hands + hand;
                let value = regrets[idx].max(0.0);
                strategy[idx] = value;
                positive_sum += value;
            }

            if positive_sum <= 1e-8 {
                let uniform = 1.0 / actions as f32;
                for action in 0..actions {
                    let idx = start + action * hands + hand;
                    strategy[idx] = uniform;
                }
            } else {
                let inv = 1.0 / positive_sum;
                for action in 0..actions {
                    let idx = start + action * hands + hand;
                    strategy[idx] *= inv;
                }
            }
        }

        for offset in 0..slot_count {
            let idx = start + offset;
            let cf = cfvalues[idx];
            let strat = strategy[idx];
            let delta = cf - iter_bias;
            regrets[idx] = (regrets[idx] + delta).clamp(-5000.0, 5000.0);
            cfvalues[idx] = (0.84 * cf) + (0.16 * strat);
        }
    }
}
