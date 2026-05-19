# ADR-0009 — Axis E: CGP integration の framing

> **Status**: `accepted` (2026-05-19、 review 完了)
> **Date**: 2026-05-19
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0001](0001-bracket-and-outcome.md) Axis E
> **Builds on**: [ADR-0004](0004-bracket-and-driver.md) (Bracket / Driver) / [ADR-0007](0007-graph-design.md) (Node / Graph)

---

## Context

[ADR-0001](0001-bracket-and-outcome.md) が framing した 5 axes のうち、 A/B/C/D が実装済み (ADR-0002〜0008)。 残るは **Axis E ── CGP-style component composition**。

Axis E の問い (ADR-0001 より): nostos と **`cgp` crate (Context-Generic Programming)** の関係。 nostos primitive を CGP component として expose するか、 CGP は consumer 側の dependency に留めるか。 ADR-0001 暫定傾き ── 「nostos core は CGP に非依存。 別 crate (`nostos-cgp`) で integration を提供する形が分離度高い」。

### CGP とは (本 ADR が前提する理解)

**Context-Generic Programming** ── component を 「context」 に対して generic にする Rust の設計手法 / `cgp` crate。 provider / consumer の分離と delegating impl により、 ある振る舞いの実装を **context ごとに遅延 bind** できる ── trait object の動的 dispatch を使わずに、 実装を後から差し替え可能にする。 chronista-club では unison が `cgp` を採用済み。

nostos との接点: nostos の `Bracket` / `Driver` / `Node` は 「実行戦略を持つ component」。 CGP は 「実行戦略を context-generic に inject する」 機構 ── 両者は **「振る舞いの遅延 bind」** という同じ関心を別レイヤーで扱う。

## Decision

### D1 — `nostos-core` / `nostos-graph` は CGP 非依存を維持

core (`club-nostos`) と graph (`club-nostos-graph`) は `cgp` crate に依存しない。

- `nostos-core` は `v0.1.0` で `no_std` / dependency-free。 `cgp` は相応に重い framework であり、 これを core に持ち込むと core の最小性・移植性が失われる
- nostos primitive (`Bracket` 等) は CGP を知らなくても完結する ── CGP は **使い方の一つ**であって、 primitive の定義に不可欠ではない

### D2 — CGP integration は別 crate `nostos-cgp`

ADR-0001 暫定を確定。 CGP 連携は workspace member の別 crate とする。

- 命名 (CONVENTIONS.md 準拠): package `club-nostos-cgp` / lib `nostos_cgp`
- `nostos-cgp` は `club-nostos` (+ 必要なら `club-nostos-graph`) と `cgp` の両方に依存し、 両者を橋渡しする adapter 層

### D3 — `nostos-cgp` が提供するもの (framing)

`nostos-cgp` は nostos primitive を **CGP component として expose** する:

- `Bracket` / `Driver` / `Node` を CGP の provider / consumer trait に写像
- bracket/lifecycle/loop の **実行戦略を context ごとに遅延 inject** する ── ADR-0001 Axis E の 「execution strategy を context-generic に inject」 の具体化
- consumer は自分の context に nostos の lifecycle 振る舞いを bind し、 trait object なしで実装を差し替えられる

具体的な trait 写像・`cgp` の API への adapt は本 ADR の scope 外 ── ADR-0010 (concrete) で扱う。

### D4 — 本 ADR は framing に留める

Axis D が ADR-0005 (framing) → ADR-0007 (concrete) の二段だったのと同じく、 Axis E も ADR-0009 (framing) → ADR-0010 (concrete) とする。 `cgp` crate は外部 framework で API も版で動く ── concrete 設計時に `cgp` の現行 API を精査する。

### D5 — `nostos-cgp` の concrete 設計 (ADR-0010) に進む

nostos の開発はこれまで一貫して **consumer-signal-driven** だった ── creoui の lifecycle-spine が Voyage を、 VP messaging が graph review を駆動した。 抽象は実需に炙られて鍛えられてきた。

review (2026-05-19) で **CGP integration の consumer signal が確認された**。 よって `nostos-cgp` は投機実装ではなく実需に基づく ── concrete 設計 ADR-0010 に進む。 ADR-0010 はその signal を Context に据え、 `cgp` crate の現行 API を精査して `nostos-cgp` の trait 写像を decide する。

## Consequences

### Positive

- ADR-0001 の 5 axes すべてに ADR が対応 ── framing が完結する
- core の dependency-free / no_std 性が CGP によって損なわれない
- 投機的実装を避け、 nostos の consumer-signal-driven な開発規律を保つ

### Negative

- `cgp` は外部 framework で API が版で動く ── `nostos-cgp` の concrete 設計 (ADR-0010) は `cgp` 現行 API への追従コストを負う

### Neutral

- `cgp` crate の version 追従は ADR-0010 の関心 ── 本 ADR は version を fix しない

## Resolved Questions

review (2026-05-19) で確定:

1. **consumer signal の有無** → signal あり。 D5 を 「ADR-0010 に即進む」 に確定
2. **どの primitive を component 化するか** → ADR-0010 で signal の具体に基づき精査
3. **`cgp` の version** → ADR-0010 で `cgp` 現行 API 精査時に決める

---

> **昇格** (2026-05-19): review で CGP integration の consumer signal を確認 ── D5 を 「signal 確認 → ADR-0010 へ」 に確定。 本 ADR を `accepted` に昇格。 これで ADR-0001 の 5 axes すべてに ADR が対応する。 次は `nostos-cgp` の concrete 設計 ADR-0010。
