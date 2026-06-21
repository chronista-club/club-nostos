//! loop driver ── step を `Reborn` が続く限り駆動する。
//!
//! 詳細設計は [ADR-0003](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0003-lifecycle-loop-dual.md)。

use crate::Outcome;

/// step を `Reborn` が続く限り繰り返し、 loop の終端結果を返す。
///
/// `initial` で 1 回目の step を呼び、 `Reborn(i)` が返るたびに `i` で step を再度呼ぶ。
/// `Done` / `Failed` が返った時点で loop を終える。 終端は必ず二相に収束するため、
/// 戻り値は `Result<O, E>` ── `Reborn` は 「終了していない」 ので終端には現れない。
///
/// 上限が無いため、 無限ループの防止は step 側の責任。 上限が要るなら
/// [`drive_bounded`] を使う。
///
/// # Examples
///
/// ```
/// use nostos::{drive, Outcome};
///
/// // 0 → 1 → 2 と Reborn、 3 で Done。
/// let result: Result<i32, ()> = drive(0, |x| {
///     if x < 3 { Outcome::Reborn(x + 1) } else { Outcome::Done(x) }
/// });
/// assert_eq!(result, Ok(3));
/// ```
pub fn drive<O, I, E, F>(initial: I, mut step: F) -> Result<O, E>
where
    F: FnMut(I) -> Outcome<O, I, E>,
{
    let mut input = initial;
    loop {
        match step(input) {
            Outcome::Done(o) => return Ok(o),
            Outcome::Reborn(i) => input = i,
            Outcome::Failed(e) => return Err(e),
        }
    }
}

/// [`drive`] の上限つき変種。 step を最大 `max_steps` 回呼ぶ。
///
/// - `max_steps` 回以内に `Done` / `Failed` が返れば、 その `Outcome` を返す
/// - `max_steps` 回呼んでも `Reborn` が続けば、 最後の `Reborn(i)` をそのまま返す
/// - `max_steps` が `0` なら step を一度も呼ばず `Reborn(initial)` を返す
///
/// 戻り値が `Outcome` (三相のまま) なのが要点 ── `Reborn(i)` は 「上限で中断、
/// `i` から再開可能」 を意味し、 同じ `i` で `drive` / `drive_bounded` を呼べば続行できる。
///
/// # Examples
///
/// ```
/// use nostos::{drive_bounded, Outcome};
///
/// // 2 回で打ち切り、 最後の Reborn(2) が返る。
/// let out: Outcome<i32, i32, ()> = drive_bounded(0, 2, |x| Outcome::Reborn(x + 1));
/// assert_eq!(out, Outcome::Reborn(2));
/// ```
pub fn drive_bounded<O, I, E, F>(initial: I, max_steps: usize, mut step: F) -> Outcome<O, I, E>
where
    F: FnMut(I) -> Outcome<O, I, E>,
{
    let mut input = initial;
    for _ in 0..max_steps {
        match step(input) {
            done @ Outcome::Done(_) => return done,
            Outcome::Reborn(i) => input = i,
            failed @ Outcome::Failed(_) => return failed,
        }
    }
    Outcome::Reborn(input)
}

/// [`drive`] の async 版。 step を `AsyncFnMut` で受け、 各 step を await しながら駆動する。
///
/// 制御フローは [`drive`] と同一 ── `Reborn` が続く限り step を呼び、 `Done` / `Failed`
/// で終端して `Result<O, E>` に収束する。 step が async である点だけが異なる。
///
/// # Examples
///
/// ```
/// use nostos::{drive_async, Outcome};
///
/// # async fn _demo() {
/// // 0 → 1 → 2 と Reborn、 3 で Done。
/// let result: Result<i32, ()> = drive_async(0, |x| async move {
///     if x < 3 { Outcome::Reborn(x + 1) } else { Outcome::Done(x) }
/// }).await;
/// assert_eq!(result, Ok(3));
/// # }
/// ```
pub async fn drive_async<O, I, E, F>(initial: I, mut step: F) -> Result<O, E>
where
    F: AsyncFnMut(I) -> Outcome<O, I, E>,
{
    let mut input = initial;
    loop {
        match step(input).await {
            Outcome::Done(o) => return Ok(o),
            Outcome::Reborn(i) => input = i,
            Outcome::Failed(e) => return Err(e),
        }
    }
}

/// [`drive_bounded`] の async 版。 step を最大 `max_steps` 回 await しながら呼ぶ。
///
/// 戻り値が `Outcome` (三相のまま) なのは [`drive_bounded`] と同じ ── `max_steps` 回で
/// `Reborn` が続けば最後の `Reborn(i)` を返し、 「上限で中断、 `i` から再開可能」 を表す。
///
/// # Examples
///
/// ```
/// use nostos::{drive_bounded_async, Outcome};
///
/// # async fn _demo() {
/// // 2 回で打ち切り、 最後の Reborn(2) が返る。
/// let out: Outcome<i32, i32, ()> =
///     drive_bounded_async(0, 2, |x| async move { Outcome::Reborn(x + 1) }).await;
/// assert_eq!(out, Outcome::Reborn(2));
/// # }
/// ```
pub async fn drive_bounded_async<O, I, E, F>(
    initial: I,
    max_steps: usize,
    mut step: F,
) -> Outcome<O, I, E>
where
    F: AsyncFnMut(I) -> Outcome<O, I, E>,
{
    let mut input = initial;
    for _ in 0..max_steps {
        match step(input).await {
            done @ Outcome::Done(_) => return done,
            Outcome::Reborn(i) => input = i,
            failed @ Outcome::Failed(_) => return failed,
        }
    }
    Outcome::Reborn(input)
}

