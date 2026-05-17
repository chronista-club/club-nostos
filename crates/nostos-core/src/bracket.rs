//! `Bracket` trait ── lifecycle primitive (enter / Active / exit)。
//!
//! 詳細設計は [ADR-0004](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0004-bracket-and-driver.md)。

use crate::Outcome;

/// 状態遷移の lifecycle primitive。
///
/// `enter` で Active 相に入り、 `exit` で [`Outcome`] へ収束する。
/// trait から見た 1 cycle は `enter → (Active 相) → exit → Outcome`。
///
/// # RAII 純粋性 (実装者への契約)
///
/// Bracket は `Active` が包む Resource を **borrow する** ── 所有・変更・移動しない。
/// `Failed` で終端しても Resource は `enter` 前と同一に戻る (bracket 内の delta だけが
/// 捨てられる)。 これは型では強制できないため、 実装者の規律として課す。
///
/// # Examples
///
/// ```
/// use nostos::{Bracket, Outcome};
///
/// struct Countdown;
/// impl Bracket for Countdown {
///     type Input = u32;
///     type Active = u32;
///     type Done = ();
///     type Reborn = u32;
///     type Failed = core::convert::Infallible;
///     fn enter(&self, input: u32) -> u32 {
///         input
///     }
///     fn exit(&self, active: u32) -> Outcome<(), u32, core::convert::Infallible> {
///         if active == 0 {
///             Outcome::Done(())
///         } else {
///             Outcome::Reborn(active - 1)
///         }
///     }
/// }
///
/// let countdown = Countdown;
/// assert_eq!(countdown.exit(countdown.enter(0)), Outcome::Done(()));
/// assert_eq!(countdown.exit(countdown.enter(3)), Outcome::Reborn(2));
/// ```
pub trait Bracket {
    /// `enter` に渡す入力。
    type Input;
    /// `enter` ～ `exit` の間の作業状態 (= Active 相)。
    type Active;
    /// `exit` が `Done` で返す成果。
    type Done;
    /// `exit` が `Reborn` で返す次回入力。
    type Reborn;
    /// `exit` が `Failed` で返す終端理由。
    type Failed;

    /// bracket を開き、 Active 相に入る。
    fn enter(&self, input: Self::Input) -> Self::Active;

    /// bracket を閉じ、 [`Outcome`] へ収束する。
    fn exit(&self, active: Self::Active) -> Outcome<Self::Done, Self::Reborn, Self::Failed>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用 Bracket ── floor 以下で `Done`、 超過で `Reborn`、 1000 超で `Failed`。
    struct Countdown {
        floor: u32,
    }

    impl Bracket for Countdown {
        type Input = u32;
        type Active = u32;
        type Done = u32;
        type Reborn = u32;
        type Failed = &'static str;

        fn enter(&self, input: u32) -> u32 {
            input
        }

        fn exit(&self, active: u32) -> Outcome<u32, u32, &'static str> {
            if active > 1_000 {
                Outcome::Failed("overflow")
            } else if active <= self.floor {
                Outcome::Done(active)
            } else {
                Outcome::Reborn(active - 1)
            }
        }
    }

    #[test]
    fn enter_produces_active() {
        let b = Countdown { floor: 0 };
        assert_eq!(b.enter(5), 5);
    }

    #[test]
    fn exit_done_at_floor() {
        let b = Countdown { floor: 2 };
        assert_eq!(b.exit(2), Outcome::Done(2));
    }

    #[test]
    fn exit_reborn_above_floor() {
        let b = Countdown { floor: 0 };
        assert_eq!(b.exit(5), Outcome::Reborn(4));
    }

    #[test]
    fn exit_failed_on_overflow() {
        let b = Countdown { floor: 0 };
        assert_eq!(b.exit(2_000), Outcome::Failed("overflow"));
    }

    #[test]
    fn bracket_can_be_entered_repeatedly() {
        // &self なので 1 つの Bracket を繰り返し enter できる。
        let b = Countdown { floor: 0 };
        assert_eq!(b.exit(b.enter(0)), Outcome::Done(0));
        assert_eq!(b.exit(b.enter(3)), Outcome::Reborn(2));
    }
}
