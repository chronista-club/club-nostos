# ADR-0004 — Bracket trait と Driver trait

> **Status**: `accepted` (2026-05-17、 review 完了)
> **Date**: 2026-05-17
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0001](0001-bracket-and-outcome.md) Axis A
> **Builds on**: [ADR-0002](0002-outcome-adt.md) (Outcome ADT) / [ADR-0003](0003-lifecycle-loop-dual.md) (loop driver)
> **Consumer input**: creoui `lifecycle-spine.md` (`chronista-club/creo-ui`)、 handoff `mem_1Cb7aCKjHmWhusyejBf2Vt` F-2

---

## Context

[ADR-0001](0001-bracket-and-outcome.md) Axis A は Bracket trait の signature を問うた。 ADR-0002 (Outcome ADT) と ADR-0003 (loop driver) が landing し、 nostos の最初の本物の consumer ── **creoui Editor Mode** ── から `lifecycle-spine.md` という形で実需 feedback が返ってきた。

本 ADR は Axis A を decide する。 creoui F-2 (「`Driver` を first-class trait に」) を取り込み、 **Bracket trait** と **Driver trait** を同時に設計する。 これにより ADR-0001 の 3 axes (B=Outcome / C=loop / A=Bracket) が 1 つの substrate に閉じる。

### creoui からの主要 input

- **Bracket は trait** ── 普遍的な lifecycle 形。 instance は `enter` のたび生成される (creoui の `EditSession`)。 型は 1 つ (D-1)
- **`Active` は owned で足りる** ── creoui の `EditSession` は owned 型。 GAT による借用 `Active` は不要 (lifecycle-spine §1)
- **非侵襲性 = RAII 純粋性** ── Bracket は Resource を borrow only、 所有・変更しない (§6)
- **`Driver` を first-class trait に** ── Human / AI が 「同じことをしている、 違うのは intent の発行源だけ」。 lifecycle = 単発 Driver、 loop = 反復 Driver、 substrate は同一 Bracket (§4 / F-2)

## Decision

### D1 — `Bracket` trait (関連型 5 つ)

```rust
pub trait Bracket {
    /// enter に渡す入力。
    type Input;
    /// enter ～ exit の間の作業状態 (= Active 相)。
    type Active;
    /// exit が `Done` で返す成果。
    type Done;
    /// exit が `Reborn` で返す次回入力。
    type Reborn;
    /// exit が `Failed` で返す終端理由。
    type Failed;

    /// bracket を開き、 Active 相に入る。
    fn enter(&self, input: Self::Input) -> Self::Active;

    /// bracket を閉じ、 Outcome へ収束する。
    fn exit(&self, active: Self::Active) -> Outcome<Self::Done, Self::Reborn, Self::Failed>;
}
```

- 関連型は **5 つ展開**で確定 (review Q4)。 `Done` / `Reborn` / `Failed` は `exit` が返す `Outcome<O,I,E>` (ADR-0002) の 3 パラメータに 1:1 対応 ── Bracket は ADR-0002 の ADT に**直接配線**される。 opaque な `type Outcome` にまとめると配線が間接になるため採らない。
- creoui の `Resource` (例: `ContentSession`) は trait のパラメータにしない ── Resource は実装型の内部事情で、 `Active` がそれを包む。 `Bracket<ContentSession>` という creoui 記法は notation。
- Active 相の内部進行 (creoui の jo/ha/kyu) は **trait に露出しない** ── それは consumer の `Active` 型固有。 trait から見た 1 cycle は `enter → (Active 不透明) → exit → Outcome`。

### D2 — `Active` は owned (非 GAT) で確定

`Active` は GAT (`type Active<'a>`) にせず **owned 関連型**とする (review Q1)。

ADR-0001 Axis A の暫定傾きは 「GAT 採択」 だったが、 実 consumer (creoui) の `Active = EditSession` は owned 型であり、 借用 `Active` を要する場面が無い。 GAT は API 学習コスト・実装複雑性が高い (ADR-0001 Negative)。 owned で足りる証拠が consumer から出た以上、 non-GAT を採る。

> 借用 `Active` を要する consumer が将来現れたら、 その時 GAT 化を別 ADR で検討する。 owned → GAT は後方非互換だが、 founding 段階で over-engineering しない (CLAUDE.md 「Minimum を保つ」) を優先。

### D3 — `enter` / `exit` は `&self` の trait method で確定

- `enter(&self, ...)` / `exit(&self, ...)` ── `&self` を取る (review Q3)。 Bracket 実装型は lifecycle 定義 (設定を持ちうる) であり、 **1 つの Bracket を繰り返し `enter` できる**。 ADR-0001 暫定の 「`self` consume」 から変更 ── creoui は同じ Bracket 型を `enter` のたび使う (D-1) ため consume は不適。
- `exit` は `Active` を **consume** する (`Self::Active` を値で取る) ── Active 相はそこで終わり、 Outcome に収束する。
- 両者とも free function でなく trait method ── Bracket 実装が enter/exit のロジックを定義する。

### D4 — RAII 純粋性を doc 契約として明記

creoui §6 の発見 ── 非侵襲性の複数の不変条件が 「Bracket は Resource を borrow only」 の 1 原理から導出される ── を、 `Bracket` trait の doc 契約として明記する:

> Bracket は `Active` が包む Resource を **borrow する**。 所有・変更・移動しない。 `Failed` で終端しても Resource は `enter` 前と同一に戻る (bracket 内の delta だけが捨てられる)。

これは型で強制できる制約ではない (Resource は `Active` 内部にあり trait level で借用検査が効かない) が、 実装者への規律として doc に置く。

