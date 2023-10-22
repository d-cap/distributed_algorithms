use merkle_tree::MerkleTree;

fn main() {
    let mut tree = MerkleTree::new();
    tree.insert(0, "test");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should() {
        main();
    }
}
