#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::{mem, num::NonZeroUsize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle {
    id: NonZeroUsize,
    generation: usize,
}

// optimization check (no tag)
const _: () = assert!(size_of::<Option<NodeHandle>>() <= size_of::<usize>() * 2);

impl NodeHandle {
    #[inline]
    fn new(value: usize, generation: usize) -> Self {
        NodeHandle {
            id: NonZeroUsize::new(value).expect("expected non-zero for NodeId"),
            generation,
        }
    }

    /// Get the raw ID of the handle.
    #[inline]
    pub fn id(&self) -> usize {
        self.id.get()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub previous: Option<NodeHandle>,
    pub next: Option<NodeHandle>,
    pub first_child: Option<NodeHandle>,
    pub last_child: Option<NodeHandle>,
    pub parent: Option<NodeHandle>,
}

enum Allocation<T> {
    Unallocated,
    Allocated(T),
}

impl<T> Allocation<T> {
    fn patch_alloc(&mut self, value: T) -> Allocation<T> {
        // source: [`Option::replace`]
        mem::replace(self, Allocation::Allocated(value))
    }

    fn mark_unallocated(&mut self) -> Allocation<T> {
        // source: [`Option::take`]
        mem::replace(self, Allocation::Unallocated)
    }
}

pub struct Dom {
    arena: Vec<Allocation<Node>>,
    generations: Vec<usize>,
    vacancies: Vec<usize>,
}

impl Default for Dom {
    #[inline]
    fn default() -> Self {
        Self {
            arena: vec![
                // this is intentional, soley to comply with the NonZero guarantee
                Allocation::Unallocated,
            ],
            generations: vec![
                // same as the above
                0,
            ],
            vacancies: vec![],
        }
    }
}

impl Dom {
    fn update_generation(&mut self, id: usize) -> usize {
        let handle = self
            .generations
            .get_mut(id)
            .expect("expected index to exist");

        *handle = handle.wrapping_add(1);
        *handle
    }

    fn check_generation(&self, handle: NodeHandle) -> Option<()> {
        if handle.generation == *self.generations.get(handle.id())? {
            Some(())
        } else {
            None
        }
    }

    fn get_node_mut(&mut self, handle: NodeHandle) -> Option<&mut Node> {
        self.check_generation(handle)?;
        self.arena.get_mut(handle.id()).and_then(|item| {
            if let Allocation::Allocated(node_ref) = item {
                Some(node_ref)
            } else {
                None
            }
        })
    }

    fn deallocate(&mut self, handle: NodeHandle) -> Option<()> {
        self.arena.get_mut(handle.id())?.mark_unallocated();
        self.update_generation(handle.id());
        self.vacancies.push(handle.id());
        Some(())
    }
}

impl Dom {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a node from its handle.
    ///
    /// Returns `None` if one of the following is satisfied:
    /// - It's marked as unallocated
    /// - It doesn't exist in the arena
    /// - Its generation ID mismatches the current one
    pub fn get_node(&self, handle: NodeHandle) -> Option<&Node> {
        self.check_generation(handle)?;
        self.arena.get(handle.id()).and_then(|item| {
            if let Allocation::Allocated(node_ref) = item {
                Some(node_ref)
            } else {
                None
            }
        })
    }

    /// Allocate a node, regardless of whether it's detached.
    /// This function only guarantees that the node provided
    /// will be allocated on the arena.
    ///
    /// To save space and avoid reallocation, removed nodes
    /// will be marked as "unallocated," and that spot may be
    /// reused in future append operations.
    ///
    /// This function is the building block of all node
    /// insertion methods.
    pub fn allocate(&mut self, node: Node) -> NodeHandle {
        match self.vacancies.pop() {
            Some(vacancy_id) => {
                let handle = self
                    .arena
                    .get_mut(vacancy_id)
                    .expect("last vacancy did not give a valid index");

                // since we replaced it, the previous one is dead
                handle.patch_alloc(node);
                let generation = self.update_generation(vacancy_id);

                NodeHandle::new(vacancy_id, generation)
            }

            None => {
                // these two MUST be tied together
                self.arena.push(Allocation::Allocated(node));
                self.generations.push(0);

                NodeHandle::new(
                    self.arena.len() - 1, // index
                    0,                    // 0th generation
                )
            }
        }
    }

    pub fn remove(&mut self, handle: NodeHandle) -> Option<()> {
        let node = self.get_node(handle)?;

        // up/down
        let maybe_parent_handle = node.parent;
        let maybe_prev_handle = node.previous;
        let maybe_next_handle = node.next;

        if let Some(parent_handle) = maybe_parent_handle
            && let Some(parent) = self.get_node_mut(parent_handle)
            && parent
                .first_child
                .is_some_and(|child| child.id() == handle.id())
        {
            parent.first_child = maybe_next_handle;
        }
        if let Some(parent_handle) = maybe_parent_handle
            && let Some(parent) = self.get_node_mut(parent_handle)
            && parent
                .last_child
                .is_some_and(|child| child.id() == handle.id())
        {
            parent.last_child = maybe_prev_handle;
        }

        // left/right
        if let Some(prev) = maybe_prev_handle.and_then(|prev| self.get_node_mut(prev)) {
            let _ = mem::replace(&mut prev.next, maybe_next_handle);
        }
        if let Some(next) = maybe_next_handle.and_then(|next| self.get_node_mut(next)) {
            next.previous = maybe_prev_handle;
        }

        // deallocate
        self.deallocate(handle);

        Some(())
    }

