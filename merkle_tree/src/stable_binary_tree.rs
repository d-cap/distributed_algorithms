/// An arena allocated binary tree
pub struct StableBinaryTree<T> {
    data: Vec<Option<T>>,
}

impl<T> StableBinaryTree<T>
where
    T: Clone,
{
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: vec![None; capacity],
        }
    }
}
