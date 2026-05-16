//! Outcome ADT ── 帰還 (νόστος) の三相。
//!
//! 詳細設計は [ADR-0002](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0002-outcome-adt.md)。

/// 帰還の三相を表す代数的データ型。
///
/// bracket の `exit` が返す型であり、 nostos の意味論的 core。
/// `Result` の二相 (`Ok` / `Err`) に対し、 Outcome は **三相** ── 完了・再生・失敗 ── を持つ。
///
/// - `Done(O)` ── 旅が完了し、 成果 `O` を持ち帰った
/// - `Reborn(I)` ── 変容を経て次の生へ。 次回入力 `I` を伴う
/// - `Failed(E)` ── 失敗。 エラー `E` を伴う
///
/// # Examples
///
/// ```
/// use nostos::Outcome;
///
/// let o: Outcome<i32, i32, &str> = Outcome::Done(42);
/// assert!(o.is_done());
/// assert_eq!(o.done(), Some(42));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome<O, I, E> {
    /// 旅が完了し、 成果を持ち帰った。
    Done(O),
    /// 変容を経て次の生へ ── 次回入力を伴う。
    Reborn(I),
    /// 失敗。
    Failed(E),
}

/// 成果と次回入力が同型の [`Outcome`] (= loop で頻出する形)。
///
/// 帰還の成果がそのまま次回の入力になる場合に使う。
///
/// ```
/// use nostos::{Cycle, Outcome};
///
/// let c: Cycle<i32, &str> = Outcome::Reborn(7);
/// assert_eq!(c.reborn(), Some(7));
/// ```
pub type Cycle<T, E> = Outcome<T, T, E>;

impl<O, I, E> Outcome<O, I, E> {
    /// `Done` なら `true`。
    pub fn is_done(&self) -> bool {
        matches!(self, Outcome::Done(_))
    }

    /// `Reborn` なら `true`。
    pub fn is_reborn(&self) -> bool {
        matches!(self, Outcome::Reborn(_))
    }

    /// `Failed` なら `true`。
    pub fn is_failed(&self) -> bool {
        matches!(self, Outcome::Failed(_))
    }

    /// `Done` の成果を取り出す。 それ以外は `None`。
    pub fn done(self) -> Option<O> {
        match self {
            Outcome::Done(o) => Some(o),
            _ => None,
        }
    }

    /// `Reborn` の次回入力を取り出す。 それ以外は `None`。
    pub fn reborn(self) -> Option<I> {
        match self {
            Outcome::Reborn(i) => Some(i),
            _ => None,
        }
    }

    /// `Failed` のエラーを取り出す。 それ以外は `None`。
    pub fn failed(self) -> Option<E> {
        match self {
            Outcome::Failed(e) => Some(e),
            _ => None,
        }
    }

    /// `Done` の成果に `f` を適用する。 `Reborn` / `Failed` はそのまま。
    pub fn map_done<O2, F>(self, f: F) -> Outcome<O2, I, E>
    where
        F: FnOnce(O) -> O2,
    {
        match self {
            Outcome::Done(o) => Outcome::Done(f(o)),
            Outcome::Reborn(i) => Outcome::Reborn(i),
            Outcome::Failed(e) => Outcome::Failed(e),
        }
    }

    /// `Reborn` の次回入力に `f` を適用する。 `Done` / `Failed` はそのまま。
    pub fn map_reborn<I2, F>(self, f: F) -> Outcome<O, I2, E>
    where
        F: FnOnce(I) -> I2,
    {
        match self {
            Outcome::Done(o) => Outcome::Done(o),
            Outcome::Reborn(i) => Outcome::Reborn(f(i)),
            Outcome::Failed(e) => Outcome::Failed(e),
        }
    }

    /// `Failed` のエラーに `f` を適用する。 `Done` / `Reborn` はそのまま。
    pub fn map_failed<E2, F>(self, f: F) -> Outcome<O, I, E2>
    where
        F: FnOnce(E) -> E2,
    {
        match self {
            Outcome::Done(o) => Outcome::Done(o),
            Outcome::Reborn(i) => Outcome::Reborn(i),
            Outcome::Failed(e) => Outcome::Failed(f(e)),
        }
    }
}

