use std::{
    collections::hash_map::DefaultHasher,
    fmt::Debug,
    hash::{Hash, Hasher},
};

#[derive(PartialEq, Eq, Debug)]
pub enum Child {
    Node(usize),
    Value(usize),
    None,
}

impl Child {
    fn is_value(&self) -> bool {
        match self {
            Child::Value(_) => true,
            Child::Node(_) | Child::None => false,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Node {
    pub hash: u64,
    pub left: Child,
    pub right: Child,
}

#[derive(PartialEq, Debug)]
pub struct Value<K, V> {
    pub hash: u64,
    pub key: K,
    pub value: V,
}

/// A Hash tree implemented with an arena allocated binary tree
#[derive(Default, Debug)]
pub struct MerkleTree<K, V> {
    pub hashes: Vec<Node>,
    pub data: Vec<Value<K, V>>,
    pub root: usize,
}

impl<K, V> MerkleTree<K, V>
where
    K: Ord + Hash,
{
    pub fn new() -> Self {
        Self {
            hashes: vec![],
            data: vec![],
            root: 0,
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let position = if self.data.is_empty() {
            0
        } else {
            search_index(&self.data, &key)
        };
        let hash_position = if position == 0 { 0 } else { position - 1 };
        let key_hash = hash(&key);
        if hash_position == 0 {
            if position == 0 {
                assert!(self.hashes.is_empty());
                self.hashes.push(Node {
                    hash: key_hash,
                    left: Child::Value(position),
                    right: Child::None,
                });
            } else {
                assert_eq!(self.hashes[hash_position].right, Child::None);
                self.hashes[hash_position].hash =
                    hash_two(&hash(&self.data[position - 1].key), &hash(&key));
                self.hashes[hash_position].right = Child::Value(position);
            }
        } else if self.hashes.len() < hash_position + 1 {
            let len = self.hashes.len();
            let last_hash = self.hashes.last_mut().unwrap();
            if !last_hash.left.is_value() && last_hash.right.is_value() {
                last_hash.right = Child::Node(len);
                let new_hash = hash_two(&hash(&self.data[position - 1].key), &key_hash);
                self.hashes.push(Node {
                    hash: new_hash,
                    left: Child::Value(position - 1),
                    right: Child::Value(position),
                });
                self.update_hashes(hash_position);
            } else {
                let left_child_position = Self::left_node_index(hash_position);
                let left_hash = self.hashes[left_child_position].hash;
                self.hashes.push(Node {
                    hash: hash_two(&left_hash, &key_hash),
                    left: Child::Node(left_child_position),
                    right: Child::Value(position),
                });
                let level = node_level(position);
                let parent = position - 2_usize.pow(level as u32);
                let parent_level = node_level(parent);
                let parent_offset = 2_usize.pow(parent_level as u32);
                if parent > parent_offset {
                    let parent = parent - parent_offset;
                    if parent < self.hashes.len() {
                        self.hashes[parent].right = Child::Node(self.hashes.len() - 1);
                        self.update_hashes(hash_position);
                    }
                }
            }
        }

        self.data.insert(
            position,
            Value {
                hash: key_hash,
                key,
                value,
            },
        );
        self.root = highest_power_of_2(self.hashes.len()) as usize - 1;
    }

    pub fn left_node_index(position: usize) -> usize {
        let min_pow = node_level(position) as u32;
        let value = if min_pow == 0 {
            0
        } else {
            2_usize.pow(min_pow - 1)
        };
        position - value
    }

    fn update_hashes(&mut self, mut position: usize) {
        let mut level = node_level(position);
        loop {
            let parent_offset = 2_usize.pow(level as u32);
            if position < parent_offset {
                break;
            }
            let parent = position - parent_offset;
            let parent_level = node_level(parent);
            let left_index = parent - parent_level;
            let right_index = (parent + parent_level).min(position);
            self.hashes[parent].hash = hash_two(
                &self.hashes[left_index].hash,
                &self.hashes[right_index].hash,
            );
            // Update hash
            level += 1;
            position = parent;
        }
    }

    pub fn left_of(&self, position: usize) -> Option<&Child> {
        if position < self.hashes.len() {
            let value = &self.hashes[position].left;
            match value {
                Child::Node(_) | Child::Value(_) => Some(value),
                Child::None => None,
            }
        } else {
            None
        }
    }

    pub fn right_of(&self, position: usize) -> Option<&Child> {
        if position < self.hashes.len() {
            let value = &self.hashes[position].right;
            match value {
                Child::Node(_) | Child::Value(_) => Some(value),
                Child::None => None,
            }
        } else {
            None
        }
    }
}

fn hash_two<T: Hash>(value_1: &T, value_2: &T) -> u64 {
    let mut hasher = DefaultHasher::default();
    value_1.hash(&mut hasher);
    value_2.hash(&mut hasher);
    hasher.finish()
}

fn hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::default();
    value.hash(&mut hasher);
    hasher.finish()
}

fn node_level(value: usize) -> usize {
    let mut value = value + 1;
    let mut highest_pow_2 = (highest_power_of_2(value) as f32).log2() as u32;
    loop {
        let pow = 2_usize.pow(highest_pow_2);
        if pow <= value {
            value -= pow;
        }
        if value == 0 {
            break;
        }
        highest_pow_2 -= 1;
    }
    highest_pow_2 as usize
}

fn highest_power_of_2(number: usize) -> u32 {
    (1..=number)
        .rev()
        .find(|i| (i & (i - 1)) == 0)
        .map_or(0, |v| v) as u32
}

fn search_index<K: Ord, V>(data: &[Value<K, V>], key: &K) -> usize {
    let mut l = 0;
    let mut r = data.len() - 1;
    while l <= r {
        let m = l + (r - l) / 2;
        if data[m].key == *key {
            return m;
        }

        if data[m].key < *key {
            l = m + 1;
        } else {
            r = m - 1;
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use super::*;

    #[test]
    fn should_insert_complex_types() {
        let mut tree = MerkleTree::new();
        let value = TestValue {
            data1: "test".to_owned(),
            data2: 1,
            data3: vec![true, false],
        };
        tree.insert(0, value.clone());
        assert_eq!(tree.data[0].key, 0);
        assert_eq!(tree.data[0].value, value);
        assert_eq!(
            tree.hashes[0],
            Node {
                hash: hash(&0),
                left: Child::Value(0),
                right: Child::None
            }
        );
    }

    #[test]
    fn should_insert_nodes_and_calculate_hashes_correctly() {
        let mut tree = MerkleTree::new();
        tree.insert(0, "value 0");
        tree.insert(1, "value 1");
        assert_eq!(
            tree.data,
            vec![Value::with(0, "value 0"), Value::with(1, "value 1")]
        );
        assert_eq!(
            tree.hashes,
            vec![Node {
                hash: hash_two(&hash(&0), &hash(&1)),
                left: Child::Value(0),
                right: Child::Value(1)
            },]
        );

        tree.insert(2, "value 2");
        assert_eq!(
            tree.data,
            vec![
                Value::with(0, "value 0"),
                Value::with(1, "value 1"),
                Value::with(2, "value 2")
            ]
        );
        assert_eq!(
            tree.hashes,
            vec![
                Node {
                    hash: hash_two(&hash(&0), &hash(&1)),
                    left: Child::Value(0),
                    right: Child::Value(1)
                },
                Node {
                    hash: hash_two(&hash_two(&hash(&0), &hash(&1)), &hash(&2)),
                    left: Child::Node(0),
                    right: Child::Value(2)
                },
            ]
        );

        tree.insert(3, "value 3");
        assert_eq!(
            tree.data,
            vec![
                Value::with(0, "value 0"),
                Value::with(1, "value 1"),
                Value::with(2, "value 2"),
                Value::with(3, "value 3")
            ]
        );
        assert_eq!(
            tree.hashes,
            vec![
                Node {
                    hash: hash_two(&hash(&0), &hash(&1)),
                    left: Child::Value(0),
                    right: Child::Value(1)
                },
                Node {
                    hash: hash_two(
                        &hash_two(&hash(&0), &hash(&1)),
                        &hash_two(&hash(&2), &hash(&3))
                    ),
                    left: Child::Node(0),
                    right: Child::Node(2)
                },
                Node {
                    hash: hash_two(&hash(&2), &hash(&3)),
                    left: Child::Value(2),
                    right: Child::Value(3)
                },
            ]
        );

        tree.insert(4, "value 4");
        assert_eq!(
            tree.data,
            vec![
                Value::with(0, "value 0"),
                Value::with(1, "value 1"),
                Value::with(2, "value 2"),
                Value::with(3, "value 3"),
                Value::with(4, "value 4")
            ]
        );
        assert_eq!(
            tree.hashes,
            vec![
                Node {
                    hash: hash_two(&hash(&0), &hash(&1)),
                    left: Child::Value(0),
                    right: Child::Value(1)
                },
                Node {
                    hash: hash_two(
                        &hash_two(&hash(&0), &hash(&1)),
                        &hash_two(&hash(&2), &hash(&3))
                    ),
                    left: Child::Node(0),
                    right: Child::Node(2)
                },
                Node {
                    hash: hash_two(&hash(&2), &hash(&3)),
                    left: Child::Value(2),
                    right: Child::Value(3)
                },
                Node {
                    hash: hash_two(
                        &hash_two(
                            &hash_two(&hash(&0), &hash(&1)),
                            &hash_two(&hash(&2), &hash(&3))
                        ),
                        &hash(&4)
                    ),
                    left: Child::Node(1),
                    right: Child::Value(4),
                },
            ]
        );

        tree.insert(5, "value 5");
        assert_eq!(
            tree.data,
            vec![
                Value::with(0, "value 0"),
                Value::with(1, "value 1"),
                Value::with(2, "value 2"),
                Value::with(3, "value 3"),
                Value::with(4, "value 4"),
                Value::with(5, "value 5"),
            ]
        );
        assert_eq!(
            tree.hashes,
            vec![
                Node {
                    hash: hash_two(&hash(&0), &hash(&1)),
                    left: Child::Value(0),
                    right: Child::Value(1)
                },
                Node {
                    hash: hash_two(
                        &hash_two(&hash(&0), &hash(&1)),
                        &hash_two(&hash(&2), &hash(&3))
                    ),
                    left: Child::Node(0),
                    right: Child::Node(2)
                },
                Node {
                    hash: hash_two(&hash(&2), &hash(&3)),
                    left: Child::Value(2),
                    right: Child::Value(3)
                },
                Node {
                    hash: hash_two(
                        &hash_two(
                            &hash_two(&hash(&0), &hash(&1)),
                            &hash_two(&hash(&2), &hash(&3))
                        ),
                        &hash_two(&hash(&4), &hash(&5))
                    ),
                    left: Child::Node(1),
                    right: Child::Node(4),
                },
                Node {
                    hash: hash_two(&hash(&4), &hash(&5)),
                    left: Child::Value(4),
                    right: Child::Value(5),
                },
            ]
        );

        tree.insert(6, "value 6");
        assert_eq!(
            tree.data,
            vec![
                Value::with(0, "value 0"),
                Value::with(1, "value 1"),
                Value::with(2, "value 2"),
                Value::with(3, "value 3"),
                Value::with(4, "value 4"),
                Value::with(5, "value 5"),
                Value::with(6, "value 6"),
            ]
        );
        assert_eq!(
            tree.hashes,
            vec![
                Node {
                    hash: hash_two(&hash(&0), &hash(&1)),
                    left: Child::Value(0),
                    right: Child::Value(1)
                },
                Node {
                    hash: hash_two(
                        &hash_two(&hash(&0), &hash(&1)),
                        &hash_two(&hash(&2), &hash(&3))
                    ),
                    left: Child::Node(0),
                    right: Child::Node(2)
                },
                Node {
                    hash: hash_two(&hash(&2), &hash(&3)),
                    left: Child::Value(2),
                    right: Child::Value(3)
                },
                Node {
                    hash: hash_two(
                        &hash_two(
                            &hash_two(&hash(&0), &hash(&1)),
                            &hash_two(&hash(&2), &hash(&3))
                        ),
                        &hash_two(&hash_two(&hash(&4), &hash(&5)), &hash(&6))
                    ),
                    left: Child::Node(1),
                    right: Child::Node(5),
                },
                Node {
                    hash: hash_two(&hash(&4), &hash(&5)),
                    left: Child::Value(4),
                    right: Child::Value(5),
                },
                Node {
                    hash: hash_two(&hash_two(&hash(&4), &hash(&5)), &hash(&6)),
                    left: Child::Node(4),
                    right: Child::Value(6),
                },
            ]
        );

        tree.insert(7, "value 7");
        assert_eq!(
            tree.data,
            vec![
                Value::with(0, "value 0"),
                Value::with(1, "value 1"),
                Value::with(2, "value 2"),
                Value::with(3, "value 3"),
                Value::with(4, "value 4"),
                Value::with(5, "value 5"),
                Value::with(6, "value 6"),
                Value::with(7, "value 7"),
            ]
        );
        assert_eq!(
            tree.hashes,
            vec![
                Node {
                    hash: hash_two(&hash(&0), &hash(&1)),
                    left: Child::Value(0),
                    right: Child::Value(1)
                },
                Node {
                    hash: hash_two(
                        &hash_two(&hash(&0), &hash(&1)),
                        &hash_two(&hash(&2), &hash(&3))
                    ),
                    left: Child::Node(0),
                    right: Child::Node(2)
                },
                Node {
                    hash: hash_two(&hash(&2), &hash(&3)),
                    left: Child::Value(2),
                    right: Child::Value(3)
                },
                Node {
                    hash: hash_two(
                        &hash_two(
                            &hash_two(&hash(&0), &hash(&1)),
                            &hash_two(&hash(&2), &hash(&3))
                        ),
                        &hash_two(
                            &hash_two(&hash(&4), &hash(&5)),
                            &hash_two(&hash(&6), &hash(&7))
                        )
                    ),
                    left: Child::Node(1),
                    right: Child::Node(5),
                },
                Node {
                    hash: hash_two(&hash(&4), &hash(&5)),
                    left: Child::Value(4),
                    right: Child::Value(5),
                },
                Node {
                    hash: hash_two(
                        &hash_two(&hash(&4), &hash(&5)),
                        &hash_two(&hash(&6), &hash(&7))
                    ),
                    left: Child::Node(4),
                    right: Child::Node(6),
                },
                Node {
                    hash: hash_two(&hash(&6), &hash(&7)),
                    left: Child::Value(6),
                    right: Child::Value(7),
                },
            ]
        );
    }

    #[test]
    fn should_search_index() {
        let data = vec![
            Value {
                hash: hash(&0),
                key: 0,
                value: "test",
            },
            Value {
                hash: hash(&1),
                key: 1,
                value: "test",
            },
            Value {
                hash: hash(&3),
                key: 3,
                value: "test",
            },
            Value {
                hash: hash(&4),
                key: 4,
                value: "test",
            },
        ];
        assert_eq!(search_index(&data, &3), 2);
        assert_eq!(search_index(&data, &2), 2);
    }

    #[test]
    fn should_search_with_single_element() {
        let data = vec![Value {
            hash: hash(&0),
            key: 0,
            value: "test",
        }];
        assert_eq!(search_index(&data, &1), 1);
    }

    #[test]
    fn should_calculate_root_position() {
        let mut tree = MerkleTree::new();
        tree.insert(0, "value 0");
        assert_eq!(tree.root, 0);
        tree.insert(1, "value 1");
        tree.insert(2, "value 2");
        assert_eq!(tree.root, 1);
        tree.insert(3, "value 3");
        tree.insert(4, "value 4");
        assert_eq!(tree.root, 3);
        tree.insert(5, "value 5");
        tree.insert(6, "value 6");
        tree.insert(7, "value 7");
        tree.insert(8, "value 8");
        assert_eq!(tree.root, 7);
        tree.insert(9, "value 9");
        tree.insert(10, "value 10");
        tree.insert(11, "value 11");
        tree.insert(12, "value 12");
        tree.insert(13, "value 13");
        tree.insert(14, "value 14");
        tree.insert(15, "value 15");
        tree.insert(16, "value 16");
        assert_eq!(tree.root, 15);
    }

    #[test]
    fn should_calculate_left_right_nodes() {
        let mut tree = MerkleTree::new();
        tree.insert(0, "value 0");
        tree.insert(1, "value 1");
        tree.insert(2, "value 2");
        tree.insert(3, "value 3");
        tree.insert(4, "value 4");
        tree.insert(5, "value 5");
        tree.insert(6, "value 6");
        tree.insert(7, "value 7");
        tree.insert(8, "value 8");
        assert_eq!(tree.left_of(1), Some(&Child::Node(0)));
        assert_eq!(tree.right_of(1), Some(&Child::Node(2)));
        assert_eq!(tree.left_of(7), Some(&Child::Node(3)));
        assert_eq!(tree.right_of(7), Some(&Child::Value(8)));
    }

    #[test]
    fn should_calculate_root_hash() {
        for v in 0..16 {
            let mut tree_1 = MerkleTree::new();
            for i in 0..16 {
                tree_1.insert(i, v);
            }

            let mut tree_2 = MerkleTree::new();
            for i in 0..16 {
                if i != 11 {
                    tree_2.insert(i, i);
                }
            }
            println!("Root: {}", tree_1.root);
            print(&tree_1);
            println!("{}", tree_1);
            println!("Root: {}", tree_2.root);
            print(&tree_2);
            println!("{}", tree_1);
            println!("{}", tree_2);
            assert_ne!(
                tree_1.hashes[tree_1.root].hash,
                tree_2.hashes[tree_2.root].hash
            );
        }
    }

    #[test]
    fn should_calculate_hashes_for_entire_tree() {
        let mut tree_1 = MerkleTree::new();
        for i in 0..16 {
            if i != 4 {
                tree_1.insert(i, i);
            }
        }

        let mut tree_2 = MerkleTree::new();
        for i in 0..16 {
            tree_2.insert(i, i);
        }
        println!("{}", tree_1);
        println!("{}", tree_2);
        assert_eq!(tree_1.hashes.len(), 14);
        assert_eq!(tree_2.hashes.len(), 15);
        for i in 0..15 {
            if i == 14 {
                break;
            }
            let v1 = &tree_1.hashes[i];
            let v2 = &tree_2.hashes[i];
            if i < 3 {
                assert_eq!(v1.hash, v2.hash);
            } else {
                assert_ne!(v1.hash, v2.hash);
            }
        }
    }

    fn print<K, V>(tree: &MerkleTree<K, V>) {
        for (i, h) in tree.hashes.iter().enumerate() {
            println!("{} -> ({:?}, {:?})", i, h.left, h.right);
        }
    }

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct TestValue {
        data1: String,
        data2: u64,
        data3: Vec<bool>,
    }

    impl<K, V> Value<K, V>
    where
        K: Hash,
    {
        fn with(key: K, value: V) -> Self {
            Self {
                hash: hash(&key),
                key,
                value,
            }
        }
    }

    impl<K, V> Display for MerkleTree<K, V>
    where
        K: Debug,
        V: Debug,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut queue = vec![(0, self.root)];
            while let Some((indent, node_index)) = queue.pop() {
                let node = &self.hashes[node_index];
                write!(f, "\n{:indent$}Node: {}", "", node.hash)?;
                match node.right {
                    Child::Node(n) => queue.push((indent + 1, n)),
                    Child::Value(v) => {
                        let data = &self.data[v];
                        writeln!(
                            f,
                            " {:indent$}\nNode: hash {} {:?} -> {:?}",
                            "", data.hash, data.key, data.value
                        )?;
                    }
                    Child::None => writeln!(f, "{:indent$}{}", "", node.hash)?,
                }
                match node.left {
                    Child::Node(n) => queue.push((indent + 1, n)),
                    Child::Value(v) => {
                        let data = &self.data[v];
                        writeln!(
                            f,
                            " {:indent$}\nNode: hash {} {:?} -> {:?}",
                            "", data.hash, data.key, data.value
                        )?;
                    }
                    Child::None => writeln!(f, "{:indent$}{}", "", node.hash)?,
                }
            }
            writeln!(f)?;
            writeln!(f, "{:?}", self.data)?;

            Ok(())
        }
    }
}