    pub fn exists(&self, node: NodeHandle) -> bool {
        self.check_generation(node).is_some()
    }

    /// Append a child in a node.
    ///
    /// It's worth noting that nothing other than the given node's
    /// `parent` and `previous` fields are changed in order to support
    /// fragments.
    ///
    /// # Parameters
    /// - `node`: The owned node to allocate and insert into the DOM.
    /// - `parent_handle`: The handle of the reference parent node.
    ///
    /// For more information on allocation, see [`Dom::allocate`].
    pub fn append_child_in(
        &mut self,
        mut node: Node,
        parent_handle: NodeHandle,
    ) -> Option<NodeHandle> {
        let last_child = self.get_node(parent_handle)?.last_child;
        node.previous = last_child;
        node.parent = Some(parent_handle);

        {
            let mut cursor = Some(&mut node);
            while let Some(next) = cursor {
                next.parent = Some(parent_handle);
                cursor = next.next.and_then(|next_node| self.get_node_mut(next_node));
            }
        }

        let node_handle = self.allocate(node);

        if let Some(prev) = last_child.and_then(|last_handle| self.get_node_mut(last_handle)) {
            prev.next = Some(node_handle);
        }

        let parent = self.get_node_mut(parent_handle)?;
        parent.first_child.get_or_insert(node_handle);
        parent.last_child = Some(node_handle);

        Some(node_handle)
    }

    /// Insert a node after a reference node.
    pub fn insert_after(&mut self, node: Node, reference_handle: NodeHandle) -> Option<NodeHandle> {
        let reference = self.get_node(reference_handle)?;
        let reference_parent_handle = reference.parent;
        let reference_orig_next_handle = reference.next;

        let node_handle = self.allocate(node);

        // left
        let reference = self.get_node_mut(reference_handle)?;
        reference.next = Some(node_handle);

        // middle
        let node = self.get_node_mut(node_handle)?;
        node.previous = Some(reference_handle);
        node.parent = reference_parent_handle;
        node.next = reference_orig_next_handle;

        // right
        if let Some(reference_orig_next) =
            reference_orig_next_handle.and_then(|on| self.get_node_mut(on))
        {
            reference_orig_next.previous = Some(node_handle);
        } else if let Some(parent) = reference_parent_handle.and_then(|h| self.get_node_mut(h)) {
            // reference was the last child
            // the new node is now the last child
            parent.last_child = Some(node_handle);
        }

        Some(node_handle)
    }

    /// Insert a node before a reference node.
    pub fn insert_before(
        &mut self,
        node: Node,
        reference_handle: NodeHandle,
    ) -> Option<NodeHandle> {
        let reference = self.get_node(reference_handle)?;
        let reference_parent_handle = reference.parent;
        let reference_orig_prev_handle = reference.previous;

        let node_handle = self.allocate(node);

        // right
        let reference = self.get_node_mut(reference_handle)?;
        reference.previous = Some(node_handle);

        // middle
        let node = self.get_node_mut(node_handle)?;
        node.next = Some(reference_handle);
        node.parent = reference_parent_handle;
        node.previous = reference_orig_prev_handle;

        // left
        if let Some(reference_orig_prev) =
            reference_orig_prev_handle.and_then(|on| self.get_node_mut(on))
        {
            reference_orig_prev.next = Some(node_handle);
        } else if let Some(parent) = reference_parent_handle.and_then(|h| self.get_node_mut(h)) {
            // reference was the first child
            // the new node is now the first child
            parent.first_child = Some(node_handle);
        }

        Some(node_handle)
    }
}

#[cfg(test)]
mod tests {
    use crate::NodeHandle;

    use super::{Dom, Node};

    /// Checks whether [`NodeId`] is actually non-zero
    /// compliant when inserting in a [`Dom`].
    #[test]
    fn nonzero_compliance() {
        let mut dom = Dom::new();
        let id = dom.allocate(Node::default());
        assert_eq!(id, NodeHandle::new(1, 0));
    }

    #[test]
    fn basic_insert_remove() {
        let mut dom = Dom::new();

        let child = dom.allocate(Node::default());
        let prev = dom.allocate(Node::default());
        let next = dom.allocate(Node::default());

        let first = Node {
            first_child: Some(child),
            previous: Some(prev),
            next: Some(next),
            ..Default::default()
        };
        let first = dom.allocate(first);

        dom.remove(first);

        assert!(!dom.exists(first));
        assert_eq!(dom.get_node(prev).unwrap().next.unwrap(), next);
    }

    #[test]
    fn basic_operations() {
        let mut dom = Dom::new();

        let document = dom.allocate(Node::default());
        let inner = dom.append_child_in(Node::default(), document).unwrap();
        let another = dom.insert_after(Node::default(), inner).unwrap();

        assert_eq!(
            dom.get_node(document).unwrap(),
            &Node {
                first_child: Some(inner),
                last_child: Some(another),
                ..Default::default()
            }
        );
        assert_eq!(
            dom.get_node(another).unwrap(),
            &Node {
                parent: Some(document),
                previous: Some(inner),
                ..Default::default()
            }
        );
    }
}
