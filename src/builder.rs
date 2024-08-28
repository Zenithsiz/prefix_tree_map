use {
    crate::{
        key_part::KeyPart,
        prefix_tree_map::{Node, PrefixTreeMap},
        std_lib::{BinaryHeap, Ordering},
    },
    core::{cell::RefCell, mem},
    std::rc::Rc,
};

/// The prefix tree map builder
#[derive(Clone)]
pub struct PrefixTreeMapBuilder<E, W, V> {
    root: Rc<NodeBuilder<E, W, V>>,
    max_wildcard_depth: usize,
}

#[derive(Clone)]
struct NodeBuilder<E, W, V> {
    key_part: Option<KeyPart<E, W>>,
    value: RefCell<Option<V>>,
    children: RefCell<Option<Children<E, W, V>>>,
}

type Children<E, W, V> = BinaryHeap<Rc<NodeBuilder<E, W, V>>>;

impl<E, W, V> PrefixTreeMapBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
    /// Create a new `PrefixTreeMapBuilder`
    pub fn new() -> Self {
        Self {
            root: Rc::new(NodeBuilder {
                key_part: None,
                value: RefCell::new(None),
                children: RefCell::new(None),
            }),
            max_wildcard_depth: 0,
        }
    }

    /// Insert a new value into the prefix tree map
    ///
    /// Key parts need to be marked by [`KeyPart`](enum.KeyPart.html)
    ///
    /// Insert into a existed key path could overwrite the value in it
    pub fn insert(&mut self, key: impl IntoIterator<Item = KeyPart<E, W>>, value: V) {
        let mut node = Rc::clone(&self.root);
        let mut wildcard_depth = 0;

        for key_part in key {
            if key_part.is_wildcard() {
                wildcard_depth += 1;
            }

            if node.children.borrow().is_none() {
                let mut children = BinaryHeap::new();
                children.push(Rc::new(NodeBuilder::new(key_part)));

                *node.children.borrow_mut() = Some(children);

                let child = Rc::clone(node.children.borrow().as_ref().unwrap().peek().unwrap());

                node = child;
            } else {
                let mut children_ref = node.children.borrow_mut();
                let children = children_ref.as_mut().unwrap();

                if let Some(child) = children
                    .iter()
                    .find(|child| child.key_part.as_ref() == Some(&key_part))
                    .map(Rc::clone)
                {
                    mem::drop(children_ref);
                    node = child;
                } else {
                    let key_part_cloned = key_part.clone();
                    children.push(Rc::new(NodeBuilder::new(key_part_cloned)));

                    let child = children
                        .iter()
                        .find(|child| child.key_part.as_ref() == Some(&key_part))
                        .map(Rc::clone)
                        .unwrap();

                    mem::drop(children_ref);
                    node = child;
                }
            }
        }

        *node.value.borrow_mut() = Some(value);

        self.max_wildcard_depth = self.max_wildcard_depth.max(wildcard_depth);
    }

    /// Insert a new value in an exact key path
    pub fn insert_exact(&mut self, key: impl IntoIterator<Item = E>, value: V) {
        self.insert(key.into_iter().map(KeyPart::Exact), value);
    }

    /// Build the prefix tree map
    pub fn build(self) -> PrefixTreeMap<E, W, V> {
        PrefixTreeMap {
            root: Self::node_builder_to_node(self.root),
            max_wildcard_depth: self.max_wildcard_depth,
        }
    }

    fn node_builder_to_node(node_builder: Rc<NodeBuilder<E, W, V>>) -> Node<E, W, V> {
        let node_builder = Rc::try_unwrap(node_builder).map_err(|_| ()).unwrap();
        let key_part = node_builder.key_part;
        let value = node_builder.value.into_inner();

        let children = node_builder.children.into_inner().map(|children| {
            children
                .into_sorted_vec()
                .into_iter()
                .map(Self::node_builder_to_node)
                .collect()
        });

        Node {
            key_part,
            value,
            children,
        }
    }
}

impl<E, W, V> Default for PrefixTreeMapBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E, W, V> NodeBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
    fn new(key_part: KeyPart<E, W>) -> Self {
        Self {
            key_part: Some(key_part),
            value: RefCell::new(None),
            children: RefCell::new(None),
        }
    }
}

impl<E, W, V> PartialEq for NodeBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
    fn eq(&self, other: &Self) -> bool {
        self.key_part == other.key_part
    }
}

impl<E, W, V> Eq for NodeBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
}

impl<E, W, V> PartialOrd for NodeBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key_part.partial_cmp(&other.key_part)
    }
}

impl<E, W, V> Ord for NodeBuilder<E, W, V>
where
    E: Clone + Ord,
    W: Clone + Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.key_part.cmp(&other.key_part)
    }
}
