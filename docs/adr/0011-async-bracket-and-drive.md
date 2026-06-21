# ADR-0011 — async Bracket / drive の framing

> **Status**: `proposed` (2026-06-21、 VP との擦り合わせ待ち)
> **Date**: 2026-06-21
> **Deciders**: mito (with claude Opus 4.8 as conversation partner)
> **Builds on**: [ADR-0002](0002-outcome-adt.md) (Outcome) / [ADR-0003](0003-lifecycle-loop-dual.md) (drive) / [ADR-0004](0004-bracket-and-driver.md) (Bracket / Driver)
> **Consumer signal**: Vantage Point destroy-side reclaim (doc 24 §4.6) — VP PR #572 完了報告で要請

> **番号について**: `ADR-0010` は [ADR-0009](0009-cgp-integration.md) が `nostos-cgp` concrete 用に予約済み。 本件 (async) は別軸のため 0011 を充てる。

---

## Context

`v0.1.1` 時点で `Bracket` / `Driver` / `drive` / `drive_bounded` はすべて **sync**。 最初の consumer (VP の worktree retry) は sync で完結している。

VP から **2 つ目の consumer signal** が来た ── **destroy-side reclaim** (doc 24 §4.6 の `destroying` lifecycle)。 create-side は VP PR #570 で出荷済、 destroy-side が文書化済みの次スライス:

```
txn{destroying} → external{ground + tmux reclaim} → txn{remove}
```

`external` op が **async** ── `git worktree remove` / `tmux kill-session` (spawn_blocking or async subprocess)、 さらに boot reconcile heal loop も async (DB + FS check を await)。 sync `Bracket` ではこの lifecycle を表現できない。

### signal 強度の honest な評価

VP destroy-side の **コードはまだ未実装**。 本 ADR は 「VP が destroy-side を書き始める前に async の shape を決めておく」 framing である。 これは [ADR-0009](0009-cgp-integration.md) が park した CGP (signal ゼロ) とは異なる ── consumer は doc 24 §4.6 で **op レベルまで具体化済**で、 bracket/retry の shape は後で変えると rework コストが高い。 「shape を先に決める」 ことが投機ではなく **de-risk** である、 というのが着手判断の根拠 (D5)。

実装 (`0.2.0`) は本 ADR 合意後。 本 ADR は設計判断の framing に留め、 concrete な API 確定は VP destroy-side の実コードと擦り合わせる。

### toolchain の前提 (channel 1.95、 2026-06-21 裏どり済み)

