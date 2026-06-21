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
//! ## Status — `v0.2.0`
//!
//! [`Outcome`] ADT ([ADR-0002])、 loop driver [`drive`] / [`drive_bounded`] ([ADR-0003])、
//! [`Bracket`] / [`Driver`] trait ([ADR-0004])、 [`Voyage`] 頂点型 ([ADR-0006]) を実装済み。
//!
//! `v0.2.0` で async 版を additive に追加 ([ADR-0011])： [`AsyncBracket`] / [`AsyncDriver`]
//! (AFIT)、 [`drive_async`] / [`drive_bounded_async`] (`AsyncFnMut` step)。 sync API は不変。
//!
//! [ADR-0002]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0002-outcome-adt.md
//! [ADR-0003]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0003-lifecycle-loop-dual.md
//! [ADR-0004]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0004-bracket-and-driver.md
//! [ADR-0006]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0006-voyage.md
//! [ADR-0011]: https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0011-async-bracket-and-drive.md

#![no_std]
#![warn(missing_docs)]
#![warn(rust_2024_compatibility)]

#[cfg(test)]
extern crate std;

pub mod async_bracket;
pub mod async_driver;
pub mod bracket;
pub mod drive;
pub mod driver;
pub mod outcome;
pub mod voyage;

pub use async_bracket::AsyncBracket;
pub use async_driver::AsyncDriver;
pub use bracket::Bracket;
pub use drive::{drive, drive_async, drive_bounded, drive_bounded_async};
pub use driver::Driver;
pub use outcome::{Cycle, Outcome};
pub use voyage::Voyage;

/// 依存ゼロの最小 `block_on` ── test 専用 (1.85 stable の [`core::task::Waker::noop`] を使う)。
///
/// test future は実 I/O を持たず即時 `Ready` になるため、 単純な poll ループで足りる。
#[cfg(test)]
pub(crate) fn block_on<F: core::future::Future>(fut: F) -> F::Output {
    use core::pin::pin;
    use core::task::{Context, Poll};

    let waker = core::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    let mut fut = pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}