impl<O, I, E> From<Result<O, E>> for Outcome<O, I, E> {
    /// `Ok(o)` → `Done(o)`、 `Err(e)` → `Failed(e)`。
    ///
    /// `Result` には `Reborn` 相当の variant が無いため、 この変換は `Reborn` を生成しない。
    fn from(result: Result<O, E>) -> Self {
        match result {
            Ok(o) => Outcome::Done(o),
            Err(e) => Outcome::Failed(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- variant 構築 ---

    #[test]
    fn constructs_each_variant() {
        let _: Outcome<i32, i32, &str> = Outcome::Done(1);
        let _: Outcome<i32, i32, &str> = Outcome::Reborn(2);
        let _: Outcome<i32, i32, &str> = Outcome::Failed("e");
    }

    // --- 述語 ---

    #[test]
    fn predicates_done() {
        let o: Outcome<i32, i32, &str> = Outcome::Done(1);
        assert!(o.is_done());
        assert!(!o.is_reborn());
        assert!(!o.is_failed());
    }

    #[test]
    fn predicates_reborn() {
        let o: Outcome<i32, i32, &str> = Outcome::Reborn(2);
        assert!(!o.is_done());
        assert!(o.is_reborn());
        assert!(!o.is_failed());
    }

    #[test]
    fn predicates_failed() {
        let o: Outcome<i32, i32, &str> = Outcome::Failed("e");
        assert!(!o.is_done());
        assert!(!o.is_reborn());
        assert!(o.is_failed());
    }

    // --- 取り出し (3×3) ---

    #[test]
    fn extract_from_done() {
        let o: Outcome<i32, i32, &str> = Outcome::Done(1);
        assert_eq!(o.done(), Some(1));
        assert_eq!(o.reborn(), None);
        assert_eq!(o.failed(), None);
    }

    #[test]
    fn extract_from_reborn() {
        let o: Outcome<i32, i32, &str> = Outcome::Reborn(2);
        assert_eq!(o.done(), None);
        assert_eq!(o.reborn(), Some(2));
        assert_eq!(o.failed(), None);
    }

    #[test]
    fn extract_from_failed() {
        let o: Outcome<i32, i32, &str> = Outcome::Failed("e");
        assert_eq!(o.done(), None);
        assert_eq!(o.reborn(), None);
        assert_eq!(o.failed(), Some("e"));
    }

    // --- map (該当 variant のみ変換、 非該当はそのまま) ---

    #[test]
    fn map_done_transforms_only_done() {
        let done: Outcome<i32, i32, &str> = Outcome::Done(1);
        assert_eq!(done.map_done(|o| o + 10), Outcome::Done(11));

        let reborn: Outcome<i32, i32, &str> = Outcome::Reborn(2);
        assert_eq!(reborn.map_done(|o| o + 10), Outcome::Reborn(2));

        let failed: Outcome<i32, i32, &str> = Outcome::Failed("e");
        assert_eq!(failed.map_done(|o| o + 10), Outcome::Failed("e"));
    }

    #[test]
    fn map_reborn_transforms_only_reborn() {
        let done: Outcome<i32, i32, &str> = Outcome::Done(1);
        assert_eq!(done.map_reborn(|i| i + 10), Outcome::Done(1));

        let reborn: Outcome<i32, i32, &str> = Outcome::Reborn(2);
        assert_eq!(reborn.map_reborn(|i| i + 10), Outcome::Reborn(12));

        let failed: Outcome<i32, i32, &str> = Outcome::Failed("e");
        assert_eq!(failed.map_reborn(|i| i + 10), Outcome::Failed("e"));
    }

    #[test]
    fn map_failed_transforms_only_failed() {
        let done: Outcome<i32, i32, &str> = Outcome::Done(1);
        assert_eq!(done.map_failed(|e: &str| e.len()), Outcome::Done(1));

        let reborn: Outcome<i32, i32, &str> = Outcome::Reborn(2);
        assert_eq!(reborn.map_failed(|e: &str| e.len()), Outcome::Reborn(2));

        let failed: Outcome<i32, i32, &str> = Outcome::Failed("err");
        assert_eq!(failed.map_failed(|e: &str| e.len()), Outcome::Failed(3));
    }

    #[test]
    fn map_can_change_type() {
        let done: Outcome<i32, i32, &str> = Outcome::Done(1);
        let mapped: Outcome<bool, i32, &str> = done.map_done(|o| o == 1);
        assert_eq!(mapped, Outcome::Done(true));
    }

    // --- From<Result> ---

    #[test]
    fn from_ok_becomes_done() {
        let o: Outcome<i32, i32, &str> = Ok(1).into();
        assert_eq!(o, Outcome::Done(1));
    }

    #[test]
    fn from_err_becomes_failed() {
        let o: Outcome<i32, i32, &str> = Err("e").into();
        assert_eq!(o, Outcome::Failed("e"));
    }

    // --- Cycle alias ---

    #[test]
    fn cycle_alias_is_outcome_with_same_o_and_i() {
        let c: Cycle<i32, &str> = Outcome::Reborn(7);
        assert_eq!(c.reborn(), Some(7));
        let _: Outcome<i32, i32, &str> = c;
    }

    // --- derive ---

    #[test]
    fn clone_and_copy() {
        let a: Outcome<i32, i32, &str> = Outcome::Done(1);
        let b = a;
        let c = a;
        assert_eq!(b, c);
        assert_eq!(a.clone(), a);
    }

    #[test]
    fn eq_compares_variant_and_value() {
        let a: Outcome<i32, i32, &str> = Outcome::Done(1);
        assert_eq!(a, Outcome::Done(1));
        assert_ne!(a, Outcome::Done(2));
        assert_ne!(a, Outcome::Reborn(1));
    }

    #[test]
    fn debug_is_derived() {
        let s = std::format!("{:?}", Outcome::<i32, i32, &str>::Done(1));
        assert_eq!(s, "Done(1)");
    }

    #[test]
    fn hash_usable_as_map_key() {
        let mut map = std::collections::HashMap::new();
        map.insert(Outcome::<i32, i32, &str>::Done(1), "a");
        map.insert(Outcome::<i32, i32, &str>::Reborn(2), "b");
        assert_eq!(map.get(&Outcome::Done(1)), Some(&"a"));
        assert_eq!(map.get(&Outcome::Reborn(2)), Some(&"b"));
    }
}
