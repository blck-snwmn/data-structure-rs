use std::{
    fmt::{self},
    ptr,
};
pub type Key = usize;
pub type Data = usize;
#[derive(Debug)]
struct Pair<T> {
    key: Key,
    value: T,
}

impl<T> Pair<T> {
    fn new(key: Key, value: T) -> Self {
        Self { key, value }
    }
}

type NodePair = Pair<Node>;
type DataPair = Pair<Data>;

#[derive(Debug)]
pub struct BPlusTree {
    cap: usize,
    node: Option<Node>,
}

impl BPlusTree {
    pub fn new(cap: usize) -> Self {
        Self { cap, node: None }
    }

    pub fn insert(&mut self, key: Key, data: Data) {
        // data.id は self.dataのindexが入る
        // この値は現在の長さに等しい
        if self.node.is_none() {
            let child = Node::Leaf(LeafNode {
                cap: self.cap,
                data: vec![DataPair::new(key, data)],
                next: ptr::null_mut(),
            });
            self.node = Some(child);
            return;
        }

        let splited = self.node.as_mut().and_then(|n| n.insert(key, data));
        if let Some(node) = splited {
            let old_child = self.node.take().unwrap();
            let mut new_child = InternalNode {
                cap: self.cap,
                nodes: vec![
                    NodePair {
                        key: old_child.min_key().unwrap(),
                        value: old_child,
                    },
                    NodePair {
                        key: node.min_key().unwrap(),
                        value: node,
                    },
                ],
            };
            let mut next_node_ptr = ptr::null();
            if let Node::Leaf(node) = &new_child.nodes.get(1).unwrap().value {
                next_node_ptr = node as *const _;
            }
            if let Node::Leaf(node) = &mut new_child.nodes.get_mut(0).unwrap().value {
                node.next = next_node_ptr;
            }

            self.node = Some(Node::Internal(new_child));
        }
    }

    pub fn search(&self, key: Key) -> Option<&Data> {
        self.node.as_ref().and_then(|n| n.search(key))
    }

    pub fn search_range(&self, min_key: Key, max_key: Key) -> Vec<&Data> {
        self.node
            .as_ref()
            .map(|n| n.search_range(min_key, max_key))
            .unwrap_or_default()
    }
}
#[derive(Debug)]
enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

impl Node {
    #[must_use = "insertion may fail"]
    fn insert(&mut self, key: Key, data: Data) -> Option<Node> {
        match self {
            Node::Internal(internal) => internal.insert(key, data),
            Node::Leaf(leaf) => leaf.insert(key, data),
        }
    }

    fn search(&self, key: Key) -> Option<&Data> {
        match self {
            Node::Internal(internal) => internal.search(key),
            Node::Leaf(leaf) => leaf.search(key),
        }
    }

    fn search_range(&self, min_key: Key, max_key: Key) -> Vec<&Data> {
        if min_key > max_key {
            return Vec::new();
        }
        match self {
            Node::Internal(internal) => internal.search_range(min_key, max_key),
            Node::Leaf(leaf) => leaf.search_range(min_key, max_key),
        }
    }

    fn min_key(&self) -> Option<usize> {
        match self {
            Node::Internal(internal) => internal.nodes.first().map(|p| p.key),
            Node::Leaf(leaf) => leaf.data.first().map(|r| r.key),
        }
    }
}

#[derive(Debug)]
struct InternalNode {
    cap: usize,
    // Vec ではなく配列にしてもいいかも。const generics
    nodes: Vec<NodePair>,
}
impl InternalNode {
    fn insert(&mut self, key: Key, data: Data) -> Option<Node> {
        // TODO 同値のkeyが存在している場合がおかしいので、要修正
        if self.nodes.is_empty() {
            self.nodes.push(NodePair::new(
                key,
                Node::Leaf(LeafNode {
                    cap: self.cap,
                    data: vec![DataPair::new(key, data)],
                    next: ptr::null_mut(),
                }),
            ));
            return None;
        }
        let node = self.find_node_for_insert(key, data);
        let splited_node = node.value.insert(key, data);
        if let Some(n) = splited_node {
            if let Some(k) = n.min_key() {
                self.nodes.push(Pair { key: k, value: n });
                self.nodes.sort_by_key(|p| p.key);
                // 並び替えたので、nextを並び替え後のものに変更
                // TODO 全要素を付け替える実装をやめる
                let mut next_node_ptr = ptr::null();
                for n in self.nodes.iter_mut().rev().map(|p| &mut p.value) {
                    if let Node::Leaf(node) = n {
                        node.next = next_node_ptr;
                        next_node_ptr = node as *const _;
                    }
                }
            }
        }
        if self.is_full() {
            return Some(self.split());
        }
        None
    }

