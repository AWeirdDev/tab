use alloc::{string::String, vec::Vec};
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

#[derive(Debug, PartialEq, Eq)]
pub enum NodeData {
    Text(String),
    Element {
        tag: String,
        attributes: AttributesList,
    },
    Comment(String),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct AttributesList(Vec<Attribute>);

impl AttributesList {
    /// Get an attribute.
    ///
    /// # Time complexity
    /// O(N), unoptimized
    pub fn get(&self, key: &str) -> Option<&Attribute> {
        self.0.iter().find(|attr| attr.name.eq(key))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attribute {
    name: String,
    key: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub(crate) previous: Option<NodeHandle>,
    pub(crate) next: Option<NodeHandle>,
    pub(crate) first_child: Option<NodeHandle>,
    pub(crate) last_child: Option<NodeHandle>,
    pub(crate) parent: Option<NodeHandle>,
    pub data: Option<NodeData>,
}

macro_rules! node_get_impl {
    ($fld:ident) => {
        #[doc = "Get the handle for `"]
        #[doc = stringify!($fld)]
        #[doc = "`.\n\nIf `None` is returned, then such item doesn't exist. "]
        #[doc = "However, there is no guarantee that its generation ID is correct, "]
        #[doc = "since it might've been deallocated from the arena already."]
        pub fn $fld(&self) -> Option<NodeHandle> {
            self.$fld
        }
    };
}

impl Node {
    node_get_impl!(previous);
    node_get_impl!(next);
    node_get_impl!(first_child);
    node_get_impl!(last_child);
    node_get_impl!(parent);

    /// Create a new node with the associated node data provided.
    #[inline]
    pub fn new(data: Option<NodeData>) -> Self {
        Self {
            data,
            ..Default::default()
        }
    }
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

/// A DOM arena.
pub struct DomArena {
    arena: Vec<Allocation<Node>>,
    generations: Vec<usize>,
    vacancies: Vec<usize>,
}

impl Default for DomArena {
    #[inline]
    fn default() -> Self {
        // rust-anaylzer keeps getting it wrong, i'll make it local
        use alloc::vec;

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

impl DomArena {
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

    fn deallocate(&mut self, handle: NodeHandle) -> Option<()> {
        self.arena.get_mut(handle.id())?.mark_unallocated();
        self.update_generation(handle.id());
        self.vacancies.push(handle.id());
        Some(())
    }
}

impl DomArena {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a node reference from its handle.
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

    /// Get a mutable node reference from its handle.
    ///
    /// Returns `None` if one of the following is satisfied:
    /// - It's marked as unallocated
    /// - It doesn't exist in the arena
    /// - Its generation ID mismatches the current one
    pub fn get_node_mut(&mut self, handle: NodeHandle) -> Option<&mut Node> {
        self.check_generation(handle)?;
        self.arena.get_mut(handle.id()).and_then(|item| {
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

    /// Remove a node, trimming off any relations to it before
    /// deallocating it from the arena.
    pub fn remove(&mut self, handle: NodeHandle) -> Option<()> {
        let node = self.get_node(handle)?;

        // up
        let maybe_parent_handle = node.parent;
        let maybe_prev_handle = node.previous;
        let maybe_next_handle = node.next;
        let maybe_first_child_handle = node.first_child();

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

        // down
        {
            let mut cursor = maybe_first_child_handle.and_then(|fc| self.get_node_mut(fc));
            while let Some(ref mut child) = cursor {
                child.parent = None;
                cursor = child.next.and_then(|next| self.get_node_mut(next));
            }
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
    /// To pass multiple nodes, simply provide a fragment-like child.
    ///
    /// It's worth noting that nothing other than the given node's
    /// `parent` and `previous` fields are changed in order to support
    /// fragments.
    ///
    /// # Parameters
    /// - `node`: The owned node to allocate and insert into the DOM.
    /// - `parent_handle`: The handle of the reference parent node.
    ///
    /// For more information on allocation, see [`DomArena::allocate`].
    pub fn append_child_in(
        &mut self,
        node_handle: NodeHandle,
        parent_handle: NodeHandle,
    ) -> Option<()> {
        let last_child = self.get_node(parent_handle)?.last_child;
        let node = self.get_node_mut(node_handle)?;
        node.previous = last_child;

        let mut tail_handle = node_handle;
        while let Some(tail) = self.get_node_mut(tail_handle) {
            tail.parent = Some(parent_handle);
            match tail.next {
                Some(nh) => tail_handle = nh,
                None => break,
            }
        }

        if let Some(prev) = last_child.and_then(|last_handle| self.get_node_mut(last_handle)) {
            prev.next = Some(node_handle);
        }

        let parent = self.get_node_mut(parent_handle)?;
        parent.first_child.get_or_insert(node_handle);
        parent.last_child = Some(tail_handle);

        Some(())
    }

    /// Insert a node after a reference node.
    pub fn insert_after(
        &mut self,
        node_handle: NodeHandle,
        reference_handle: NodeHandle,
    ) -> Option<()> {
        let reference = self.get_node(reference_handle)?;
        let reference_parent_handle = reference.parent;
        let reference_orig_next_handle = reference.next;

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

        Some(())
    }

    /// Insert a node before a reference node.
    pub fn insert_before(
        &mut self,
        node_handle: NodeHandle,
        reference_handle: NodeHandle,
    ) -> Option<()> {
        let reference = self.get_node(reference_handle)?;
        let reference_parent_handle = reference.parent;
        let reference_orig_prev_handle = reference.previous;

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

        Some(())
    }
}

#[cfg(test)]
mod tests {
    use crate::NodeHandle;

    use super::{DomArena, Node};

    /// Checks whether [`NodeId`] is actually non-zero
    /// compliant when inserting in a [`Dom`].
    #[test]
    fn nonzero_compliance() {
        let mut dom = DomArena::new();
        let id = dom.allocate(Node::default());
        assert_eq!(id, NodeHandle::new(1, 0));
    }

    #[test]
    fn basic_insert_remove() {
        let mut dom = DomArena::new();

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
        let mut dom = DomArena::new();

        let document = dom.allocate(Node::default());

        let inner = dom.allocate(Node::default());
        dom.append_child_in(inner, document).unwrap();

        let inner_other = dom.allocate(Node::default());
        dom.insert_after(inner_other, inner);

        let another = dom.allocate(Node::default());
        dom.insert_before(another, inner).unwrap();

        assert_eq!(
            dom.get_node(document).unwrap(),
            &Node {
                first_child: Some(another),
                last_child: Some(inner_other),
                ..Default::default()
            }
        );
        assert_eq!(
            dom.get_node(inner).unwrap(),
            &Node {
                parent: Some(document),
                previous: Some(another),
                next: Some(inner_other),
                ..Default::default()
            }
        );
    }
}
