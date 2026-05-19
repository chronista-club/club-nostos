//! [`Graph`] ── node 集合 + routing + boundary。 `impl Node for Graph` で自己入れ子する。

use alloc::boxed::Box;
use alloc::vec::Vec;
use nostos::{Outcome, Voyage};

use crate::node::Node;

/// graph 内の node を指す id (= `Graph` の node 配列の index)。
pub type NodeId = usize;

/// graph 評価の構造的エラー (graph 構築の不整合を表す)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// 非 exit node が `Done` を出したが、 下流への route が無い。
    DanglingDone(NodeId),
    /// work-list が尽きたが exit node の terminal に到達しなかった
    /// (例: 非 exit node が `OneWay` の sink で終わった)。
    ExitUnreached,
}

/// node ごとの routing 設定。
struct Routing {
    /// `RoundTrip(Done)` の行き先 (非 exit node 用)。
    done: Option<NodeId>,
    /// `RoundTrip(Failed)` の行き先。 無ければ graph 全体が `Failed` で終端する。
    failed: Option<NodeId>,
}

/// `nostos` primitive を node とする有向グラフ。
///
/// node = [`Node`]、 edge = [`Voyage`] routing。 `Done` は下流 node へ、 `Reborn` は
/// self-loop (同 node 再駆動)、 `Failed` は failed route または graph 終端へ。
///
/// [`Graph`] 自身が [`Node`] を実装する ([`Graph::drive`]) ため、 `Graph` を別の
/// `Graph` の node として畳める ── graph が graph に入れ子する (ADR-0005 D6)。
///
/// # Examples
///
/// ```
/// use nostos::{Outcome, Voyage};
/// use nostos_graph::{Graph, Node};
///
/// struct Identity;
/// impl Node<i32> for Identity {
///     fn drive(&self, input: i32) -> Voyage<i32, i32, i32> {
///         Voyage::RoundTrip(Outcome::Done(input))
///     }
/// }
///
/// let mut graph: Graph<i32> = Graph::new();
/// let n = graph.add_node(Box::new(Identity));
/// graph.set_entry(n);
/// graph.set_exit(n);
/// assert_eq!(graph.evaluate(7), Ok(Voyage::RoundTrip(Outcome::Done(7))));
/// ```
pub struct Graph<V> {
    nodes: Vec<Box<dyn Node<V>>>,
    routes: Vec<Routing>,
    entry: NodeId,
    exit: NodeId,
}

impl<V> Graph<V> {
    /// 空の graph を作る。 node を [`add_node`](Graph::add_node) で足し、
    /// [`set_entry`](Graph::set_entry) / [`set_exit`](Graph::set_exit) で境界を設定する。
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            routes: Vec::new(),
            entry: 0,
            exit: 0,
        }
    }

    /// node を追加し、 その [`NodeId`] を返す。
    pub fn add_node(&mut self, node: Box<dyn Node<V>>) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        self.routes.push(Routing {
            done: None,
            failed: None,
        });
        id
    }

    /// `from` node が `Done` を出した時の行き先を `to` に設定する。
    pub fn route_done(&mut self, from: NodeId, to: NodeId) {
        self.routes[from].done = Some(to);
    }

    /// `from` node が `Failed` を出した時の行き先を `to` に設定する。
    pub fn route_failed(&mut self, from: NodeId, to: NodeId) {
        self.routes[from].failed = Some(to);
    }

    /// entry node (評価の起点) を設定する。
    pub fn set_entry(&mut self, id: NodeId) {
        self.entry = id;
    }

    /// exit node (評価の終点) を設定する。 exit node の terminal が graph 全体の出力。
    pub fn set_exit(&mut self, id: NodeId) {
        self.exit = id;
    }

    /// graph を `input` から評価し、 終端の [`Voyage`] を返す。
    ///
    /// push 型 work-list で駆動する ── node を `drive` → `Voyage` で routing →
    /// 行き先 node を work-list へ。 exit node が terminal を出した時点で停止し、
    /// その `Voyage` が graph 全体の出力になる。 構造的不整合は [`GraphError`]。
    pub fn evaluate(&self, input: V) -> Result<Voyage<V, V, V>, GraphError> {
        let mut worklist: Vec<(NodeId, V)> = Vec::new();
        worklist.push((self.entry, input));

        while let Some((id, value)) = worklist.pop() {
            match self.nodes[id].drive(value) {
                Voyage::RoundTrip(Outcome::Done(v)) => {
                    if id == self.exit {
                        return Ok(Voyage::RoundTrip(Outcome::Done(v)));
                    }
                    match self.routes[id].done {
                        Some(target) => worklist.push((target, v)),
                        None => return Err(GraphError::DanglingDone(id)),
                    }
                }
                Voyage::RoundTrip(Outcome::Reborn(v)) => {
                    // self-loop ── 時間の再帰 (ADR-0005 D5)。
                    worklist.push((id, v));
                }
                Voyage::RoundTrip(Outcome::Failed(v)) => {
                    if id == self.exit {
                        return Ok(Voyage::RoundTrip(Outcome::Failed(v)));
                    }
                    match self.routes[id].failed {
                        Some(target) => worklist.push((target, v)),
                        // route が無ければ graph 全体が Failed で終端。
                        None => return Ok(Voyage::RoundTrip(Outcome::Failed(v))),
                    }
                }
                Voyage::OneWay => {
                    if id == self.exit {
                        return Ok(Voyage::OneWay);
                    }
                    // 非 exit node の OneWay は path の終端 (sink) ── 値は流れない。
                }
            }
        }
        Err(GraphError::ExitUnreached)
    }
}

