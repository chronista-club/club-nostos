import Nostos

/-
  Nostos.lean は実際の Rust から生成された定義。
  このファイルには期待する仕様と、それに一致することの証明だけを書く。
-/
namespace NostosProof

open NostosGenerated.outcome

-- 仕様: Done のときだけ true。
def expectedIsDone {O I E : Type} : Outcome O I E → Bool
  | .Done _ => true
  | .Reborn _ => false
  | .Failed _ => false

-- 任意の型・値について、実装は正常終了し、仕様どおりの Bool を返す。
theorem is_done_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.is_done x = Aeneas.Std.Result.ok (expectedIsDone x) := by
  cases x <;> rfl

#print axioms is_done_matches_spec

def expectedIsReborn {O I E : Type} : Outcome O I E → Bool
  | .Done _ => false
  | .Reborn _ => true
  | .Failed _ => false

theorem is_reborn_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.is_reborn x = Aeneas.Std.Result.ok (expectedIsReborn x) := by
  cases x <;> rfl

#print axioms is_reborn_matches_spec

def expectedIsFailed {O I E : Type} : Outcome O I E → Bool
  | .Done _ => false
  | .Reborn _ => false
  | .Failed _ => true

theorem is_failed_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.is_failed x = Aeneas.Std.Result.ok (expectedIsFailed x) := by
  cases x <;> rfl

#print axioms is_failed_matches_spec

-- 取り出し: 該当する variant の中身をそのまま Some で返し、それ以外は None。
def expectedDone {O I E : Type} : Outcome O I E → Option O
  | .Done o => some o
  | .Reborn _ => none
  | .Failed _ => none

theorem done_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.done x = Aeneas.Std.Result.ok (expectedDone x) := by
  cases x <;> rfl

#print axioms done_matches_spec

def expectedReborn {O I E : Type} : Outcome O I E → Option I
  | .Done _ => none
  | .Reborn i => some i
  | .Failed _ => none

theorem reborn_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.reborn x = Aeneas.Std.Result.ok (expectedReborn x) := by
  cases x <;> rfl

#print axioms reborn_matches_spec

def expectedFailed {O I E : Type} : Outcome O I E → Option E
  | .Done _ => none
  | .Reborn _ => none
  | .Failed e => some e

theorem failed_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.failed x = Aeneas.Std.Result.ok (expectedFailed x) := by
  cases x <;> rfl

#print axioms failed_matches_spec

end NostosProof
