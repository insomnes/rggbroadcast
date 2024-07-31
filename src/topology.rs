use rand::seq::SliceRandom;
pub struct Topology {
    pub node_id: String,
    pub all_nodes: Vec<String>,
    pub neighbors: Vec<String>,
}

impl Topology {
    pub fn new(node_id: String, all_nodes: Vec<String>, chunk_size: usize) -> Self {
        let all_chunks = all_nodes.chunks(chunk_size).collect::<Vec<_>>();
        let (i, chunk) = all_chunks
            .iter()
            .enumerate()
            .find(|(_, chunk)| chunk.contains(&node_id))
            .clone()
            .expect("Node ID not found in all_nodes");

        let i_in_chunk = chunk
            .iter()
            .position(|n| n == &node_id)
            .expect("Node ID not found in chunk");

        let neighbors = if chunk.len() <= all_nodes.len() {
            all_nodes
                .clone()
                .into_iter()
                .filter(|n| n != &node_id)
                .collect()
        } else {
            calculate_neighbors(&all_chunks, i, i_in_chunk)
        };
        eprintln!("Node ID: {}, Neighbors: {:?}", node_id, neighbors);

        Topology {
            node_id: node_id.clone(),
            all_nodes: all_nodes.into_iter().filter(|n| n != &node_id).collect(),
            neighbors,
        }
    }

    pub fn get_random_targets_from_all(&self, count: usize, exclude: &[String]) -> Vec<String> {
        get_random_targets_from(&self.all_nodes, count, exclude)
    }

    pub fn get_random_targets_from_neighbors(
        &self,
        count: usize,
        exclude: &[String],
    ) -> Vec<String> {
        get_random_targets_from(&self.neighbors, count, exclude)
    }
}

fn calculate_neighbors(all_chunks: &[&[String]], y: usize, x: usize) -> Vec<String> {
    let mut neighbors = vec![];
    let y_max = all_chunks.len() - 1;
    let chunk = all_chunks[y][x].clone();
    let x_max = chunk.len() - 1;

    for (y, x) in calculate_neighbors_coordinates_wrapped(y, x, y_max, x_max) {
        neighbors.push(all_chunks[y][x].clone());
    }

    neighbors
}

fn calculate_neighbors_coordinates_wrapped(
    y: usize,
    x: usize,
    max_y: usize,
    max_x: usize,
) -> Vec<(usize, usize)> {
    let y_max = max_y;
    let x_max = max_x;

    // Provide the neighbors on grid, left, right, up, down, and diagonals wrapped
    // around the grid
    let left = if x == 0 { x_max } else { x - 1 };
    let right = if x == x_max { 0 } else { x + 1 };
    let up = if y == 0 { y_max } else { y - 1 };
    let down = if y == y_max { 0 } else { y + 1 };
    if y_max == 0 {
        return vec![(y, left), (y, right)];
    }
    vec![
        (up, left),
        (up, x),
        (up, right),
        (y, left),
        (y, right),
        (down, left),
        (down, x),
        (down, right),
    ]
}

fn get_random_targets_from(targets: &[String], count: usize, expect: &[String]) -> Vec<String> {
    let mut rng = rand::thread_rng();
    targets
        .choose_multiple(&mut rng, count + expect.len())
        .filter(|n| !expect.contains(n))
        .take(count)
        .cloned()
        .collect()
}
