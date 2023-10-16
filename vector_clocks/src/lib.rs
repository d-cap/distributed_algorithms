use std::cmp::Ordering;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VectorClock {
    nodes: Vec<u8>,
}

impl PartialOrd for VectorClock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        assert_eq!(self.nodes.len(), other.nodes.len());
        let mut ordering = Ordering::Equal;
        for i in 0..self.nodes.len() {
            let local_ordering = self.nodes[i].cmp(&other.nodes[i]);
            match ordering {
                Ordering::Less => match local_ordering {
                    Ordering::Less | Ordering::Equal => ordering = Ordering::Less,
                    Ordering::Greater => return None,
                },
                Ordering::Equal => ordering = local_ordering,
                Ordering::Greater => match local_ordering {
                    Ordering::Less => return None,
                    Ordering::Equal | Ordering::Greater => ordering = Ordering::Greater,
                },
            }
        }
        Some(ordering)
    }
}

impl VectorClock {
    pub fn new(node_amount: usize) -> Self {
        Self {
            nodes: vec![0; node_amount],
        }
    }

    pub fn increment(&mut self, node: usize) {
        assert!(self.nodes.len() > node);
        self.nodes[node] += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        assert_eq!(self.nodes.len(), other.nodes.len());
        for i in 0..self.nodes.len() {
            self.nodes[i] = self.nodes[i].max(other.nodes[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use more_asserts::*;

    use super::*;

    #[test]
    fn should_merge_vector_clocks() {
        let mut v1 = VectorClock::new(5);
        let mut v2 = VectorClock::new(5);

        v1.increment(1);
        v1.increment(1);
        v1.increment(2);

        v2.increment(1);
        v2.increment(2);
        v2.increment(3);

        v1.merge(&v2);
        assert_eq!(v1.nodes, vec![0, 2, 1, 1, 0]);
    }

    #[test]
    fn should_compare_vector_clocks() {
        let mut v1 = VectorClock::new(5);
        let mut v2 = VectorClock::new(5);

        v1.increment(1);
        v1.increment(2);
        v1.increment(3);

        v2.increment(1);
        v2.increment(1);
        v2.increment(2);
        v2.increment(3);

        assert_lt!(v1, v2);
        assert_gt!(v2, v1);

        let mut v1 = VectorClock::new(5);
        let mut v2 = VectorClock::new(5);

        v1.increment(1);
        v1.increment(2);
        v1.increment(3);

        v2.increment(1);
        v2.increment(2);
        v2.increment(3);
        assert_eq!(v1, v2);
    }

    #[test]
    fn should_not_compare_concurrent_vector_clocks() {
        let mut v1 = VectorClock::new(5);
        let mut v2 = VectorClock::new(5);

        v1.increment(1);
        v1.increment(2);
        v1.increment(3);
        v1.increment(4);

        v2.increment(1);
        v2.increment(1);
        v2.increment(2);
        v2.increment(3);

        assert_eq!(v1.partial_cmp(&v2), None);
    }

    #[test]
    fn should_sort_casuality() {
        let mut v1 = VectorClock::new(5);
        v1.increment(0);

        let mut v2 = VectorClock::new(5);
        v2.increment(0);
        v2.increment(1);

        let mut v3 = VectorClock::new(5);
        v3.increment(0);
        v3.increment(3);

        let mut vec = vec![v3.clone(), v2.clone(), v1.clone()];
        vec.sort_unstable_by(|a, b| {
            if let Some(o) = a.partial_cmp(b) {
                o
            } else {
                Ordering::Equal
            }
        });

        assert_eq!(vec, vec![v1, v3, v2]);
    }
}