    fn split(&mut self) -> Node {
        let right = self.nodes.split_off(self.nodes.len() / 2);
        let new_next = Self {
            cap: self.cap,
            nodes: right,
        };
        Node::Internal(new_next)
    }

    fn find_node_for_insert(&mut self, key: Key, data: Data) -> &mut NodePair {
        if self.nodes.is_empty() {
            self.nodes.push(NodePair::new(
                key,
                Node::Leaf(LeafNode {
                    cap: self.cap,
                    data: vec![DataPair::new(key, data)],
                    next: ptr::null_mut(),
                }),
            ))
        }
        return self.find_mut_node(key).unwrap();
    }

    fn find_mut_node(&mut self, key: Key) -> Option<&mut NodePair> {
        let exist = self.nodes.iter().any(|pair| pair.key <= key);
        if exist {
            self.nodes
                .iter_mut()
                .take_while(|pair| pair.key <= key)
                .last()
        } else {
            self.nodes.first_mut()
        }
    }

    fn search(&self, key: Key) -> Option<&Data> {
        // TODO 同値のkeyが存在している場合がおかしいので、要修正
        let p = self.find_node(key);
        p.and_then(|p| p.value.search(key))
    }

    fn search_range(&self, min_key: Key, max_key: Key) -> Vec<&Data> {
        let p = self.find_node(min_key);
        p.map(|p| p.value.search_range(min_key, max_key))
            .unwrap_or_default()
    }

    fn find_node(&self, key: Key) -> Option<&NodePair> {
        self.nodes
            .iter()
            .take_while(|pair| pair.key <= key)
            .last()
            .or_else(|| self.nodes.first())
    }

    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        // 暗黙的に最小値のキー分保持している
        // その分がcapを圧迫しちゃうので、そのサイズ分無視するために１加算
        self.nodes.len() > (self.cap + 1)
    }
}
#[derive(Debug)]
struct LeafNode {
    cap: usize,
    data: Vec<DataPair>, // TODO generics
    next: *const LeafNode,
}

impl fmt::Pointer for LeafNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // use `as` to convert to a `*const T`, which implements Pointer, which we can use
        let ptr = self as *const Self;
        fmt::Pointer::fmt(&ptr, f)
    }
}

impl LeafNode {
    fn insert(&mut self, key: Key, data_id: Data) -> Option<Node> {
        // 末尾に常に入れるわけではない
        self.data.push(DataPair::new(key, data_id));
        self.data.sort_by_key(|r| r.key);
        if self.is_full() {
            return Some(self.split());
        }
        None
    }

    fn split(&mut self) -> Node {
        let right = self.data.split_off(self.data.len() / 2);
        let mut new_next = Self {
            cap: self.cap,
            data: right,
            next: ptr::null_mut(),
        };
        // 以下のようになるので、self.nextを引き継ぐ
        //   before split: self->other
        //   after  split: self->new_next->other
        new_next.next = self.next;
        Node::Leaf(new_next)
    }

    fn search(&self, key: Key) -> Option<&Data> {
        self.data.iter().find(|p| p.key == key).map(|p| &p.value)
    }