impl<V> Default for Graph<V> {
    fn default() -> Self {
        Self::new()
    }
}

/// `Graph` 自身が [`Node`] ── graph を別の graph の node として畳める (ADR-0005 D6)。
///
/// 構造的不整合 ([`GraphError`]) は graph 構築のバグであり、 ここでは panic する。
/// 事前に検証したい場合は [`Graph::evaluate`] を直接使う。
impl<V> Node<V> for Graph<V> {
    fn drive(&self, input: V) -> Voyage<V, V, V> {
        self.evaluate(input).expect("malformed graph")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- テスト用 Node ---

    /// 受けた値をそのまま `Done`。
    struct Identity;
    impl Node<i32> for Identity {
        fn drive(&self, input: i32) -> Voyage<i32, i32, i32> {
            Voyage::RoundTrip(Outcome::Done(input))
        }
    }

    /// `limit` 未満は `Reborn(input+1)`、 到達で `Done`。
    struct CountTo {
        limit: i32,
    }
    impl Node<i32> for CountTo {
        fn drive(&self, input: i32) -> Voyage<i32, i32, i32> {
            if input < self.limit {
                Voyage::RoundTrip(Outcome::Reborn(input + 1))
            } else {
                Voyage::RoundTrip(Outcome::Done(input))
            }
        }
    }

    /// 常に `Failed`。
    struct Fail;
    impl Node<i32> for Fail {
        fn drive(&self, input: i32) -> Voyage<i32, i32, i32> {
            Voyage::RoundTrip(Outcome::Failed(input))
        }
    }

    /// 常に `OneWay`。
    struct Notify;
    impl Node<i32> for Notify {
        fn drive(&self, _input: i32) -> Voyage<i32, i32, i32> {
            Voyage::OneWay
        }
    }

    #[test]
    fn linear_chain_routes_done_downstream() {
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Identity));
        let b = g.add_node(Box::new(Identity));
        g.route_done(a, b);
        g.set_entry(a);
        g.set_exit(b);
        assert_eq!(g.evaluate(5), Ok(Voyage::RoundTrip(Outcome::Done(5))));
    }

    #[test]
    fn reborn_drives_self_loop() {
        let mut g: Graph<i32> = Graph::new();
        let n = g.add_node(Box::new(CountTo { limit: 3 }));
        g.set_entry(n);
        g.set_exit(n);
        // 0 → 1 → 2 → 3 と Reborn self-loop、 3 で Done。
        assert_eq!(g.evaluate(0), Ok(Voyage::RoundTrip(Outcome::Done(3))));
    }

    #[test]
    fn failed_at_exit_is_graph_output() {
        let mut g: Graph<i32> = Graph::new();
        let n = g.add_node(Box::new(Fail));
        g.set_entry(n);
        g.set_exit(n);
        assert_eq!(g.evaluate(7), Ok(Voyage::RoundTrip(Outcome::Failed(7))));
    }

    #[test]
    fn failed_routes_to_failed_target() {
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Fail));
        let b = g.add_node(Box::new(Identity));
        g.route_failed(a, b);
        g.set_entry(a);
        g.set_exit(b);
        // a が Failed(7) → failed route で b へ → b が Done(7)。
        assert_eq!(g.evaluate(7), Ok(Voyage::RoundTrip(Outcome::Done(7))));
    }

    #[test]
    fn oneway_at_exit_is_graph_output() {
        let mut g: Graph<i32> = Graph::new();
        let n = g.add_node(Box::new(Notify));
        g.set_entry(n);
        g.set_exit(n);
        assert_eq!(g.evaluate(1), Ok(Voyage::OneWay));
    }

    #[test]
    fn dangling_done_is_error() {
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Identity));
        let b = g.add_node(Box::new(Identity));
        g.set_entry(a);
        g.set_exit(b);
        // a は Done を出すが done route が無い、 かつ a != exit。
        assert_eq!(g.evaluate(5), Err(GraphError::DanglingDone(a)));
    }

    #[test]
    fn oneway_sink_at_non_exit_leaves_exit_unreached() {
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Notify));
        let b = g.add_node(Box::new(Identity));
        g.set_entry(a);
        g.set_exit(b);
        // a の OneWay は sink、 exit b に到達しない。
        assert_eq!(g.evaluate(1), Err(GraphError::ExitUnreached));
    }

    #[test]
    fn graph_nests_in_graph() {
        // inner graph ── Identity 1 個。
        let mut inner: Graph<i32> = Graph::new();
        let i = inner.add_node(Box::new(Identity));
        inner.set_entry(i);
        inner.set_exit(i);

        // outer graph ── inner graph を 1 個の node として畳む (ADR-0005 D6)。
        let mut outer: Graph<i32> = Graph::new();
        let nested = outer.add_node(Box::new(inner));
        outer.set_entry(nested);
        outer.set_exit(nested);

        assert_eq!(outer.evaluate(9), Ok(Voyage::RoundTrip(Outcome::Done(9))));
    }
}
