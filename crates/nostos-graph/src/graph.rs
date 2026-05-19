//! [`Graph`] ── node 集合 + routing + boundary。 `impl Node for Graph` で自己入れ子する。

use alloc::boxed::Box;
use alloc::vec::Vec;
use nostos::{Outcome, Voyage};

use crate::node::Node;

/// graph 内の node を指す id (= `Graph` の node 配列の index)。
pub type NodeId = usize;

/// `OneWay` を出した node の拡散 policy (ADR-0008)。
///
/// `Voyage::OneWay(payload)` を受けた評価器が、 その payload を graph 構造へ
/// さらに伝播させるか否かを決める。 payload は core (`Voyage`) の責務、
/// 伝播 (spread) は graph topology の責務 ── 本型がその境界。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spread {
    /// fan-out ── payload を全近傍 node へ broadcast (空間の再帰)。
    Ok,
    /// sink ── その node で消費、 伝播しない。
    Ng,
}

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
    /// `OneWay` の拡散 policy。
    spread: Spread,
    /// `Spread::Ok` での fan-out 先。
    neighbors: Vec<NodeId>,
}

/// `nostos` primitive を node とする有向グラフ。
///
/// node = [`Node`]、 edge = [`Voyage`] routing。 `Done` は下流 node へ、 `Reborn` は
/// self-loop (時間の再帰)、 `Failed` は failed route または graph 終端へ、
/// `OneWay` は [`Spread`] policy に従い fan-out (空間の再帰) または sink。
///
/// [`Graph`] 自身が [`Node`] を実装する ([`Graph::evaluate`]) ため、 `Graph` を別の
/// `Graph` の node として畳める ── graph が graph に入れ子する (深さの再帰、 ADR-0005 D6)。
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
            spread: Spread::Ng,
            neighbors: Vec::new(),
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

    /// `node` が `OneWay` を出した時の拡散 policy を設定する (既定は [`Spread::Ng`])。
    pub fn set_spread(&mut self, node: NodeId, spread: Spread) {
        self.routes[node].spread = spread;
    }

    /// `from` node の `Spread::Ok` fan-out 先に `to` を追加する。
    pub fn add_neighbor(&mut self, from: NodeId, to: NodeId) {
        self.routes[from].neighbors.push(to);
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
    ///
    /// `OneWay` の `Spread::Ok` fan-out は payload を複数 node へ複製するため、
    /// `V: Clone` を要する。
    pub fn evaluate(&self, input: V) -> Result<Voyage<V, V, V>, GraphError>
    where
        V: Clone,
    {
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
                Voyage::OneWay(v) => {
                    if id == self.exit {
                        return Ok(Voyage::OneWay(v));
                    }
                    match self.routes[id].spread {
                        // fan-out ── payload を全近傍へ broadcast (空間の再帰)。
                        Spread::Ok => {
                            for &neighbor in &self.routes[id].neighbors {
                                worklist.push((neighbor, v.clone()));
                            }
                        }
                        // sink ── payload は消費され、 伝播しない。
                        Spread::Ng => {}
                    }
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
/// 事前に検証したい場合は [`Graph::evaluate`] を直接使う。 `OneWay` fan-out のため
/// `V: Clone` を要する。
impl<V: Clone> Node<V> for Graph<V> {
    fn drive(&self, input: V) -> Voyage<V, V, V> {
        self.evaluate(input).expect("malformed graph")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::rc::Rc;
    use core::cell::Cell;

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

    /// 常に `OneWay(input)` ── 一方向に payload を発する。
    struct Notify;
    impl Node<i32> for Notify {
        fn drive(&self, input: i32) -> Voyage<i32, i32, i32> {
            Voyage::OneWay(input)
        }
    }

    /// 駆動されるたび counter を増やし、 `OneWay(input)` (sink) を返す。
    struct Hit(Rc<Cell<i32>>);
    impl Node<i32> for Hit {
        fn drive(&self, input: i32) -> Voyage<i32, i32, i32> {
            self.0.set(self.0.get() + 1);
            Voyage::OneWay(input)
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
        assert_eq!(g.evaluate(1), Ok(Voyage::OneWay(1)));
    }

    #[test]
    fn oneway_sink_at_non_exit_leaves_exit_unreached() {
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Notify));
        let b = g.add_node(Box::new(Identity));
        g.set_entry(a);
        g.set_exit(b);
        // a の OneWay は spread 既定 Ng = sink、 exit b に到達しない。
        assert_eq!(g.evaluate(1), Err(GraphError::ExitUnreached));
    }

    #[test]
    fn oneway_spread_ok_fans_out_to_neighbor() {
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Notify));
        let b = g.add_node(Box::new(Identity));
        g.set_spread(a, Spread::Ok);
        g.add_neighbor(a, b);
        g.set_entry(a);
        g.set_exit(b);
        // a が OneWay(7) → spread Ok → payload 7 を b へ fan-out → b が Done(7)。
        assert_eq!(g.evaluate(7), Ok(Voyage::RoundTrip(Outcome::Done(7))));
    }

    #[test]
    fn oneway_spread_ok_drives_every_neighbor() {
        let hits = Rc::new(Cell::new(0));
        let mut g: Graph<i32> = Graph::new();
        let a = g.add_node(Box::new(Notify));
        let b = g.add_node(Box::new(Hit(hits.clone())));
        let c = g.add_node(Box::new(Hit(hits.clone())));
        let d = g.add_node(Box::new(Identity));
        g.set_spread(a, Spread::Ok);
        g.add_neighbor(a, b);
        g.add_neighbor(a, c);
        g.set_entry(a);
        g.set_exit(d); // d は到達不能 ── fan-out は exit short-circuit せず全て走る
        // a OneWay → b, c へ fan-out。 b, c は Hit (sink)、 d 未到達。
        assert_eq!(g.evaluate(7), Err(GraphError::ExitUnreached));
        // 評価順序に依らず b・c の両方が駆動された。
        assert_eq!(hits.get(), 2);
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