    fn search_range(&self, min_key: Key, max_key: Key) -> Vec<&Data> {
        let mut target_leaf_node = Some(self);
        let mut result: Vec<&Data> = self
            .data
            .iter()
            .filter(|p| p.key >= min_key && p.key <= max_key)
            .map(|x| &x.value)
            .collect();
        loop {
            target_leaf_node = target_leaf_node.and_then(|x| unsafe { x.next.as_ref() });
            if target_leaf_node.is_none() {
                break result;
            }
            let target_leaf_node = target_leaf_node.unwrap();
            let mut data: Vec<&Data> = target_leaf_node
                .data
                .iter()
                .filter(|x| x.key <= max_key)
                .map(|x| &x.value)
                .collect();
            let l = data.len();
            result.append(&mut data);
            if l < target_leaf_node.data.len() {
                break result;
            }
        }
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
    #[test]
    fn insert() {
        {
            let mut b = BPlusTree::new(3);
            b.insert(1, 1);
            let r = b.search(1);
            assert!(r.is_some());
            assert_eq!(*r.unwrap(), 1)
        }
        {
            let mut b = BPlusTree::new(3);
            b.insert(11, 11);
            b.insert(25, 25);
            b.insert(12, 12);
            b.insert(24, 24);
            b.insert(13, 13);
            b.insert(10, 10);
            b.insert(14, 14);
            // dbg!(b);
            {
                let r = b.search(24);
                assert!(r.is_some());
                assert_eq!(*r.unwrap(), 24)
            }
            {
                let r = b.search(10);
                assert!(r.is_some());
                assert_eq!(*r.unwrap(), 10)
            }
            {
                let r = b.search(11);
                assert!(r.is_some());
                assert_eq!(*r.unwrap(), 11)
            }
            {
                let r = b.search(12);
                assert!(r.is_some());
                assert_eq!(*r.unwrap(), 12)
            }
            {
                let r = b.search_range(11, 11);
                assert_eq!(r, vec![&11]);
            }
        }
        {
            let mut b = BPlusTree::new(3);
            b.insert(11, 11);
            b.insert(25, 25);
            b.insert(12, 12);
            b.insert(24, 24);
            b.insert(13, 13);
            b.insert(10, 10);
            b.insert(14, 14);
            // dbg!(&b);
            // let node = &b.node;
            // if let Some(Node::Internal(ni)) = &node {
            //     for n in &ni.nodes {
            //         if let Node::Leaf(ln) = &n.value {
            //             // println!("test:own={:p}: next:{:?}", &ln as  *const LeafNode, ln.next);
            //             // println!(
            //             //     "test:own={:?}: next:{:?}; data:{:?}",
            //             //     ln as *const LeafNode, ln.next, ln.data
            //             // );
            //             // let x = unsafe { ln.next.as_ref() };
            //             // if let Some(x) = x {
            //             //     println!("{:?}{:?}", x.cap, x.data)
            //             // }
            //         }
            //     }
            // }
            {
                let r = b.search_range(11, 11);
                assert_eq!(r, vec![&11]);
            }
            {
                let r = b.search_range(11, 13);
                assert_eq!(r, vec![&11, &12, &13]);
            }
            {
                let r = b.search_range(11, 24);
                assert_eq!(r, vec![&11, &12, &13, &14, &24]);
            }
            {
                let r = b.search_range(0, 100);
                assert_eq!(r, vec![&10, &11, &12, &13, &14, &24, &25]);
            }
        }
        {
            let mut b = BPlusTree::new(3);
            b.insert(11, 11);
            b.insert(25, 25);
            b.insert(12, 12);
            b.insert(14, 14);
            b.insert(15, 15);
            b.insert(16, 16);
            b.insert(17, 17);
            // dbg!(b);
            {
                let r = b.search_range(11, 11);
                assert_eq!(r, vec![&11]);
            }
        }
    }

    struct TestData {
        #[allow(dead_code)]
        s: String,
    }

    impl fmt::Pointer for TestData {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // use `as` to convert to a `*const T`, which implements Pointer, which we can use
            let ptr = self as *const Self;
            fmt::Pointer::fmt(&ptr, f)
        }
    }
    fn return_data() -> TestData {
        let d = TestData {
            s: "aaaa".to_string(),
        };
        println!("{:p}", d);
        d
    }
    #[test]
    fn test_adress() {
        let d = return_data();
        println!("{:p}", d);
    }
}