#[cfg(test)]
mod async_tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn drive_async_done_immediately() {
        let result: Result<i32, ()> = block_on(drive_async(1, |x| async move { Outcome::Done(x) }));
        assert_eq!(result, Ok(1));
    }

    #[test]
    fn drive_async_reborn_then_done() {
        let result: Result<i32, ()> = block_on(drive_async(0, |x| async move {
            if x < 3 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Done(x)
            }
        }));
        assert_eq!(result, Ok(3));
    }

    #[test]
    fn drive_async_reborn_then_failed() {
        let result: Result<i32, &str> = block_on(drive_async(0, |x| async move {
            if x < 2 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Failed("stop")
            }
        }));
        assert_eq!(result, Err("stop"));
    }

    #[test]
    fn drive_bounded_async_done_within_limit() {
        let out: Outcome<i32, i32, ()> = block_on(drive_bounded_async(0, 10, |x| async move {
            if x < 3 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Done(x)
            }
        }));
        assert_eq!(out, Outcome::Done(3));
    }

    #[test]
    fn drive_bounded_async_interrupted_at_limit() {
        let out: Outcome<i32, i32, ()> = block_on(drive_bounded_async(0, 3, |x| async move {
            Outcome::Reborn(x + 1)
        }));
        assert_eq!(out, Outcome::Reborn(3));
    }

    #[test]
    fn drive_bounded_async_zero_steps_returns_initial() {
        let out: Outcome<i32, i32, ()> = block_on(drive_bounded_async(42, 0, |x| async move {
            Outcome::Reborn(x + 1)
        }));
        assert_eq!(out, Outcome::Reborn(42));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    // --- drive (上限なし) ---

    #[test]
    fn drive_done_immediately() {
        let result: Result<i32, ()> = drive(1, Outcome::Done);
        assert_eq!(result, Ok(1));
    }

    #[test]
    fn drive_failed_immediately() {
        let result: Result<(), &str> = drive(1, |_| Outcome::Failed("e"));
        assert_eq!(result, Err("e"));
    }

    #[test]
    fn drive_reborn_then_done() {
        let calls = Cell::new(0);
        let result: Result<i32, ()> = drive(0, |x| {
            calls.set(calls.get() + 1);
            if x < 3 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Done(x)
            }
        });
        assert_eq!(result, Ok(3));
        // 0,1,2 で Reborn、 3 で Done = 4 回。
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn drive_reborn_then_failed() {
        let result: Result<i32, &str> = drive(0, |x| {
            if x < 2 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Failed("stop")
            }
        });
        assert_eq!(result, Err("stop"));
    }

    #[test]
    fn drive_passes_reborn_value_to_next_step() {
        // 各 step に渡る入力を記録し、 Reborn 値が次へ渡ることを確認。
        let result: Result<i32, ()> = drive(10, |x| {
            if x < 13 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Done(x)
            }
        });
        assert_eq!(result, Ok(13));
    }

    // --- drive_bounded (上限あり) ---

    #[test]
    fn drive_bounded_done_within_limit() {
        let out: Outcome<i32, i32, ()> = drive_bounded(0, 10, |x| {
            if x < 3 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Done(x)
            }
        });
        assert_eq!(out, Outcome::Done(3));
    }

    #[test]
    fn drive_bounded_failed_within_limit() {
        let out: Outcome<i32, i32, &str> = drive_bounded(0, 10, |x| {
            if x < 2 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Failed("e")
            }
        });
        assert_eq!(out, Outcome::Failed("e"));
    }

    #[test]
    fn drive_bounded_interrupted_at_limit() {
        // 常に Reborn ── 3 回で打ち切り、 最後の Reborn(3)。
        let out: Outcome<i32, i32, ()> = drive_bounded(0, 3, |x| Outcome::Reborn(x + 1));
        assert_eq!(out, Outcome::Reborn(3));
    }

    #[test]
    fn drive_bounded_done_exactly_at_limit() {
        // ちょうど max_steps 回目 (3 回目) で Done。
        let out: Outcome<i32, i32, ()> = drive_bounded(0, 3, |x| {
            if x < 2 {
                Outcome::Reborn(x + 1)
            } else {
                Outcome::Done(x)
            }
        });
        assert_eq!(out, Outcome::Done(2));
    }

    #[test]
    fn drive_bounded_zero_steps_returns_initial() {
        let calls = Cell::new(0);
        let out: Outcome<i32, i32, ()> = drive_bounded(42, 0, |x| {
            calls.set(calls.get() + 1);
            Outcome::Reborn(x + 1)
        });
        assert_eq!(out, Outcome::Reborn(42));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn drive_bounded_never_exceeds_max_steps() {
        let calls = Cell::new(0);
        let _: Outcome<i32, i32, ()> = drive_bounded(0, 5, |x| {
            calls.set(calls.get() + 1);
            Outcome::Reborn(x + 1)
        });
        assert_eq!(calls.get(), 5);
    }
}
