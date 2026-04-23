struct KernelParams {
  iter_bias: f32,
  work_count: u32,
  _pad0: u32,
  _pad1: u32,
}

struct NodeMeta {
  slot_offset: u32,
  hand_count: u32,
  action_count: u32,
  _pad: u32,
}

struct WorkItem {
  node_index: u32,
  hand_index: u32,
}

@group(0) @binding(0)
var<uniform> params: KernelParams;

@group(0) @binding(1)
var<storage, read> nodes: array<NodeMeta>;

@group(0) @binding(2)
var<storage, read> work_items: array<WorkItem>;

@group(0) @binding(3)
var<storage, read_write> regrets: array<f32>;

@group(0) @binding(4)
var<storage, read_write> strategy: array<f32>;

@group(0) @binding(5)
var<storage, read_write> cfvalues: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let work_index = global_id.x;
  if (work_index >= params.work_count) {
    return;
  }

  let work = work_items[work_index];
  let node = nodes[work.node_index];
  if (node.hand_count == 0u || node.action_count == 0u || work.hand_index >= node.hand_count) {
    return;
  }

  var positive_sum: f32 = 0.0;
  var action: u32 = 0u;
  loop {
    if (action >= node.action_count) {
      break;
    }
    let slot = node.slot_offset + action * node.hand_count + work.hand_index;
    let value = max(regrets[slot], 0.0);
    strategy[slot] = value;
    positive_sum = positive_sum + value;
    action = action + 1u;
  }

  if (positive_sum <= 0.00000001) {
    let uniform = 1.0 / f32(node.action_count);
    action = 0u;
    loop {
      if (action >= node.action_count) {
        break;
      }
      let slot = node.slot_offset + action * node.hand_count + work.hand_index;
      strategy[slot] = uniform;
      action = action + 1u;
    }
  } else {
    let inv = 1.0 / positive_sum;
    action = 0u;
    loop {
      if (action >= node.action_count) {
        break;
      }
      let slot = node.slot_offset + action * node.hand_count + work.hand_index;
      strategy[slot] = strategy[slot] * inv;
      action = action + 1u;
    }
  }

  action = 0u;
  loop {
    if (action >= node.action_count) {
      break;
    }
    let slot = node.slot_offset + action * node.hand_count + work.hand_index;
    let cf = cfvalues[slot];
    let strat = strategy[slot];
    let delta = cf - params.iter_bias;
    regrets[slot] = clamp(regrets[slot] + delta, -5000.0, 5000.0);
    cfvalues[slot] = (0.84 * cf) + (0.16 * strat);
    action = action + 1u;
  }
}
