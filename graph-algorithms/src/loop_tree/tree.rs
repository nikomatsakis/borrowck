use super::super::Graph;
use super::super::node_vec::NodeVec;

pub struct LoopTree<G: Graph> {
    loop_ids: NodeVec<G, Option<LoopId>>,
    loop_infos: Vec<LoopInfo<G>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoopId {
    index: usize
}

struct LoopInfo<G: Graph> {
    parent: Option<LoopId>,
    head: G::Node,
    exits: Vec<G::Node>,
}

impl<G: Graph> LoopTree<G> {
    pub fn new(graph: &G) -> Self {
        LoopTree {
            loop_ids: NodeVec::from_default(graph),
            loop_infos: vec![]
        }
    }

    pub fn new_loop(&mut self, head: G::Node) -> LoopId {
        let loop_id = LoopId { index: self.loop_infos.len() };
        self.loop_infos.push(LoopInfo {
            parent: None, // will get updated later
            head: head,
            exits: vec![]
        });
        loop_id
    }

    pub fn set_parent(&mut self, loop_id: LoopId, parent_loop_id: Option<LoopId>) {
        self.loop_infos[loop_id.index].parent = parent_loop_id;
    }

    pub fn parent(&self, loop_id: LoopId) -> Option<LoopId> {
        self.loop_infos[loop_id.index].parent
    }

    pub fn parents(&self, loop_id: LoopId) -> Parents<G> {
        Parents { tree: self, next_loop_id: self.parent(loop_id) }
    }

    pub fn loop_head(&self, loop_id: LoopId) -> G::Node {
        self.loop_infos[loop_id.index].head
    }

    pub fn loop_head_of_node(&self, node: G::Node) -> Option<G::Node> {
        self.loop_id(node).map(|loop_id| self.loop_head(loop_id))
    }

    pub fn loop_exits(&self, loop_id: LoopId) -> &[G::Node] {
        &self.loop_infos[loop_id.index].exits
    }

    pub fn push_loop_exit(&mut self, loop_id: LoopId, exit: G::Node) {
        self.loop_infos[loop_id.index].exits.push(exit);
    }

    pub fn loop_id(&self, node: G::Node) -> Option<LoopId> {
        self.loop_ids[node]
    }

    pub fn set_loop_id(&mut self, node: G::Node, id: Option<LoopId>) {
        self.loop_ids[node] = id;
    }
}

pub struct Parents<'iter, G: Graph + 'iter> {
    tree: &'iter LoopTree<G>,
    next_loop_id: Option<LoopId>
}

impl<'iter, G: Graph> Iterator for Parents<'iter, G> {
    type Item = LoopId;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_loop_id {
            Some(loop_id) => {
                self.next_loop_id = self.tree.parent(loop_id);
                Some(loop_id)
            }
            None => {
                None
            }
        }
    }
}