| 機能 | stable since | 1.95 で |
|------|-------------|---------|
| AFIT (async fn in trait) | 1.75 | ✅ 使える |
| RPITIT | 1.75 | ✅ |
| `AsyncFn` / `AsyncFnMut` / `AsyncFnOnce` + async closures | 1.85 | ✅ 使える |
| **RTN** (return type notation = future への `Send` 境界記法) | — | ❌ **未 stable** ── 標準化 PR [#138424](https://github.com/rust-lang/rust/pull/138424) が 2025-12 に未マージで close |
| **AsyncDrop** | — | ❌ **未 stable** (nightly のみ、 tracking [#126482](https://github.com/rust-lang/rust/issues/126482)) |

→ `async-trait` crate (boxing コスト・`Box<dyn Future>` alloc) は **不要**。 AFIT と AsyncFn family を直接使える。 AFIT/RPITIT は alloc を要さない state machine に desugar されるため **`no_std` を保てる** (core の dependency-free / no_std 性 ── ADR-0004 の前提 ── を壊さない)。

残る制約は **2 つ**:

1. **dyn 非互換**: `dyn AsyncBracket` は不可 (AFIT の戻り future は impl 毎の匿名型で vtable に載らない)。
2. **`Send` 境界 (本命)**: AFIT の戻り future に `Send` を綺麗に課す記法 (RTN) が **stable に存在しない** ── 上表の通り標準化が頓挫。 tokio の multithread executor は `spawn` に `Send` 必須なので、 VP には実害がある。 → 扱いは **D7** で decide。

## Decision (提案 ── VP 擦り合わせ後に確定)

### D1 — 別 trait `AsyncBracket` (AFIT)、 sync `Bracket` は不変

sync `Bracket` の `enter` / `exit` を async 化すると、 それを消費している VP worktree retry が壊れる (breaking)。 よって **既存 trait は触らず**、 async は別 trait として additive に足す:

```rust
pub trait AsyncBracket {
    type Input; type Active; type Done; type Reborn; type Failed;
    async fn enter(&self, input: Self::Input) -> Self::Active;
    async fn exit(&self, active: Self::Active)
        -> Outcome<Self::Done, Self::Reborn, Self::Failed>;
}
```

却下した代替:
- **(a) 既存 `Bracket` を async 化**: D3 (sync 不変) に反する breaking change。 却下。
- **(c) effect-generic / maybe-async** (一つの trait で sync/async 両対応): Rust に effect generics は未 stable、 パターンも未成熟。 nostos の 「Minimum を保つ・過剰な抽象化を避ける」 (CLAUDE.md) に反する。 却下 ── 将来 effect generics が stable 化し、 かつ consumer signal があれば再考。

### D2 — async `drive` / `drive_bounded` (AsyncFnMut step)

sync 版に加え、 async step を回す版を additive に追加 (名前は仮、 D-OPEN-2):

```rust
pub async fn drive_async<O, I, E, F>(initial: I, mut step: F) -> Result<O, E>
where F: AsyncFnMut(I) -> Outcome<O, I, E>;

pub async fn drive_bounded_async<O, I, E, F>(initial: I, max_steps: usize, mut step: F)
    -> Outcome<O, I, E>
where F: AsyncFnMut(I) -> Outcome<O, I, E>;
```

`AsyncFnMut(I) -> Outcome<…>` (1.85 stable) で step を受け、 各 step を `.await` する。 制御フロー (Reborn が続く限り回す / 上限で `Reborn(i)` を返す) は sync 版と同一 ── ADR-0003 の意味論をそのまま async に持ち上げる。

### D3 — sync との共存は additive、 既存を壊さない

`0.1.x` の sync `Bracket` / `Driver` / `drive` / `drive_bounded` は VP worktree retry が消費中。 **一切変更しない**。 async は新 API として並置する。 `0.2.0` は additive minor bump で、 breaking を含まない。

### D4 — `Outcome` / `Voyage` は sync/async 共通

帰還の三相 (`Done` / `Reborn` / `Failed`) と `Voyage` は **データ**であり executor 非依存。 sync/async いずれの substrate も同じ `Outcome<O, I, E>` を返す ── 型の共有は確定 (新型は作らない)。

### D5 — async Driver も surface に含める

`Driver::run` は `Bracket::enter`/`exit` を呼ぶため、 async Bracket には async な駆動主体が要る。 `AsyncDriver<B: AsyncBracket>` (`async fn next` + `async fn run`) を D1/D2 と同じ additive 方針で足す。 handoff は Bracket/drive を名指したが、 Driver は論理的従属物として本 ADR の scope に含める。

### D6 — RAII 契約の async 境界を honest に引く

ADR-0004 の RAII 純粋性契約: 「`Active` は Resource を borrow し、 `Failed` でも `enter` 前と同一に戻る」。 async ではこれが **そのままは保てない**:

- async cancellation = future が await 点で drop される。 cleanup が必要。
- Rust に **AsyncDrop は未 stable** ── `Drop` は sync しか実行できない。 `tmux kill-session` のような **async cleanup を drop 時に走らせられない**。
- 裏どり (2026-06-21): nightly の AsyncDrop ですら 「**`AsyncDrop` を実装する型は sync `Drop` も実装必須**」 ([PR #142606](https://github.com/rust-lang/rust/pull/142606)) ── async drop が sync 文脈 (panic/unwinding 等) でも走るため sync fallback が要る、 という未解決問題の現れ。 **AsyncDrop が将来 stable 化しても本 D6 の境界線は妥当**。

よって async 版の契約はこう **弱める** (そして明示する):

- **sync-restorable な resource** (Drop で巻き戻せる) → ADR-0004 の strict 契約を維持。
- **async-cleanup な resource** (cleanup 自体が async) → strict RAII は保証不能。 契約を 「cancel されても **reconcile 可能な状態**を残す (idempotent)。 収束は heal loop が担う」 に置き換える。

これは VP の設計と整合する ── VP が **boot reconcile heal loop** を持つのは、 まさに async destroy op の完了を drop で保証できないからである。 nostos は 「strict RAII が効く範囲」 と 「heal loop に委ねる範囲」 の **境界を型と doc で明示**し、 AsyncDrop があるかのように振る舞わない。

### D7 — `Send` 境界は consumer 側で解決、 core は bare `AsyncBracket` を出す

toolchain 前提のとおり、 stable 1.95 に `Send` 境界の記法 (RTN) は無い。 唯一の現実的 workaround は `trait_variant` crate (`Send` 版 super-trait を生成する proc-macro) だが、 これを core に入れると **依存ゼロ原則が崩れる**。 よって:

- **`nostos-core` は bare な `AsyncBracket` (AFIT) を expose する**のみ。 `Send` 境界は課さない。
- multithread executor (tokio 等) で future を `Send` にしたい consumer は、 **自分の側で `trait_variant` を被せる** (or nightly で RTN を使う)。
- core の **dependency-free / no_std を死守**する ── これは ADR-0004 / `v0.1.0` から一貫した nostos の根本制約。

却下: core が `trait_variant` で `Send` 版も提供する案 ── 依存を 1 つ許容して consumer の手間を省くトレードオフだが、 「依存ゼロ」 を優先して却下。 RTN が将来 stable 化すれば、 core で `Send` 境界を無依存に書けるようになり本判断は再考の余地が出る。

## Consequences

### Positive
- VP destroy-side reclaim (doc 24 §4.6) の `destroying` lifecycle が型で表現できる。
- async が **2 つ目の consumer** として bracket/Outcome 抽象の generality を裏取りする。
- `async-trait` crate 不要 (AFIT/AsyncFn) ── core の no_std / 依存ゼロを維持。
- sync API 無変更 ── VP worktree retry を壊さない (additive `0.2.0`)。

### Negative
- trait が sync/async で二重化 (`Bracket` / `AsyncBracket`、 `Driver` / `AsyncDriver`)。 共通ロジックの重複が生じうる。
- AFIT は dyn 非互換 ── `dyn AsyncBracket` が要る consumer が出たら別途対応 (D-OPEN-1)。
- `Send` 境界の記法 (RTN) が stable に無い (D7) ── multithread executor で回す consumer は `trait_variant` を被せる手間を負う。 core の依存ゼロとのトレードオフで consumer 側に倒した。
- RAII 契約が async で弱まる (D6) ── 利用者に 「heal loop 前提」 の理解を要求する。

### Neutral
- 具体 API (命名・shape) は VP destroy-side の実コードと擦り合わせて `0.2.0` 実装時に確定。
- effect-generic は将来 (stable + signal) に再考の余地を残す。

## Open Questions (VP / 実装前に詰める)

1. **D-OPEN-1 — dyn 互換性**: VP は `dyn AsyncBracket` を要するか? 不要なら AFIT 直書きで済む (VP は具体型を駆動するので不要の見込み)。 要るなら `Box<dyn Future>` への boxing が要る ── consumer 側で包むか、 core が boxed variant を提供するか。 (`Send` 境界の扱いは D7 で決定済み。)
2. **D-OPEN-2 — 命名**: `drive_async` / `AsyncBracket` suffix 方式か、 `nostos::r#async` module 分離か、 cargo feature (`async`) gate か。 discoverability と no_std 維持のバランス。
3. **D-OPEN-3 — spawn_blocking vs async subprocess**: `git worktree remove` を VP がどちらで回すかで step の `await` 形が変わる。 nostos は step を `AsyncFnMut` で受けるだけなので非依存のはずだが要確認。
4. **D-OPEN-4 — heal loop の責務分界 (D6)**: reconcile-可能状態の最小契約を nostos が型で課すか、 doc 規約に留めるか。

## 着手条件 (D5 の規律)

本 ADR が `accepted` になり、 かつ VP destroy-side の実コード (or 直近の着手) が確認できた時点で `0.2.0` 実装に進む。 それまでは framing として保持 ── async API は 「VP が destroy-side を書く時」 に実需と擦り合わせて確定する。

---

> **draft note** (2026-06-21): VP handoff (PR #572 完了報告への返答) を受けた framing draft。 sync で proven な Outcome/Bracket/drive を、 no_std/依存ゼロ・additive を保ったまま async へ持ち上げる方針。 D1 (別 `AsyncBracket` trait via AFIT)・D6 (RAII の async 境界)・D7 (`Send` は consumer 側) が擦り合わせの核。 `accepted` 化と実装着手は VP destroy-side の実需確認後。
>
> **裏どり** (2026-06-21): toolchain 前提を web で検証。 AFIT (1.75) / AsyncFn family (1.85) は stable、 **RTN (Send 境界) と AsyncDrop は未 stable** を確認 ── これが D7 と D6 の根拠。 sources: [RTN 標準化 PR #138424 (close)](https://github.com/rust-lang/rust/pull/138424) / [AsyncDrop tracking #126482](https://github.com/rust-lang/rust/issues/126482) / [AsyncDrop+Drop 必須 #142606](https://github.com/rust-lang/rust/pull/142606) / [AFIT announce](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)。
</content>
