use std::fmt::Display;

#[derive(Debug)]
pub struct Data<T>
where
    T: Display,
{
    id: usize,
    data: T,
    #[allow(dead_code)]
    next_id: Option<usize>,
}

impl<T: Display> Data<T> {
    pub fn new(id: usize, data: T) -> Self {
        Self {
            id,
            data,
            next_id: None,
        }
    }
}

#[derive(Debug)]
pub struct BPlusTree<T>
where
    T: Display,
{
    cap: usize,
    node: Option<Node>,
    data: Vec<Data<T>>,
}

impl<T: Display> BPlusTree<T> {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            node: None,
            data: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: Key, mut data: Data<T>) {
        // data.id は self.dataのindexが入る
        // この値は現在の長さに等しい
        let data_id = self.data.len();
        data.id = data_id;
        self.data.push(data);

        if self.node.is_none() {
            let child = Node::Leaf(LeafNode {
                cap: self.cap,
                data_ids: vec![DataPair::new(key, data_id)],
            });
            self.node = Some(child);
            return;
        }

        let splited = self.node.as_mut().and_then(|n| n.insert(key, data_id));
        if let Some(node) = splited {
            let old_child = self.node.take().unwrap();
            let new_child = InternalNode {
                cap: self.cap,
                nodes: vec![
                    NodePair::new(old_child.min_key().unwrap(), old_child),
                    NodePair::new(node.min_key().unwrap(), node),
                ],
            };
            self.node = Some(Node::Internal(new_child));
        }
    }

    pub fn search(&self, key: Key) -> Option<&T> {
        self.node
            .as_ref()
            .and_then(|n| n.search(key))
            .and_then(|data_id| self.data.get(data_id))
            .map(|d| &d.data)
    }
}

pub type Key = usize;
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
type DataPair = Pair<usize>;

#[derive(Debug)]
enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

impl Node {
    #[must_use = "insertion may fail"]
    pub fn insert(&mut self, key: Key, data_id: usize) -> Option<Node> {
        match self {
            Node::Internal(internal) => internal.insert(key, data_id),
            Node::Leaf(leaf) => leaf.insert(key, data_id),
        }
    }

    pub fn search(&self, key: Key) -> Option<usize> {
        match self {
            Node::Internal(internal) => internal.search(key),
            Node::Leaf(leaf) => leaf.search(key),
        }
    }

    fn min_key(&self) -> Option<usize> {
        match self {
            Node::Internal(internal) => internal.nodes.first().map(|p| p.key),
            Node::Leaf(leaf) => leaf.data_ids.first().map(|r| r.key),
        }
    }
}

#[derive(Debug)]
struct InternalNode {
    cap: usize,
    // Vec ではなく配列にしてもいいかも
    nodes: Vec<NodePair>,
}

impl InternalNode {
    fn insert(&mut self, key: Key, data_id: usize) -> Option<Node> {
        // TODO 同値のkeyが存在している場合がおかしいので、要修正
        if self.nodes.is_empty() {
            self.nodes.push(Pair::new(
                key,
                Node::Leaf(LeafNode {
                    cap: self.cap,
                    data_ids: vec![Pair::new(key, data_id)],
                }),
            ));
            return None;
        }
        let node = self.find_node_for_insert(key, data_id);
        let splited = node.value.insert(key, data_id);
        if let Some(n) = splited {
            if let Some(k) = n.min_key() {
                self.nodes.push(Pair { key: k, value: n });
                self.nodes.sort_by_key(|p| p.key);
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

    fn find_node_for_insert(&mut self, key: Key, data_id: usize) -> &mut NodePair {
        if self.nodes.is_empty() {
            self.nodes.push(NodePair::new(
                key,
                Node::Leaf(LeafNode {
                    cap: self.cap,
                    data_ids: vec![DataPair::new(key, data_id)],
                }),
            ))
        }
        return self.find_mut_node(key).unwrap();
    }

    fn find_mut_node(&mut self, key: Key) -> Option<&mut NodePair> {
        let exist = self.nodes.iter().any(|pair| pair.key < key);
        if exist {
            self.nodes
                .iter_mut()
                .take_while(|pair| pair.key < key)
                .last()
        } else {
            self.nodes.first_mut()
        }
    }

    pub fn search(&self, key: Key) -> Option<usize> {
        // TODO 同値のkeyが存在している場合がおかしいので、要修正
        let p = self.find_node(key);
        p.and_then(|p| p.value.search(key))
    }

    fn find_node(&self, key: Key) -> Option<&NodePair> {
        self.nodes
            .iter()
            .take_while(|pair| pair.key < key)
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
    // Vec ではなく配列にしてもいいかも
    data_ids: Vec<DataPair>,
}

impl LeafNode {
    fn insert(&mut self, key: Key, data_id: usize) -> Option<Node> {
        // 末尾に常に入れるわけではない
        self.data_ids.push(DataPair::new(key, data_id));
        self.data_ids.sort_by_key(|r| r.key);
        if self.is_full() {
            return Some(self.split());
        }
        None
    }

    fn split(&mut self) -> Node {
        let right = self.data_ids.split_off(self.data_ids.len() / 2);
        let new_next = Self {
            cap: self.cap,
            data_ids: right,
        };
        Node::Leaf(new_next)
    }

    pub fn search(&self, key: Key) -> Option<usize> {
        self.data_ids.iter().find(|p| p.key == key).map(|p| p.value)
    }

    // capacityに空きがあるかどうか
    fn is_full(&self) -> bool {
        self.data_ids.len() > self.cap
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn insert() {
        // {
        //     let mut b = BPlusTree::<i64>::new(3);
        //     b.insert(1, Data::new(0, -1));
        //     let r = b.search(1);
        //     assert!(r.is_some());
        //     assert_eq!(*r.unwrap(), -1)
        // }
        {
            let mut b = BPlusTree::<i64>::new(3);
            b.insert(1, Data::new(0, -1));
            b.insert(5, Data::new(0, -5));
            b.insert(2, Data::new(0, -2));
            b.insert(4, Data::new(0, -4));
            b.insert(3, Data::new(0, -3));
            // dbg!(b);
            let r = b.search(5);
            assert!(r.is_some());
            assert_eq!(*r.unwrap(), -5)
        }
    }
}
