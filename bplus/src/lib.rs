use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub type RefNode = Rc<RefCell<Node>>;
#[derive(Debug)]
struct Pair {
    key: usize,
    value: RefNode,
}

impl Pair {
    fn new(key: usize, node: Node) -> Self {
        Pair {
            key,
            value: Rc::new(RefCell::new(node)),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Record {
    key: usize,
    value: usize,
}
#[derive(Debug)]
pub enum Node {
    Root(RootNode),
    Internal(InternalNode),
    Leaf(LeafNode),
}
impl Node {
    pub fn new(cap: usize) -> Self {
        Node::Root(RootNode { cap, data: None })
    }

    #[must_use = "insertion may fail"]
    pub fn insert(&mut self, data: Record) -> Option<RefNode> {
        match self {
            Node::Root(root) => root.insert(data),
            Node::Internal(internal) => internal.insert(data),
            Node::Leaf(leaf) => leaf.insert(data),
        }
    }

    fn min_key(&self) -> Option<usize> {
        match self {
            Node::Root(root) => root.data.as_ref().and_then(|n| n.min_key()),
            Node::Internal(internal) => internal.data.first().map(|p| p.key),
            Node::Leaf(leaf) => leaf.data.first().map(|r| r.key),
        }
    }
}

#[derive(Debug)]
pub struct RootNode {
    cap: usize,
    data: Option<Box<Node>>,
}
impl RootNode {
    fn insert(&mut self, data: Record) -> Option<RefNode> {
        // 末尾に常に入れるわけではない
        if self.data.is_none() {
            self.data = Some(Box::new(Node::Leaf(LeafNode::new(self.cap, data))));
            return None;
        }
        let n = self.data.as_deref_mut().unwrap();
        let inserted = n.insert(data);
        if let Some(inserted) = inserted {
            // 子はfullになって分割したので、自身の保持しているnodeには追加済みなので、
            let n = self.data.take().unwrap();
            let new_child_data = vec![
                Pair::new(
                    n.min_key().unwrap(),
                    *n, // unwrap from box
                ),
                Pair {
                    key: inserted.borrow().min_key().unwrap(),
                    value: inserted.clone(), // unwrap from box
                },
            ];
            let new_child = InternalNode {
                cap: self.cap,
                data: new_child_data,
            };
            self.data = Some(Box::new(Node::Internal(new_child)));
        }
        None
    }
}
#[derive(Debug)]
pub struct InternalNode {
    cap: usize,
    data: Vec<Pair>,
}
impl InternalNode {
    fn insert(&mut self, data: Record) -> Option<RefNode> {
        if self.data.is_empty() {
            self.data.push(Pair::new(
                data.key,
                Node::Leaf(LeafNode {
                    cap: self.cap,
                    data: Vec::new(),
                    next: None,
                }),
            ))
        }
        let splited = self.find_node(&data).value.borrow_mut().insert(data);
        if let Some(n) = splited {
            if let Some(k) = n.borrow().min_key() {
                self.data.push(Pair {
                    key: k,
                    value: n.clone(),
                });
                self.data.sort_by_key(|p| p.key);
            }
        }
        if self.is_full() {
            return Some(self.split());
        }
        None
    }

    fn split(&mut self) -> RefNode {
        let right = self.data.split_off(self.data.len() / 2);
        let new_next = Self {
            cap: self.cap,
            data: right,
        };
        Rc::new(RefCell::new(Node::Internal(new_next)))
    }

    fn find_node<'a>(&'a mut self, data: &Record) -> &'a Pair {
        if self.data.is_empty() {
            self.data.push(Pair::new(
                data.key,
                Node::Leaf(LeafNode {
                    cap: self.cap,
                    data: Vec::new(),
                    next: None,
                }),
            ))
        }
        let x = self
            .data
            .iter()
            .take_while(|pair| pair.key < data.key)
            .last();
        if let Some(x) = x {
            x
        } else {
            self.data.first().unwrap()
        }
    }

    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        // 暗黙的に最小値のキー分保持している
        // その分がcapを圧迫しちゃうので、そのサイズ分無視するために１加算
        self.data.len() > (self.cap + 1)
    }
}
#[derive(Debug)]
pub struct LeafNode {
    cap: usize,
    data: Vec<Record>,
    next: Option<Weak<RefCell<Node>>>,
}

