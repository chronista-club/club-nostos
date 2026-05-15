//! # nostos
//!
//! **nostos** ── ギリシャ語 νόστος、 「変容を経た帰還」。 『オデュッセイア』 中核概念。
//!
//! > *"Climb the ladder of abstraction, then come back down with new eyes."*
//!
//! ## Scope
//!
//! `nostos` は、 状態遷移の **bracket** (enter / active / exit) と、
//! 帰還の三相 (`Done` / `Reborn` / `Fail`) を表現する **Outcome ADT** を、
//! Rust の trait と型として提供する primitive ライブラリです。
//!
//! lifecycle ↔ loop の dual view、 node graph editor との同型 substrate、
//! CGP-style component composition を将来的に視野に入れます。
//!
//! ## Status — `v0.1.0` (founding scaffold)
//!
//! trait 設計は ADR-0001 ([`docs/adr/0001-bracket-and-outcome.md`]) 確定後に着手します。
//! 現時点では module stub のみで、 実装 commit は ADR-0001 review 後に分けます。
//!
//! [`docs/adr/0001-bracket-and-outcome.md`]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0001-bracket-and-outcome.md

#![warn(missing_docs)]
#![warn(rust_2024_compatibility)]

// Module stubs — implementations gated on ADR-0001 review.
//
// pub mod bracket;
// pub mod outcome;
