//! # nostos-graph
//!
//! `nostos` primitive を node とする有向グラフ substrate (Axis D)。
//!
//! - **node** = [`Node`] ── [`nostos::Bracket`] の `enter`+`exit` を `drive` 1 手に畳んだ
//!   object-safe な層。 graph は heterogeneous な `Box<dyn Node<V>>` を扱う
//! - **edge** = [`nostos::Voyage`] routing ── `Done` は下流へ、 `Reborn` は self-loop、
//!   `Failed` は error edge
//! - [`Graph`] 自身が [`Node`] を実装する ── graph が graph に入れ子する (ADR-0005 D6)
//!
//! 詳細設計は [ADR-0005] (framing) と [ADR-0007] (具体)。
//!
//! [ADR-0005]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0005-graph-substrate.md
//! [ADR-0007]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0007-graph-design.md
//!
//! ## Status
//!
//! RoundTrip routing (`Done` / `Reborn` / `Failed`) を実装済み。 `OneWay` 由来の
//! 拡散 (`Spread`) は未実装 ── `Voyage::OneWay` が bare (payload 無し) で fan-out が
//! `Node::drive` の値要求と噛み合わないため、 後続の設計を待つ。 現状 `OneWay` は
//! node の終端 (sink) として扱う。

#![no_std]
#![warn(missing_docs)]

extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod graph;
pub mod node;

pub use graph::{Graph, GraphError, NodeId};
pub use node::{BracketNode, Node};
