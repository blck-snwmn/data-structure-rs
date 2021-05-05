use std::{
    cell::RefCell,
    ops::Deref,
    rc::{Rc, Weak},
};

use anyhow::Result;

type RefNode = Rc<RefCell<Node>>;
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
struct Record {
    key: usize,
    value: usize,
}
#[derive(Debug)]
enum Node {
    Root(RootNode),
    Internal(InternalNode),
    Leaf(LeafNode),
}
impl Node {
    fn new(cap: usize) -> Self {
        Node::Root(RootNode { cap, data: None })
    }

    #[must_use = "insertion may fail"]
    fn insert(&mut self, data: Record) -> Result<Option<RefNode>> {
        match self {
            Node::Root(root) => root.insert(data),
            Node::Internal(internal) => internal.insert(data),
            Node::Leaf(leaf) => leaf.insert(data),
        }
    }

    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        match self {
            Node::Root(root) => root.is_full(),
            Node::Internal(internal) => internal.is_full(),
            Node::Leaf(leaf) => leaf.is_full(),
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
struct RootNode {
    cap: usize,
    data: Option<Box<Node>>,
}
impl RootNode {
    fn insert(&mut self, data: Record) -> Result<Option<RefNode>> {
        // 末尾に常に入れるわけではない
        if self.data.is_none() {
            self.data = Some(Box::new(Node::Leaf(LeafNode::new(self.cap, data))));
            return Ok(None);
        }
        let n = self.data.as_deref_mut().unwrap();
        let inserted = n.insert(data)?;
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
        Ok(None)
    }
    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        self.data.is_some()
    }
}
#[derive(Debug)]
struct InternalNode {
    cap: usize,
    data: Vec<Pair>,
}
impl InternalNode {
    fn insert(&mut self, data: Record) -> Result<Option<RefNode>> {
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
        let mut x = self
            .data
            .iter_mut()
            .take_while(|pair| pair.key < data.key)
            .last();
        if x.is_none() {
            x = self.data.first_mut();
        }
        if let Some(p) = x {
            let splited = p.value.borrow_mut().insert(data)?;
            if let Some(n) = splited {
                if let Some(k) = n.borrow().min_key() {
                    self.data.push(Pair {
                        key: k,
                        value: n.clone(),
                    });
                    self.data.sort_by_key(|p| p.key);
                }
            }
        }
        //  else {
        //     self.data.push(Pair::new(
        //         data.key,
        //         Node::Leaf(LeafNode {
        //             cap: self.cap,
        //             data: Vec::new(),
        //             next: None,
        //         }),
        //     ));
        //     let p = self.data.first_mut().unwrap();
        //     let splited = p.value.borrow_mut().insert(data)?;
        //     if let Some(n) = splited {
        //         if let Some(k) = n.borrow().min_key() {
        //             self.data.push(Pair {
        //                 key: k,
        //                 value: n.clone(),
        //             })
        //         }
        //     }
        // }
        // TODO key の探し方おかしい

        if self.is_full() {
            // split

            return self.split();
        }
        return Ok(None);
    }

    fn split(&mut self) -> Result<Option<RefNode>> {
        let right = self.data.split_off(self.data.len() / 2);
        let new_next = Self {
            cap: self.cap,
            data: right,
        };
        Ok(Some(Rc::new(RefCell::new(Node::Internal(new_next)))))
    }

    // fn find_node(&mut self, data: Record) -> &mut Pair {
    //     if self.data.is_empty() {
    //         self.data.push(Pair::new(
    //             data.key,
    //             Node::Leaf(LeafNode {
    //                 cap: self.cap,
    //                 data: Vec::new(),
    //                 next: None,
    //             }),
    //         ))
    //     }
    //     let mut x = self
    //         .data
    //         .iter_mut()
    //         .take_while(|pair| pair.key < data.key)
    //         .last();
    //     if x.is_none() {
    //         return self.data.first_mut().unwrap();
    //     }
    //     x.unwrap()
    // }

    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        // 暗黙的に最小値のキー分保持している
        // その分がcapを圧迫しちゃうので、そのサイズ分無視するために１加算
        self.data.len() > (self.cap + 1)
    }
}
#[derive(Debug)]
struct LeafNode {
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
    fn insert(&mut self, data: Record) -> Result<Option<RefNode>> {
        // 末尾に常に入れるわけではない
        self.data.push(data);
        self.data.sort_by_key(|r| r.key);
        if self.is_full() {
            return self.split();
        }
        Ok(None)
    }

    fn split(&mut self) -> Result<Option<RefNode>> {
        let right = self.data.split_off(self.data.len() / 2);
        let next = self.next.take();
        let new_next = Self {
            cap: self.cap,
            data: right,
            next,
        };
        let ref_node = Rc::new(RefCell::new(Node::Leaf(new_next)));
        self.next = Some(Rc::downgrade(&ref_node));
        Ok(Some(ref_node))
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
            let _ = n.insert(Record { key: 9, value: 11 }).unwrap();
            {
                let root = extract_root(&n).unwrap();
                assert!(root.data.is_some());
                let leaf = extract_leaf(root.data.as_deref().unwrap()).unwrap();
                assert_eq!(leaf.data.len(), 1);
                assert_eq!(leaf.data.first().unwrap(), &Record { key: 9, value: 11 });
            }
            let _ = n.insert(Record { key: 8, value: 11 }).unwrap();
            let _ = n.insert(Record { key: 7, value: 11 }).unwrap();
            {
                let root = extract_root(&n).unwrap();
                assert!(root.data.is_some());
                let leaf = extract_leaf(root.data.as_deref().unwrap()).unwrap();
                assert_eq!(leaf.data.len(), 3);
            }
            let _ = n.insert(Record { key: 10, value: 11 }).unwrap();
            {
                let root = extract_root(&n).unwrap();
                let leaf = extract_internal(root.data.as_deref().unwrap()).unwrap();
                assert_eq!(leaf.data.len(), 2);
            }
            let _ = n.insert(Record { key: 11, value: 11 }).unwrap();

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
            let _ = n.insert(Record { key: 9, value: 11 }).unwrap();
            let _ = n.insert(Record { key: 8, value: 11 }).unwrap();
            let _ = n.insert(Record { key: 7, value: 11 }).unwrap();
            let _ = n.insert(Record { key: 10, value: 11 }).unwrap();
            let _ = n.insert(Record { key: 11, value: 11 }).unwrap();

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
