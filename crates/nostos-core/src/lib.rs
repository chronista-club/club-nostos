//! # nostos
//!
//! **nostos** ── ギリシャ語 νόστος、 「変容を経た帰還」。 『オデュッセイア』 中核概念。
//!
//! > *"Climb the ladder of abstraction, then come back down with new eyes."*
//!
//! ## Scope
//!
//! `nostos` は、 状態遷移の **bracket** (enter / active / exit) と、
//! 帰還の三相 (`Done` / `Reborn` / `Failed`) を表現する **Outcome ADT** を、
//! Rust の trait と型として提供する primitive ライブラリです。
//!
//! lifecycle ↔ loop の dual view、 node graph editor との同型 substrate、
//! CGP-style component composition を将来的に視野に入れます。
//!
//! ## Status — `v0.1.0`
//!
//! [`Outcome`] ADT ([ADR-0002]) と loop driver [`drive`] / [`drive_bounded`] ([ADR-0003])
//! を実装済み。 `Bracket` trait (ADR-0001 Axis A) は後続。
//!
//! [ADR-0002]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0002-outcome-adt.md
//! [ADR-0003]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0003-lifecycle-loop-dual.md

#![no_std]
#![warn(missing_docs)]
#![warn(rust_2024_compatibility)]

#[cfg(test)]
extern crate std;

pub mod drive;
pub mod outcome;

pub use drive::{drive, drive_bounded};
pub use outcome::{Cycle, Outcome};
