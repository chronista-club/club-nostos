# ADR-0006 — Voyage: 往相の獲得 (OneWay | RoundTrip)

> **Status**: `accepted` (2026-05-18、 review PR #1 完了)
> **Date**: 2026-05-17
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0001](0001-bracket-and-outcome.md) (5 axes の外に出る新方向) / [ADR-0002](0002-outcome-adt.md) (Outcome ADT)
> **Consumer signal**: VP messaging (`msg` / Mailbox) ── 一方向 (notification / broadcast) と往復 (Q&A) の二相

---

## Context

nostos は README で **還相回向** を、 概念欄で **十牛図 入鄽垂手** を引いている。 だが回向は本来 **往相回向 ＋ 還相回向** ── 「往く相」 と 「還る相」 の対で一つ。

現状の nostos は [`Outcome`](0002-outcome-adt.md) (`Done` / `Reborn` / `Failed`) で **還相だけ** を型にしている。 ADR-0002 自身が 「帰還が完了した三相」 と明言する通り、 Outcome の三相はすべて *還ってきた* 後の相である。 **往相 ── 「往ったが還らない」 ── を表す型が無い。**

最初の本格 consumer の一つ **VP messaging** (`msg` / Mailbox) が、 この欠落を実需として露出させた:

- msg の第一パターンは **一方通行** (タスク指示・進捗報告・通知・broadcast)。 これは **発がそのまま telos** であり、 還を持たない。
- 残りが **往復** (Q&A = request / response)。 これは `Outcome` 三相で表せる。

一方向を `Outcome` に押し込む試み ── 例えば `Done(())` ── は誤りである。 `Done(())` は 「往復して、 空の成果を持ち帰った」 を意味し、 *還の発生を前提* にする。 一方向は還との関係が *端から無い*。 両者は別の相。

→ nostos は **往相を第一級の型として獲得**する必要がある。 これは異物の追加ではなく、 nostos が自ら引いた回向の片肺を回収して全体になる動きである。

## Decision

### D1 — 頂点型 `Voyage<O, I, E>` を導入

```rust
pub enum Voyage<O, I, E> {
    /// 往相 ── 発が telos。 還を持たない一方向。
    OneWay,
    /// 還相 ── 往って還った。 Outcome 三相を内包する。
    RoundTrip(Outcome<O, I, E>),
}
```

`Voyage` は 「発と、 その往還」 を表す。 旅 (voyage) は還るとは限らない ── `OneWay` 側を字義的に裏切らない名として `Voyage` を採る (代替案は Open Questions)。

### D2 — `Outcome` は不可侵。 昇華 = 戴冠であって改造ではない

[ADR-0002](0002-outcome-adt.md) D1 「3 variant で固定」 は **supersede しない**。 `Voyage` は `Outcome` に 4 つ目の variant を足すのではなく、 `Outcome` を `RoundTrip` arm に **そのまま内包**する一段上の型である。

- `Outcome` を 4 variant 化すると、 「帰還の三相」 という ADR-0002 の意味論的 core が壊れる ── `OneWay` は還の相ではないので、 三相の仲間ではない。
- 昇華 (sublimation) の語義通り ── 下層 (`Outcome`) を壊さず、 相が一段上がる。

### D3 — `nostos-core` の `OneWay` は bare (payload 無し)

> ⚠ **部分 supersede** ([ADR-0008](0008-oneway-payload-and-spread.md)、 2026-05-19): nostos-graph 実装が 「bare な `OneWay` は fan-out で値を運べない」 隙間を露出。 ADR-0008 D1 が **`OneWay` に payload を持たせる** (`OneWay(O)`) ── 本 D3 の 「bare」 部分は撤回。 ただし 「`Spread` (拡散) は `nostos-graph` の責務」 という判断は ADR-0008 D3 が引き継ぎ、 有効。

`OneWay` は core では値を持たない。

「往ったものが構造を *さらに伝播* してよいか」 (= 拡散) という問いは存在するが、 それは **node 構造 (topology) を前提**にする問いであり、 単一の旅の lifecycle を扱う `nostos-core` の領分ではない。 `nostos-core` は `no_std` / dependency-free を保つ ([ADR-0005](0005-graph-substrate.md) D3)。

拡散 policy (`Spread` 型) は **`nostos-graph` の責務**とする ── graph crate が OneWay の edge routing として扱う。 [ADR-0005](0005-graph-substrate.md) D5 を参照。

### D4 — `Bracket` は RoundTrip 専用 primitive のまま

[`Bracket`](0004-bracket-and-driver.md) (`enter → Active → exit → Outcome`) は変更しない。 `OneWay` は `exit` の儀式を持たない ── 発のみで、 Active 相も還も無い。 OneWay 形のプロセスは Bracket を介さず `Voyage::OneWay` として直接表される。

Bracket を OneWay-aware にする (例: `exit` が `Voyage` を返す) のは過剰 ── core の Minimum を保つ (CLAUDE.md)。 頂点型 `Voyage` だけが二相を持ち、 `Bracket` は還相の道具に留まる。

## Consequences

### Positive

- nostos が自らの引用 (回向 = 往相 + 還相) に **追いつく** ── 往相獲得は metaphor の完成であって拡張ではない
- `Outcome` 不可侵 ── ADR-0002 の決定・実装・test がすべて無傷
- 一方向 consumer (VP messaging 等) の **force-fit が消える** ── 欠けていた差分がちょうど往相だった。 往相を入れた瞬間、 messaging が力まず nostos に乗る
- 再帰が両軸で表現可能になる下地 ── `Reborn` = 時間の再帰、 `OneWay` の拡散 (nostos-graph) = 空間の再帰

### Negative

- nostos の identity が 「帰還の抽象」 から 「発と、 その往還」 へ広がる ── 名 "nostos" (= νόστος、 還ること) が、 非還の arm (`OneWay`) を含む型の library を指す軽い意味論的ストレッチ
  - 緩和: 還 (`Outcome`) は依然 nostos の心臓・名の由来。 `OneWay` は還を意味あらしめる対 (往無くして還は語れない)。 名は妥当なまま保つ
- core の公開型が 1 つ増える (`Voyage`) ── API surface 微増

### Neutral

- 拡散 (`Spread`) は本 ADR の scope 外 ── nostos-graph / [ADR-0005](0005-graph-substrate.md) が扱う

## Resolved Questions

1. **頂点型の名** → `Voyage` で確定 (review PR #1、 2026-05-18)。 旅は還るとは限らず `OneWay` を字義的に裏切らない。 加えて Odyssey との韻 ── νόστος は voyage の *還る相*であり、 apex 型 `Voyage` が `RoundTrip` arm に `Outcome`(=nostos) を内包する構造が、 原典で 「オデュッセイア (voyage) が nostos (帰還) を内包する」 入れ子と一致する。

## Open Questions (後続)

1. **`OneWay` の将来 payload** ── 現状 bare で確定。 往相が値を持つ必要が将来 core レベルで生じたら、 bare → generic は後方非互換 ── その時点で別 ADR。 founding 段階では over-engineering しない
2. **`Driver` と `Voyage`** ── lifecycle / loop の Driver ([ADR-0004](0004-bracket-and-driver.md)) は `Outcome` を駆動する。 OneWay-only の Driver が要るか、 Driver は還相専用かは未決 ── consumer signal を待つ

---

> **昇格** (2026-05-18): review (PR #1) で頂点型の名 `Voyage` を確定 (OQ1 解決)。 `Outcome` 不可侵・`Bracket` 据え置きの昇華構造を nostos lead が承認。 本 ADR を `accepted` に昇格。 残る Open Questions は後続の deepening 事項。