### D5 — `Driver` trait (駆動主体として 1 trait)

`Driver` は **Bracket を駆動する主体**を、 1 つの first-class trait として表す (review Q2)。

```rust
pub trait Driver<B: Bracket> {
    /// `Reborn` を受けて次サイクルの入力を決める。
    ///
    /// - `Ok(input)` ── 継続。 `input` で次の `enter` を呼ぶ
    /// - `Err(reborn)` ── 打ち切り。 受け取った `Reborn` をそのまま手放す
    ///
    /// Human / AI の差はここに宿る ── `HumanDriver` は人の入力を待ち、
    /// `AgentDriver` は次の値を計算する。 単発か反復かも、
    /// `next` が `Ok` を返し続けるか否かで決まる。
    fn next(&mut self, reborn: B::Reborn) -> Result<B::Input, B::Reborn>;

    /// bracket を初期入力から駆動し、 終端結果を返す。 (provided method)
    ///
    /// `Done` / `Failed` で即終端。 `Reborn` のたび `next` に諮り、
    /// `Ok` なら次サイクル、 `Err` なら `Reborn` を返して終わる。
    fn run(
        &mut self,
        bracket: &B,
        initial: B::Input,
    ) -> Outcome<B::Done, B::Reborn, B::Failed> {
        let mut input = initial;
        loop {
            let active = bracket.enter(input);
            match bracket.exit(active) {
                Outcome::Done(o) => return Outcome::Done(o),
                Outcome::Failed(e) => return Outcome::Failed(e),
                Outcome::Reborn(r) => match self.next(r) {
                    Ok(next) => input = next,
                    Err(reborn) => return Outcome::Reborn(reborn),
                },
            }
        }
    }
}
```

設計の核:

- **要求メソッドは `next` 1 つ** ── 「`Reborn` を受けて次の `Input` を決める」。 Human/AI の駆動の違いも、 lifecycle (単発) ↔ loop (反復) の違いも、 すべて `next` の実装に還元される。 単発 Driver は `next` が常に `Err` を返す ── `run` はちょうど 1 cycle 回る。
- **`run` は provided method** ── `enter → exit → (Reborn なら next) → loop` を既定実装。 Driver 実装は `next` だけ書けばよい。 戻り値は `Outcome<..>` (三相のまま) ── loop Driver は `Reborn` を返さず、 単発 Driver や打ち切りは `Reborn` を返す。 ADR-0003 `drive_bounded` の戻り値慣習と一致。
- creoui の jo/ha/kyu intent 発行は `Active` 型固有のため `Driver` trait には現れない ── nostos-core の `Driver` は **cycle 粒度** (`enter→exit` 1 周) で駆動し、 「次の intent」 を 「次の `Input`」 として抽象化する。

### D6 — ADR-0003 free function との関係

ADR-0003 の `drive` / `drive_bounded` は **closure ベースの便利形**として残す。 両者の住み分け:

| | 形 | 駆動対象 | 用途 |
|--|-----|---------|------|
| `drive` / `drive_bounded` | free function | `FnMut(I) -> Outcome` (closure step) | 手軽な loop、 Bracket を介さない step |
| `Driver::run` | trait method | `Bracket` 実装 | 駆動戦略 (Human/AI) を差し替える構造的な形 |

`Bracket` + `Driver` の組が ADR-0003 が保留した 「step 抽象の最終形」。 closure 版は廃止しない ── 軽い用途の入口として有用。

### D7 — ファイル構成

```
crates/nostos-core/src/
├── lib.rs        # pub use bracket::Bracket / driver::Driver
├── outcome.rs    # ADR-0002
├── drive.rs      # ADR-0003 (drive / drive_bounded)
├── bracket.rs    # Bracket trait
└── driver.rs     # Driver trait (next 要求 + run provided)
```

## Consequences

### Positive

- Bracket が ADR-0002 の `Outcome` に直接配線され、 ADR-0001 の 3 axes が 1 つの substrate に閉じる
- non-GAT 採択で API 学習コストを抑え、 founding 段階の minimum を保つ
- `Driver` の要求メソッドが `next` 1 つ ── 実装が軽く、 Human/AI/単発/反復の全差異がそこに局所化される。 human 編集と AI 編集が構造的に分岐しない (creoui の主目的)

### Negative

- 関連型 5 つは `impl Bracket` の記述量が多い ── type alias や derive macro で将来緩和の余地 (本 ADR scope 外)
- non-GAT は借用 `Active` を要する consumer 出現時に後方非互換の GAT 化が要る ── その判断は先送り
- `Driver::run` の既定実装は上限なし loop ── 無限 loop 防止は `next` 実装の責任。 上限つき変種は実装時に `drive_bounded` 相当を検討

### Neutral

- Axis D (graph 同型) / Axis E (CGP) は本 ADR では扱わない ── creoui §5 が 4 方向 layout = graph topology の早期 signal を出しているが、 ADR-0001 の通り別 crate・後続

## Resolved Questions

draft 段階の Open Questions は review (2026-05-17) で以下に確定:

1. **GAT** → non-GAT (owned `Active`) で確定。 借用 `Active` consumer 出現を GAT 化トリガとする (D2)
2. **`Driver` trait の位置** → 駆動主体として **1 trait**。 要求メソッド `next` + provided `run` (D5)
3. **`enter` / `exit` の self** → `&self` で確定 (D3)
4. **関連型** → 5 つ展開のまま (D1)

---

> 本 ADR は `accepted`。 次はテストリスト作成 (t-wada 流) → Bracket / Driver 実装。 creoui handoff (`mem_1Cb7aCKjHmWhusyejBf2Vt`) は実装 landing 時に close する。
