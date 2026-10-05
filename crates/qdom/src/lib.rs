#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::num::NonZeroUsize;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(NonZeroUsize);

impl From<usize> for NodeId {
    #[inline]
    fn from(value: usize) -> Self {
        NodeId(NonZeroUsize::new(value).expect("expected non-zero for NodeId"))
    }
}

impl From<NodeId> for usize {
    #[inline]
    fn from(value: NodeId) -> Self {
        value.0.get()
    }
}

impl NodeId {
    #[inline]
    fn new(x: usize) -> Self {
        x.into()
    }
}

// pub struct NodeRef();

#[derive(Default)]
pub struct Node {
    pub previous: Option<NodeId>,
    pub next: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub parent: Option<NodeId>,
}

pub struct Dom {
    arena: Vec<Option<Node>>,
    vacancies: Vec<NodeId>,
}

impl Default for Dom {
    #[inline]
    fn default() -> Self {
        Self {
            arena: vec![
                // this is intentional, soley to comply with the NonZero guarantee
                None,
            ],
            vacancies: vec![],
        }
    }
}

impl Dom {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, node: Node) -> NodeId {
        match self.vacancies.pop() {
            Some(vacancy) => {
                let index: usize = vacancy.into();
                let handle = self
                    .arena
                    .get_mut(index)
                    .expect("last vacancy did not give a valid index");

                handle.replace(node);
                vacancy
            }

            None => {
                self.arena.push(Some(node));
                NodeId::new(self.arena.len() - 1)
            }
        }
    }

    // pub fn remove(&mut self, node: Node) {}
}

#[cfg(test)]
mod tests {
    use crate::NodeId;

    use super::{Dom, Node};

    /// Checks whether [`NodeId`] is actually non-zero
    /// compliant when inserting in a [`Dom`].
    #[test]
    fn nonzero_compliance() {
        let mut dom = Dom::new();
        let id = dom.insert(Node::default());
        assert_eq!(id, NodeId::new(1));
    }
}
