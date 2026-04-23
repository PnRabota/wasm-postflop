#[derive(Debug, Clone)]
pub struct NodeLegacy {
    pub first_child: u32,
    pub child_count: u16,
    pub hand_count: u16,
    pub action_count: u16,
    pub regrets: Vec<f32>,
    pub strategy: Vec<f32>,
    pub cfvalues: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct LegacyTree {
    pub nodes: Vec<NodeLegacy>,
    pub edges: Vec<u32>,
}

impl LegacyTree {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn total_slots(&self) -> usize {
        self.nodes.iter().map(|node| node.regrets.len()).sum()
    }
}

pub fn build_synthetic_tree(
    depth: usize,
    branching: usize,
    hand_count: usize,
    action_count: usize,
) -> LegacyTree {
    assert!(branching > 0, "branching must be > 0");
    assert!(hand_count > 0, "hand_count must be > 0");
    assert!(action_count > 0, "action_count must be > 0");
    assert!(
        hand_count <= u16::MAX as usize,
        "hand_count must fit in u16 for synthetic tree"
    );
    assert!(
        action_count <= u16::MAX as usize,
        "action_count must fit in u16 for synthetic tree"
    );

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seed = 0x0C0F_FEE1_u64;
    build_recursive(
        &mut nodes,
        &mut edges,
        depth,
        branching,
        hand_count as u16,
        action_count as u16,
        &mut seed,
    );
    LegacyTree { nodes, edges }
}

fn build_recursive(
    nodes: &mut Vec<NodeLegacy>,
    edges: &mut Vec<u32>,
    depth_left: usize,
    branching: usize,
    hand_count: u16,
    action_count: u16,
    seed: &mut u64,
) -> u32 {
    let node_index = nodes.len() as u32;
    nodes.push(NodeLegacy {
        first_child: 0,
        child_count: 0,
        hand_count,
        action_count,
        regrets: Vec::new(),
        strategy: Vec::new(),
        cfvalues: Vec::new(),
    });

    let child_count = if depth_left > 0 { branching as u16 } else { 0 };
    let first_child = edges.len() as u32;
    if child_count > 0 {
        for _ in 0..child_count {
            let child = build_recursive(
                nodes,
                edges,
                depth_left - 1,
                branching,
                hand_count,
                action_count,
                seed,
            );
            edges.push(child);
        }
    }

    let slot_count = (hand_count as usize) * (action_count as usize);
    let mut regrets = vec![0.0; slot_count];
    let mut strategy = vec![0.0; slot_count];
    let mut cfvalues = vec![0.0; slot_count];

    for idx in 0..slot_count {
        let noise = lcg(seed);
        let centered = (noise - 0.5) * 2.0;
        regrets[idx] = centered * 0.25;
        strategy[idx] = 1.0 / action_count as f32;
        cfvalues[idx] = centered * 0.5;
    }

    nodes[node_index as usize] = NodeLegacy {
        first_child,
        child_count,
        hand_count,
        action_count,
        regrets,
        strategy,
        cfvalues,
    };
    node_index
}

#[inline]
fn lcg(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let bits = ((*seed >> 32) & 0xFFFF_FFFF) as u32;
    bits as f32 / u32::MAX as f32
}
