use core::{cell::RefCell, marker::PhantomData};

use alloc::{rc::Rc, string::String};

pub use crate::core::DomArena;
use crate::{AttributesList, Node, NodeData, NodeHandle};

type DomInner = Rc<RefCell<DomArena>>;

/// A DOM container.
///
/// Note that there is no document root upon creation; you'll need to explicitly create one.
/// For more information, see [`Dom::create_document_fragment`].
///
/// # Example
/// Let's say we want to create the following HTML structure:
///
/// ```html
/// <div>Hello, World!</div>
/// ```
///
/// We'd need a `<div>` element, as well as a text node for the message.
///
/// ```
/// # use qdom::prelude::*;
/// let mut dom = Dom::new();
///
/// // Allocate nodes on the arena
/// let element = dom.create_element("div".to_string());
/// let text = dom.create_text_node("Hello, World!".to_string());
///
/// // Build hierarchy
/// element.append_child(text);
/// ```
pub struct Dom {
    inner: DomInner,
}

impl Dom {
    pub fn new() -> Self {
        let arena = DomArena::new();
        Self {
            inner: Rc::new(RefCell::new(arena)),
        }
    }

    pub(crate) fn allocate(&self, node: Node) -> NodeHandle {
        let mut arena = self.inner.borrow_mut();
        arena.allocate(node)
    }

    /// Create a fragment.
    ///
    /// This can also be used to create the root document.
    pub fn create_document_fragment(&mut self) -> GenericNode<node_type::DocumentFragment> {
        GenericNode::from_handle(self.inner.clone(), self.allocate(Node::new(None)))
    }

    /// Create an element.
    pub fn create_element(&mut self, tag: String) -> GenericNode<node_type::Element> {
        GenericNode::from_handle(
            self.inner.clone(),
            self.allocate(Node::new(Some(NodeData::Element {
                tag,
                attributes: AttributesList::default(),
            }))),
        )
    }

    /// Create a text node.
    pub fn create_text_node(&mut self, data: String) -> GenericNode<node_type::Text> {
        GenericNode::from_handle(
            self.inner.clone(),
            self.allocate(Node::new(Some(NodeData::Text(data)))),
        )
    }

    /// Create an HTML comment.
    pub fn create_comment(&mut self, data: String) -> GenericNode<node_type::Comment> {
        GenericNode::from_handle(
            self.inner.clone(),
            self.allocate(Node::new(Some(NodeData::Comment(data)))),
        )
    }
}

/// A generic node.
///
/// Note that the actual data is not held by this struct whatsoever;
/// you'd need to use dedicated functions to access them.
pub struct GenericNode<Marker> {
    phantom: PhantomData<fn() -> Marker>,
    arena: DomInner,
    handle: NodeHandle,
}

impl<M> GenericNode<M> {
    fn from_handle(arena: DomInner, handle: NodeHandle) -> Self {
        Self {
            phantom: PhantomData,
            arena,
            handle,
        }
    }

    fn apply<T>(&self, f: impl FnOnce(&mut Node) -> T) -> Option<T> {
        let mut arena = self.arena.borrow_mut();
        arena.get_node_mut(self.handle).map(|n| f(n))
    }

    /// Erase type information (the marker) of this node. Some might
    /// refer to this as "upcasting," and it cannot be downcast back
    /// (that is, narrow the type).
    pub fn into_unknown(self) -> GenericNode<node_type::Unknown> {
        GenericNode {
            phantom: PhantomData,
            arena: self.arena,
            handle: self.handle,
        }
    }
}

pub trait _Appendable {}
pub trait _DataOnly {}

impl<M: _Appendable> GenericNode<M> {
    pub fn append_child<AnyMarker>(&self, node: GenericNode<AnyMarker>) -> Option<()> {
        let mut arena = self.arena.borrow_mut();
        arena.append_child_in(node.handle, self.handle)
    }

    /// Append multiple children at once by iterating through `nodes`, an iterator
    /// of nodes with type information erased, namely [`node_type::Unknown`].
    ///
    /// To comply with Rust's type system, you'll need to erase type information
    /// (markers) that come with the nodes.
    ///
    /// # Example
    /// ```
    /// # use qdom::prelude::*;
    /// #
    /// let mut dom = Dom::new();
    ///
    /// let mut divs = vec![];
    /// for _ in 0..10 {
    ///     divs.push(
    ///         dom.create_element("div".to_string()).into_unknown()
    ///     );
    /// }
    ///
    /// let parent = dom.create_element("div".to_string());
    /// parent.append_children(divs.into_iter());
    /// ```
    pub fn append_children<N: Iterator<Item = GenericNode<node_type::Unknown>>>(
        &self,
        mut nodes: N,
    ) -> Option<()> {
        let mut arena = self.arena.borrow_mut();

        let head = nodes.next()?;

        let mut prev = head.handle;
        while let Some(node_ref) = nodes.next() {
            arena.insert_after(node_ref.handle, prev);
            prev = node_ref.handle;
        }

        arena.append_child_in(head.handle, self.handle)
    }
}

impl<M: _DataOnly> GenericNode<M> {
    /// Set the text content.
    pub fn set_text_content(&self, new_text: String) -> Option<()> {
        self.apply(|node| {
            node.data.replace(NodeData::Text(new_text));
        })
    }
}

pub mod node_type {
    use super::*;

    macro_rules! make_marker {
        ($ident:ident, $name:literal, trait $marker:ident) => {
            #[doc = "Represents "]
            #[doc = $name]
            #[doc = "\n\nThis is a marker trait paired with [`GenericNode`]."]
            pub struct $ident;

            impl $marker for $ident {}
        };
    }

    make_marker!(DocumentFragment, "a document fragment", trait _Appendable);
    make_marker!(Element, "an HTML element", trait _Appendable);
    make_marker!(Text, "a text node, which has data only", trait _DataOnly);
    make_marker!(Comment, "an HTML comment, which has data only", trait _DataOnly);
    make_marker!(
        Unknown,
        "an unknown item. The type is erased to match with Rust's type system.",
        trait _DataOnly
    );
}
