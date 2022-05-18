#[derive(Debug)]
pub struct AdjacencyMatrix {
    array: Vec<bool>,
    dimension: usize,
}

impl AdjacencyMatrix {
    pub fn with_dimension(size: usize) -> Self {
        if size > 0 {
            let size = size * (size + 1) / 2;
            Self {
                array: vec![false; size],
                dimension: size,
            }
        } else {
            Self {
                array: vec![],
                dimension: size,
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    pub fn get_value(&self, rows_index: usize, columns_index: usize) -> Option<&bool> {
        self.array
            .get(self.calculate_index(rows_index, columns_index))
    }

    pub fn get_value_mut(&mut self, rows_index: usize, columns_index: usize) -> Option<&mut bool> {
        let index = self.calculate_index(rows_index, columns_index);
        self.array.get_mut(index)
    }

    pub fn set_value(&mut self, rows_index: usize, columns_index: usize, data: bool) {
        let index = self.calculate_index(rows_index, columns_index);
        self.array[index] = data;
    }

    #[inline]
    fn calculate_index(&self, rows_index: usize, columns_index: usize) -> usize {
        // if cfg!(debug_assertions) {
        //     self.check_boundaries(rows_index, columns_index);
        // }

        if rows_index < columns_index {
            columns_index * (columns_index + 1) / 2 + rows_index
        } else if rows_index == 0 {
            columns_index
        } else {
            rows_index * (rows_index + 1) / 2 + columns_index
        }
    }

    fn check_boundaries(&self, rows_index: usize, columns_index: usize) {
        assert!(
            rows_index <= self.dimension,
            "The row({}) must be smaller than the number of columns({})",
            rows_index,
            self.dimension,
        );
        assert!(
            columns_index <= self.dimension,
            "The column({}) must be smaller than the number of rows({})",
            columns_index,
            self.dimension,
        );
    }

    fn connect(&mut self, row_index: usize, column_index: usize) -> usize {
        let index = self.calculate_index(row_index, column_index);
        let len = self.array.len();
        if index >= len {
            self.array.reserve(len + 1);
            self.dimension += // Needs to be calculated;
        }
        self.array[index] = true;
        index
    }
}

#[derive(Debug)]
struct Node {}

impl Node {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Debug)]
struct Graph {
    nodes: Vec<Node>,
    connections: AdjacencyMatrix,
}

impl Graph {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: AdjacencyMatrix::with_dimension(0),
        }
    }

    fn add_node(&mut self) -> usize {
        let new_node = self.nodes.len();
        self.nodes.push(Node::new());
        new_node
    }

    fn connect(&mut self, node_1: usize, node_2: usize) -> usize {
        self.connections.connect(node_1, node_2)
    }
}

fn main() {
    let mut graph = Graph::new();
    let node_1 = graph.add_node();
    let node_2 = graph.add_node();
    let connection_1 = graph.connect(node_1, node_2);
    let connection_1 = graph.connect(node_1, node_2);
    let connection_1 = graph.connect(node_1, node_2);
    let connection_1 = graph.connect(node_1, node_2);
    let connection_1 = graph.connect(node_1, node_2);
    let connection_1 = graph.connect(node_1, node_2);
    println!("Graph: {:#?}", graph);
}
