# nostos で初めての Lean

`Outcome::map_done` の性質を、Lean の小さなモデルで証明する実験。
対象の Rust は [`outcome.rs`](../../crates/nostos-core/src/outcome.rs)、
実験の範囲は [ADR-0012](../../docs/adr/0012-lean-outcome-spike.md) を参照。

## 実行する

macOS では Homebrew から Lean のバージョン管理ツール elan を導入する。

```bash
brew install --formula elan-init
```

リポジトリのルートから実行する。

```bash
cd formal/lean
lean --version
lean -DwarningAsError=true Outcome.lean
```

`lean-toolchain` が Lean 4.36.0 を指定している。初回は elan がそのバージョンを
ダウンロードする。グローバルのデフォルトバージョン指定は不要。
外部ライブラリや Lake のビルドプロジェクトを必要としない、1 ファイルの実験。

成功時は終了コード 0 になり、四つの定理について
`does not depend on any axioms` と表示される。
`warningAsError=true` により、証明の穴を残す `sorry` も警告で済ませず失敗させる。

## テストと何が違うのか

既存の Rust テストは、たとえば `Done(1)` に `+10` すると `Done(11)` になることを確認する。
今回の証明は、型や具体的な値を決めずに、次の命題を扱う。

| Lean の定理 | 言っていること |
|---|---|
| `mapDone_id` | 何も変えない変換なら、元の Outcome と等しい |
| `mapDone_comp` | f の後に g を適用しても、合成した関数を一度適用しても等しい |
| `mapDone_reborn` | Reborn の中身は変わらない |
| `mapDone_failed` | Failed の中身は変わらない |

たくさんの値を試す代わりに、任意の値について通用する理由を Lean がチェックする。

## 最初の証明を読む

```lean
theorem mapDone_id (x : Outcome O I E) : mapDone id x = x := by
  cases x <;> rfl
```

- `theorem`: これから証明する命題に名前を付ける。
- `(x : Outcome O I E)`: 任意の Outcome を x とする。O / I / E も特定しない。
- `mapDone id x = x`: 何も変えない関数 `id` を適用しても元と等しい。
- `by`: ここから証明を書く。
- `cases x`: Done / Reborn / Failed の三つに場合分けする。
- `<;> rfl`: 三つとも、定義を計算すれば左右が同じになることを確かめる。

Done の場合は `id o = o`。残り二つは `mapDone` が中身をそのまま返す。
これで全ケースを覆っているため、値を列挙する必要がない。

## わざと間違えてみる

`formal/lean/` で次を実行すると、元のファイルを変更せずに誤った命題を試せる。

```bash
proof_trial=$(mktemp -d)
cat Outcome.lean > "$proof_trial/FalseIdentity.lean"
cat >> "$proof_trial/FalseIdentity.lean" <<'LEAN'

example (x : Nostos.Outcome Nat Nat Nat) :
    Nostos.mapDone (fun n => n + 1) x = x := by
  cases x <;> rfl
LEAN
lean -DwarningAsError=true "$proof_trial/FalseIdentity.lean"
rm "$proof_trial/FalseIdentity.lean"
rmdir "$proof_trial"
```

Lean のコマンドは終了コード 1、`case done` で `Tactic rfl failed` になる。
`Done(n + 1)` と `Done(n)` は同じではない。たとえば `Done(0)` が反例。
残り二つの相では中身が変わらないため、その分岐の証明は通る。

証明を間違えた時にも失敗するので、「成功表示が出るだけ」の体験では終わらない。
なお、一般には一つの証明手順の失敗だけで命題が偽と決まるわけではない。
今回は具体的な反例もある。

## 今回の保証の範囲

証明したのは **Lean に手で写したモデル上の性質**。
Rust ソースとモデルが一致することは証明していない。
Lean の関数は純粋で停止するものとして扱い、Rust の `FnOnce` の副作用・panic・非停止・
所有権や Drop の挙動はモデルに含めていない。
Rust のテストは引き続き必要。

次の題材には、`drive_bounded` の「n 回進めて中断・m 回再開する」と
「n + m 回まとめて進める」が一致する性質が考えられる。
その際は `FnMut` の内部状態をどう表すかも決める必要がある。

公式入門: [Theorem Proving in Lean 4](https://lean-lang.org/theorem_proving_in_lean4/)、
[場合分けと帰納的な型](https://lean-lang.org/theorem_proving_in_lean4/Inductive-Types/)。
