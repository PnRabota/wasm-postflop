use crate::aligned::AlignedF32Buffer;
use crate::tree::LegacyTree;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub struct NodeMeta {
    pub first_child: u32,
    pub child_count: u16,
    pub hand_count: u16,
    pub action_count: u16,
    pub slot_offset: u32,
    pub slot_count: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TransferStats {
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_ops: u64,
    pub download_ops: u64,
}

#[derive(Debug, Clone)]
struct DeviceMirror {
    regrets: Vec<f32>,
    strategy: Vec<f32>,
    cfvalues: Vec<f32>,
    stats: TransferStats,
}

#[derive(Debug)]
pub struct FlatTree {
    pub nodes: Vec<NodeMeta>,
    pub edges: Vec<u32>,
    regrets: AlignedF32Buffer,
    strategy: AlignedF32Buffer,
    cfvalues: AlignedF32Buffer,
    device: Option<DeviceMirror>,
}

impl FlatTree {
    pub fn from_legacy(legacy: &LegacyTree, alignment: usize) -> Self {
        let total_slots = legacy.total_slots();
        let mut regrets = AlignedF32Buffer::new_zeroed(total_slots, alignment);
        let mut strategy = AlignedF32Buffer::new_zeroed(total_slots, alignment);
        let mut cfvalues = AlignedF32Buffer::new_zeroed(total_slots, alignment);

        let mut nodes = Vec::with_capacity(legacy.nodes.len());
        let mut offset = 0usize;
        for node in &legacy.nodes {
            let slot_count = node.regrets.len();
            let range = offset..offset + slot_count;
            regrets.as_mut_slice()[range.clone()].copy_from_slice(&node.regrets);
            strategy.as_mut_slice()[range.clone()].copy_from_slice(&node.strategy);
            cfvalues.as_mut_slice()[range.clone()].copy_from_slice(&node.cfvalues);

            nodes.push(NodeMeta {
                first_child: node.first_child,
                child_count: node.child_count,
                hand_count: node.hand_count,
                action_count: node.action_count,
                slot_offset: offset as u32,
                slot_count: slot_count as u32,
            });
            offset += slot_count;
        }

        Self {
            nodes,
            edges: legacy.edges.clone(),
            regrets,
            strategy,
            cfvalues,
            device: None,
        }
    }

    pub fn enable_device_mirror(&mut self) {
        if self.device.is_some() {
            return;
        }
        let len = self.total_slots();
        self.device = Some(DeviceMirror {
            regrets: vec![0.0; len],
            strategy: vec![0.0; len],
            cfvalues: vec![0.0; len],
            stats: TransferStats::default(),
        });
    }

    pub fn upload_to_device(&mut self) {
        let Some(mut device) = self.device.take() else {
            return;
        };
        device.regrets.copy_from_slice(self.regrets.as_slice());
        device.strategy.copy_from_slice(self.strategy.as_slice());
        device.cfvalues.copy_from_slice(self.cfvalues.as_slice());

        let bytes = (self.total_slots() * std::mem::size_of::<f32>() * 3) as u64;
        device.stats.upload_bytes += bytes;
        device.stats.upload_ops += 1;
        self.device = Some(device);
    }

    pub fn download_from_device(&mut self) {
        let Some(mut device) = self.device.take() else {
            return;
        };
        self.regrets.as_mut_slice().copy_from_slice(&device.regrets);
        self.strategy
            .as_mut_slice()
            .copy_from_slice(&device.strategy);
        self.cfvalues
            .as_mut_slice()
            .copy_from_slice(&device.cfvalues);

        let bytes = (self.total_slots() * std::mem::size_of::<f32>() * 3) as u64;
        device.stats.download_bytes += bytes;
        device.stats.download_ops += 1;
        self.device = Some(device);
    }

    pub fn transfer_stats(&self) -> TransferStats {
        self.device
            .as_ref()
            .map(|device| device.stats)
            .unwrap_or_default()
    }

    pub fn total_slots(&self) -> usize {
        self.regrets.len()
    }

    #[inline]
    pub fn slot_range(&self, node_index: usize) -> Range<usize> {
        let node = self.nodes[node_index];
        let start = node.slot_offset as usize;
        start..(start + node.slot_count as usize)
    }

    #[inline]
    pub fn regrets(&self) -> &[f32] {
        self.regrets.as_slice()
    }

    #[inline]
    pub fn regrets_mut(&mut self) -> &mut [f32] {
        self.regrets.as_mut_slice()
    }

    #[inline]
    pub fn strategy(&self) -> &[f32] {
        self.strategy.as_slice()
    }

    #[inline]
    pub fn strategy_mut(&mut self) -> &mut [f32] {
        self.strategy.as_mut_slice()
    }

    #[inline]
    pub fn cfvalues(&self) -> &[f32] {
        self.cfvalues.as_slice()
    }

    #[inline]
    pub fn cfvalues_mut(&mut self) -> &mut [f32] {
        self.cfvalues.as_mut_slice()
    }

    #[inline]
    pub fn with_host_buffers<R>(
        &mut self,
        f: impl FnOnce(&[NodeMeta], &mut [f32], &mut [f32], &mut [f32]) -> R,
    ) -> R {
        let nodes = &self.nodes;
        let regrets = self.regrets.as_mut_slice();
        let strategy = self.strategy.as_mut_slice();
        let cfvalues = self.cfvalues.as_mut_slice();
        f(nodes, regrets, strategy, cfvalues)
    }

    #[inline]
    pub fn with_device_buffers<R>(
        &mut self,
        f: impl FnOnce(&[NodeMeta], &mut [f32], &mut [f32], &mut [f32]) -> R,
    ) -> Option<R> {
        let nodes = &self.nodes;
        let device = self.device.as_mut()?;
        Some(f(
            nodes,
            device.regrets.as_mut_slice(),
            device.strategy.as_mut_slice(),
            device.cfvalues.as_mut_slice(),
        ))
    }
}
