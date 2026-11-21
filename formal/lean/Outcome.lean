/-
  初めての Lean: nostos の Outcome.map_done の値に関する性質。
  対応する Rust: crates/nostos-core/src/outcome.rs
  手書きモデル。Rust 実装そのものの検証ではない。
-/
namespace Nostos

-- Rust の enum に対応する三つのコンストラクタ。
inductive Outcome (O I E : Type) where
  | done : O → Outcome O I E
  | reborn : I → Outcome O I E
  | failed : E → Outcome O I E

variable {O O' O'' I E : Type}

-- Rust の match と同じ三分岐。f は純粋で停止する関数として扱う。
def mapDone (f : O → O') : Outcome O I E → Outcome O' I E
  | .done o => .done (f o)
  | .reborn i => .reborn i
  | .failed e => .failed e

-- 命題1: 中身をそのまま返す関数なら、Outcome 全体も変わらない。
-- x は特定の値ではなく、任意の型 O / I / E の任意の Outcome。
theorem mapDone_id (x : Outcome O I E) : mapDone id x = x := by
  cases x <;> rfl

-- 命題2: f の後に g を適用することと、合成した関数を適用することは等しい。
theorem mapDone_comp (f : O → O') (g : O' → O'') (x : Outcome O I E) :
    mapDone g (mapDone f x) = mapDone (fun o => g (f o)) x := by
  cases x <;> rfl

-- 命題3・4: 対象外の相では、関数にかかわらず中身を保存する。
theorem mapDone_reborn (f : O → O') (i : I) :
    mapDone f (Outcome.reborn i : Outcome O I E) = .reborn i := by
  rfl

theorem mapDone_failed (f : O → O') (e : E) :
    mapDone f (Outcome.failed e : Outcome O I E) = .failed e := by
  rfl

-- 各証明が依存する公理を表示。四つとも空であることを確認する。
#print axioms mapDone_id
#print axioms mapDone_comp
#print axioms mapDone_reborn
#print axioms mapDone_failed

end Nostos
