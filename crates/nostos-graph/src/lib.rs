//! # nostos-graph
//!
//! `nostos` primitive を node とする有向グラフ substrate (Axis D)。
//!
//! - **node** = [`Node`] ── [`nostos::Bracket`] の `enter`+`exit` を `drive` 1 手に畳んだ
//!   object-safe な層。 graph は heterogeneous な `Box<dyn Node<V>>` を扱う
//! - **edge** = [`nostos::Voyage`] routing ── `Done` は下流へ、 `Reborn` は self-loop、
//!   `Failed` は error edge、 `OneWay` は [`Spread`] policy で fan-out / sink
//! - [`Graph`] 自身が [`Node`] を実装する ── graph が graph に入れ子する (ADR-0005 D6)
//!
//! 詳細設計は [ADR-0005] (framing) と [ADR-0007] (具体)。
//!
//! [ADR-0005]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0005-graph-substrate.md
//! [ADR-0007]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0007-graph-design.md
//!
//! ## Status
//!
//! RoundTrip routing (`Done` / `Reborn` / `Failed`) と `OneWay` の [`Spread`] routing
//! (fan-out / sink、 ADR-0008) を実装済み。 graph の再帰三軸 ── 時間 (`Reborn`
//! self-loop) / 空間 (`Spread::Ok` fan-out) / 深さ (`Graph: Node` 入れ子) ── が揃った。

#![no_std]
#![warn(missing_docs)]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod graph;
pub mod node;

pub use graph::{Graph, GraphError, NodeId, Spread};
pub use node::{BracketNode, Node};
