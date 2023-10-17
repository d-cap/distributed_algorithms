use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

mod stable_binary_tree;

/// A Hash tree implemented with an arena allocated binary tree
pub struct MerkleTree {
    data: Vec<u64>,
    hasher: Box<dyn Hasher>,
    nodes: usize,
}

impl MerkleTree {
    pub fn with_capacity(capacity: u32) -> Self {
        let capacity = if capacity % 2 == 0 {
            capacity
        } else {
            capacity + 1
        };
        Self {
            data: vec![0; 2_usize.pow(capacity / 2)],
            hasher: Box::<DefaultHasher>::default(),
            nodes: capacity as usize,
        }
    }

    pub fn insert<T: Hash>(&mut self, value: T)
    where
        T: Hash,
    {
        value.hash(&mut self.hasher);
        let hash = self.hasher.finish();
        let relative_index = hash as usize % self.nodes;
        let absolute_index = 2_usize.pow(self.nodes as u32 / 2) / 2 - 1 + relative_index;
        self.data[absolute_index] = hash;
        let mut parent_index = (absolute_index + 1) / 2 - 1;
        loop {
            parent_index = (parent_index + 1) / 2 - 1;
            if parent_index == 0 {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::hash::Hash;

    use super::*;

    #[derive(Hash)]
    struct TestValue {
        data1: String,
        data2: u64,
        data3: Vec<bool>,
    }

    #[test]
    fn should_create_with_capacity() {
        let mut tree = MerkleTree::with_capacity(8);
        let value = TestValue {
            data1: "test".to_owned(),
            data2: 1,
            data3: vec![true, false],
        };
        tree.insert(value);
    }
}