impl LeafNode {
    fn new(cap: usize, data: Record) -> Self {
        LeafNode {
            cap,
            data: vec![data],
            next: None,
        }
    }
    fn insert(&mut self, data: Record) -> Option<RefNode> {
        // 末尾に常に入れるわけではない
        self.data.push(data);
        self.data.sort_by_key(|r| r.key);
        if self.is_full() {
            return Some(self.split());
        }
        None
    }

    fn split(&mut self) -> RefNode {
        let right = self.data.split_off(self.data.len() / 2);
        let next = self.next.take();
        let new_next = Self {
            cap: self.cap,
            data: right,
            next,
        };
        let ref_node = Rc::new(RefCell::new(Node::Leaf(new_next)));
        self.next = Some(Rc::downgrade(&ref_node));
        ref_node
    }
    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        self.data.len() > self.cap
    }
}

#[cfg(test)]
mod test {
    use std::fmt;

    use super::*;

    impl fmt::Pointer for Node {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // use `as` to convert to a `*const T`, which implements Pointer, which we can use

            let ptr = self as *const Self;
            fmt::Pointer::fmt(&ptr, f)
        }
    }

    fn extract_root(node: &Node) -> Option<&RootNode> {
        match node {
            Node::Root(root) => Some(root),
            _ => None,
        }
    }
    fn extract_internal(node: &Node) -> Option<&InternalNode> {
        match node {
            Node::Internal(internal) => Some(internal),
            _ => None,
        }
    }
    fn extract_leaf(node: &Node) -> Option<&LeafNode> {
        match node {
            Node::Leaf(leaf) => Some(leaf),
            _ => None,
        }
    }

    #[test]
    fn insert() {
        {
            let mut n = Node::new(3);
            {
                let root = extract_root(&n).unwrap();
                assert!(root.data.is_none());
            }
            let _ = n.insert(Record { key: 9, value: 11 });
            {
                let root = extract_root(&n).unwrap();
                assert!(root.data.is_some());
                let leaf = extract_leaf(root.data.as_deref().unwrap()).unwrap();
                assert_eq!(leaf.data.len(), 1);
                assert_eq!(leaf.data.first().unwrap(), &Record { key: 9, value: 11 });
            }
            let _ = n.insert(Record { key: 8, value: 11 });
            let _ = n.insert(Record { key: 7, value: 11 });
            {
                let root = extract_root(&n).unwrap();
                assert!(root.data.is_some());
                let leaf = extract_leaf(root.data.as_deref().unwrap()).unwrap();
                assert_eq!(leaf.data.len(), 3);
            }
            let _ = n.insert(Record { key: 10, value: 11 });
            {
                let root = extract_root(&n).unwrap();
                let leaf = extract_internal(root.data.as_deref().unwrap()).unwrap();
                assert_eq!(leaf.data.len(), 2);
            }
            let _ = n.insert(Record { key: 11, value: 11 });

            let root = extract_root(&n).unwrap();
            let internal = extract_internal(root.data.as_deref().unwrap()).unwrap();
            assert_eq!(internal.data.len(), 2);
            {
                let x = internal.data.get(0).unwrap();
                let x = x.value.as_ref().borrow();
                let leaf = extract_leaf(&x).unwrap();
                assert_eq!(
                    leaf.data,
                    vec![Record { key: 7, value: 11 }, Record { key: 8, value: 11 },]
                );
                let next = leaf.next.as_ref();
                assert!(next.is_some());
                let x = next.unwrap().upgrade().unwrap();
                let x = x.borrow();

                let leaf = extract_leaf(&x).unwrap();
                // 分割後に追加された値が入っている
                assert_eq!(
                    leaf.data,
                    vec![
                        Record { key: 9, value: 11 },
                        Record { key: 10, value: 11 },
                        Record { key: 11, value: 11 }
                    ]
                );
            }
            {
                let x = internal.data.get(1).unwrap();
                assert_eq!(x.key, 9);
                let x = x.value.as_ref().borrow();
                let leaf = extract_leaf(&x).unwrap();
                assert_eq!(
                    leaf.data,
                    vec![
                        Record { key: 9, value: 11 },
                        Record { key: 10, value: 11 },
                        Record { key: 11, value: 11 }
                    ]
                );
            }
        }
        {
            let mut n = Node::new(2);
            let _ = n.insert(Record { key: 9, value: 11 });
            let _ = n.insert(Record { key: 8, value: 11 });
            let _ = n.insert(Record { key: 7, value: 11 });
            let _ = n.insert(Record { key: 10, value: 11 });
            let _ = n.insert(Record { key: 11, value: 11 });

            let root = extract_root(&n).unwrap();
            let internal = extract_internal(root.data.as_deref().unwrap()).unwrap();
            assert_eq!(internal.data.len(), 2);
            {
                let p = internal.data.get(0).unwrap();
                let x = p.value.as_ref().borrow();
                let internal = extract_internal(&x).unwrap();
                {
                    let p = internal.data.get(0).unwrap();
                    let x = p.value.as_ref().borrow();
                    let leaf = extract_leaf(&x).unwrap();
                    assert_eq!(leaf.data, vec![Record { key: 7, value: 11 }]);

                    let next = leaf.next.as_ref();
                    let x = next.unwrap().upgrade().unwrap();
                    let x = x.borrow();
                    let next_leaf = extract_leaf(&x).unwrap();
                    // 分割後に追加された値が入っている
                    assert_eq!(next_leaf.data, vec![Record { key: 8, value: 11 }]);
                }
                {
                    let p = internal.data.get(1).unwrap();
                    let x = p.value.as_ref().borrow();
                    let leaf = extract_leaf(&x).unwrap();
                    assert_eq!(leaf.data, vec![Record { key: 8, value: 11 }]);

                    let next = leaf.next.as_ref();
                    let x = next.unwrap().upgrade().unwrap();
                    let x = x.borrow();
                    let next_leaf = extract_leaf(&x).unwrap();
                    // 分割後に追加された値が入っている
                    assert_eq!(next_leaf.data, vec![Record { key: 9, value: 11 }]);
                }
            }
            {
                let p = internal.data.get(1).unwrap();
                let x = p.value.as_ref().borrow();
                let internal = extract_internal(&x).unwrap();
                {
                    let p = internal.data.get(0).unwrap();
                    let x = p.value.as_ref().borrow();
                    let leaf = extract_leaf(&x).unwrap();
                    assert_eq!(leaf.data, vec![Record { key: 9, value: 11 }]);

                    let next = leaf.next.as_ref();
                    let x = next.unwrap().upgrade().unwrap();
                    let x = x.borrow();
                    let next_leaf = extract_leaf(&x).unwrap();
                    // 分割後に追加された値が入っている
                    assert_eq!(
                        next_leaf.data,
                        vec![Record { key: 10, value: 11 }, Record { key: 11, value: 11 }]
                    );
                }
                {
                    let p = internal.data.get(1).unwrap();
                    let x = p.value.as_ref().borrow();
                    let leaf = extract_leaf(&x).unwrap();
                    assert_eq!(
                        leaf.data,
                        vec![Record { key: 10, value: 11 }, Record { key: 11, value: 11 }]
                    );

                    let next = leaf.next.as_ref();
                    assert!(next.is_none());
                }
            }
        }
        // assert_eq!(leaf.data.first().unwrap(), &Record { key: 9, value: 11 });
        // println!("{:?}", n)
    }

    #[test]
    fn a() {
        let v = gen();
        println!("     :{:p}", v);
    }
    fn gen() -> Node {
        let v = Node::Internal(InternalNode {
            cap: 0,
            data: Vec::new(),
        });
        println!("gem():{:p}", v);
        v
    }
}
