use crate::backend::SolverBackend;
use crate::flat::{FlatTree, TransferStats};
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use std::sync::mpsc;
use wgpu::util::DeviceExt;

const WORKGROUP_SIZE: u32 = 256;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct NodeMetaGpu {
    slot_offset: u32,
    hand_count: u32,
    action_count: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct WorkItemGpu {
    node_index: u32,
    hand_index: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct KernelParams {
    iter_bias: f32,
    work_count: u32,
    _pad0: u32,
    _pad1: u32,
}

struct WgpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    params_buffer: wgpu::Buffer,
    regrets_buffer: wgpu::Buffer,
    strategy_buffer: wgpu::Buffer,
    cfvalues_buffer: wgpu::Buffer,
    slot_count: usize,
    work_count: u32,
    dispatch_x: u32,
}

pub struct WgpuComputeBackend {
    state: WgpuState,
    stats: TransferStats,
}

impl WgpuComputeBackend {
    pub fn try_new(tree: &FlatTree) -> Result<Self, String> {
        if tree.total_slots() == 0 {
            return Err("wgpu backend disabled: tree has no slots".to_string());
        }

        let nodes: Vec<NodeMetaGpu> = tree
            .nodes
            .iter()
            .map(|node| NodeMetaGpu {
                slot_offset: node.slot_offset,
                hand_count: node.hand_count as u32,
                action_count: node.action_count as u32,
                _pad: 0,
            })
            .collect();

        let total_work_items: usize = tree.nodes.iter().map(|node| node.hand_count as usize).sum();
        let work_count = u32::try_from(total_work_items)
            .map_err(|_| "wgpu backend disabled: work-item count exceeds u32".to_string())?;
        if work_count == 0 {
            return Err("wgpu backend disabled: no hand work items".to_string());
        }

        let mut work_items = Vec::with_capacity(total_work_items);
        for (node_index, node) in tree.nodes.iter().enumerate() {
            for hand_index in 0..(node.hand_count as u32) {
                work_items.push(WorkItemGpu {
                    node_index: node_index as u32,
                    hand_index,
                });
            }
        }

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "wgpu backend disabled: no GPU adapter available".to_string())?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("hybrid-lab-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .map_err(|err| format!("wgpu backend disabled: failed to create device ({err})"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hybrid-lab-kernel"),
            source: wgpu::ShaderSource::Wgsl(include_str!("wgpu_kernel.wgsl").into()),
        });

        let params = KernelParams {
            iter_bias: 0.0,
            work_count,
            _pad0: 0,
            _pad1: 0,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hybrid-lab-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let nodes_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hybrid-lab-nodes"),
            contents: bytemuck::cast_slice(&nodes),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let work_items_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hybrid-lab-work-items"),
            contents: bytemuck::cast_slice(&work_items),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let regrets_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hybrid-lab-regrets"),
            contents: bytemuck::cast_slice(tree.regrets()),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let strategy_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hybrid-lab-strategy"),
            contents: bytemuck::cast_slice(tree.strategy()),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let cfvalues_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hybrid-lab-cfvalues"),
            contents: bytemuck::cast_slice(tree.cfvalues()),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hybrid-lab-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hybrid-lab-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: nodes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: work_items_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: regrets_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: strategy_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: cfvalues_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hybrid-lab-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("hybrid-lab-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        let dispatch_x = ((work_count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE).max(1);
        let slot_count = tree.total_slots();
        let mut stats = TransferStats::default();
        stats.upload_bytes = (nodes.len() * size_of::<NodeMetaGpu>()
            + work_items.len() * size_of::<WorkItemGpu>()
            + slot_count * size_of::<f32>() * 3
            + size_of::<KernelParams>()) as u64;
        stats.upload_ops = 6;

        Ok(Self {
            state: WgpuState {
                device,
                queue,
                pipeline,
                bind_group,
                params_buffer,
                regrets_buffer,
                strategy_buffer,
                cfvalues_buffer,
                slot_count,
                work_count,
                dispatch_x,
            },
            stats,
        })
    }

    fn readback_f32(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        source: &wgpu::Buffer,
        slot_count: usize,
        label: &str,
    ) -> Result<Vec<f32>, String> {
        let size_bytes = (slot_count * size_of::<f32>()) as wgpu::BufferAddress;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("hybrid-lab-readback-copy"),
        });
        encoder.copy_buffer_to_buffer(source, 0, &staging, 0, size_bytes);
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);

        let slice = staging.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        device.poll(wgpu::Maintain::Wait);

        let map_result = rx
            .recv()
            .map_err(|_| "wgpu readback failed: map callback channel closed".to_string())?;
        map_result.map_err(|err| format!("wgpu readback failed: {err:?}"))?;

        let data = slice.get_mapped_range();
        let out = bytemuck::cast_slice::<u8, f32>(&data).to_vec();
        drop(data);
        staging.unmap();
        Ok(out)
    }
}

impl SolverBackend for WgpuComputeBackend {
    fn name(&self) -> &'static str {
        "wgpu-compute"
    }

    fn run_iteration(&mut self, _tree: &mut FlatTree, iteration: u32) {
        let params = KernelParams {
            iter_bias: (iteration as f32) * 0.00017,
            work_count: self.state.work_count,
            _pad0: 0,
            _pad1: 0,
        };

        self.state
            .queue
            .write_buffer(&self.state.params_buffer, 0, bytemuck::bytes_of(&params));
        self.stats.upload_bytes += size_of::<KernelParams>() as u64;
        self.stats.upload_ops += 1;

        let mut encoder =
            self.state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("hybrid-lab-kernel-encoder"),
                });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("hybrid-lab-kernel-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.state.pipeline);
            pass.set_bind_group(0, &self.state.bind_group, &[]);
            pass.dispatch_workgroups(self.state.dispatch_x, 1, 1);
        }
        self.state.queue.submit(Some(encoder.finish()));
    }

    fn finish(&mut self, tree: &mut FlatTree) {
        self.state.device.poll(wgpu::Maintain::Wait);

        let slot_count = self.state.slot_count;
        let download_bytes = (slot_count * size_of::<f32>()) as u64;
        let regrets = match Self::readback_f32(
            &self.state.device,
            &self.state.queue,
            &self.state.regrets_buffer,
            slot_count,
            "hybrid-lab-read-regrets",
        ) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("{err}");
                return;
            }
        };
        self.stats.download_bytes += download_bytes;
        self.stats.download_ops += 1;

        let strategy = match Self::readback_f32(
            &self.state.device,
            &self.state.queue,
            &self.state.strategy_buffer,
            slot_count,
            "hybrid-lab-read-strategy",
        ) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("{err}");
                return;
            }
        };
        self.stats.download_bytes += download_bytes;
        self.stats.download_ops += 1;

        let cfvalues = match Self::readback_f32(
            &self.state.device,
            &self.state.queue,
            &self.state.cfvalues_buffer,
            slot_count,
            "hybrid-lab-read-cfvalues",
        ) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("{err}");
                return;
            }
        };
        self.stats.download_bytes += download_bytes;
        self.stats.download_ops += 1;

        tree.regrets_mut().copy_from_slice(&regrets);
        tree.strategy_mut().copy_from_slice(&strategy);
        tree.cfvalues_mut().copy_from_slice(&cfvalues);
    }

    fn transfer_stats(&self, _tree: &FlatTree) -> TransferStats {
        self.stats
    }
}
