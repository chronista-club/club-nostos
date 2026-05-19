//! `Voyage` ── 往相と還相を併せ持つ頂点型。
//!
//! 詳細設計は [ADR-0006](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0006-voyage.md)。

use crate::Outcome;

/// 旅の頂点型 ── 往相 (`OneWay`) と還相 (`RoundTrip`) の二相。
///
/// [`Outcome`] (ADR-0002) は 「帰還の三相」 だけを型にしていた。 `Voyage` は
/// 「往ったが還らない」 一方向 (`OneWay`) も第一級で表す ── nostos が自ら引いた
/// 回向 (往相 + 還相) の往相を回収して全体になる型。
///
/// `Outcome` は不可侵 ── `Voyage` は `Outcome` を `RoundTrip` arm に**そのまま内包**
/// する一段上の型であり、 `Outcome` に 4 つ目の variant を足すものではない (昇華であって改造でない)。
///
/// # Examples
///
/// ```
/// use nostos::{Outcome, Voyage};
///
/// // 一方向 ── 発が telos、 還を持たない。
/// let notify: Voyage<(), (), ()> = Voyage::OneWay;
/// assert!(notify.is_one_way());
///
/// // 往復 ── Outcome を内包する。
/// let answered: Voyage<i32, i32, ()> = Outcome::Done(42).into();
/// assert_eq!(answered.round_trip(), Some(Outcome::Done(42)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Voyage<O, I, E> {
    /// 往相 ── 発が telos。 還を持たない一方向 (notification / broadcast 等)。
    OneWay,
    /// 還相 ── 往って還った。 [`Outcome`] 三相を内包する。
    RoundTrip(Outcome<O, I, E>),
}

impl<O, I, E> Voyage<O, I, E> {
    /// `OneWay` なら `true`。
    pub fn is_one_way(&self) -> bool {
        matches!(self, Voyage::OneWay)
    }

    /// `RoundTrip` なら `true`。
    pub fn is_round_trip(&self) -> bool {
        matches!(self, Voyage::RoundTrip(_))
    }

    /// `RoundTrip` の内包する [`Outcome`] を取り出す。 `OneWay` は `None`。
    pub fn round_trip(self) -> Option<Outcome<O, I, E>> {
        match self {
            Voyage::RoundTrip(outcome) => Some(outcome),
            Voyage::OneWay => None,
        }
    }
}

impl<O, I, E> From<Outcome<O, I, E>> for Voyage<O, I, E> {
    /// [`Outcome`] は自明に `RoundTrip` ── 還相は往還の片側。
    fn from(outcome: Outcome<O, I, E>) -> Self {
        Voyage::RoundTrip(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_one_way_and_round_trip() {
        let _: Voyage<i32, i32, &str> = Voyage::OneWay;
        let _: Voyage<i32, i32, &str> = Voyage::RoundTrip(Outcome::Done(1));
    }

    #[test]
    fn predicates_one_way() {
        let v: Voyage<i32, i32, &str> = Voyage::OneWay;
        assert!(v.is_one_way());
        assert!(!v.is_round_trip());
    }

    #[test]
    fn predicates_round_trip() {
        let v: Voyage<i32, i32, &str> = Voyage::RoundTrip(Outcome::Reborn(2));
        assert!(!v.is_one_way());
        assert!(v.is_round_trip());
    }

    #[test]
    fn round_trip_accessor() {
        let rt: Voyage<i32, i32, &str> = Voyage::RoundTrip(Outcome::Done(1));
        assert_eq!(rt.round_trip(), Some(Outcome::Done(1)));

        let ow: Voyage<i32, i32, &str> = Voyage::OneWay;
        assert_eq!(ow.round_trip(), None);
    }

    #[test]
    fn from_outcome_becomes_round_trip() {
        let v: Voyage<i32, i32, &str> = Outcome::Failed("e").into();
        assert_eq!(v, Voyage::RoundTrip(Outcome::Failed("e")));
    }

    #[test]
    fn derives_clone_copy_eq() {
        let a: Voyage<i32, i32, &str> = Voyage::RoundTrip(Outcome::Done(1));
        let b = a;
        assert_eq!(a, b);
        assert_eq!(a.clone(), a);
        assert_ne!(a, Voyage::OneWay);
    }
}
