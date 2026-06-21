# ADR-0011 — async Bracket / drive の framing

> **Status**: `accepted` (2026-06-22、 VP review go + Bastet real-need 確認)
> **Date**: 2026-06-21 (draft) / 2026-06-22 (accepted)
> **Deciders**: mito (with claude Opus 4.8 as conversation partner)
> **Builds on**: [ADR-0002](0002-outcome-adt.md) (Outcome) / [ADR-0003](0003-lifecycle-loop-dual.md) (drive) / [ADR-0004](0004-bracket-and-driver.md) (Bracket / Driver)
> **Consumer signal**: VP **Bastet** (MIDI device lifecycle、 real-need・ユーザー承認済) — 加えて destroy-side reclaim (doc 24 §4.6)

> **番号について**: `ADR-0010` は [ADR-0009](0009-cgp-integration.md) が `nostos-cgp` concrete 用に予約済み。 本件 (async) は別軸のため 0011 を充てる。

---

## Context

`v0.1.1` 時点で `Bracket` / `Driver` / `drive` / `drive_bounded` はすべて **sync**。 最初の consumer (VP の worktree retry) は sync で完結している。

VP から **2 つ目の consumer signal** が来た ── **destroy-side reclaim** (doc 24 §4.6 の `destroying` lifecycle)。 create-side は VP PR #570 で出荷済、 destroy-side が文書化済みの次スライス:

```
txn{destroying} → external{ground + tmux reclaim} → txn{remove}
```

`external` op が **async** ── `git worktree remove` / `tmux kill-session` (spawn_blocking or async subprocess)、 さらに boot reconcile heal loop も async (DB + FS check を await)。 sync `Bracket` ではこの lifecycle を表現できない。

### signal 強度 ── real-need 確定 (2026-06-22 更新)

draft 時点 (2026-06-21) は destroy-side reclaim (doc 24 §4.6) を **文書化済みだが未コードの intent** signal として framing していた。 その後 VP review で **より早く立つ real-need consumer が確定**した:

**VP Bastet ── MIDI device lifecycle** (ユーザー承認済):
- device 接続 = `AsyncBracket` (`enter` = in/out port open + handshake / `exit` = close)
- registry = **reconnect heal loop** (hot-plug event で known device を再接続)
- World (TheWorld) lifecycle に **enclose** = 「閉じ込める」 を型で表現
- discovery は time-based poll を廃し **CoreMIDI notify (event-driven)** へ → `await` するため **async Bracket が必須**

これは destroy-side より先に立つ async-Bracket consumer であり、 signal は 「文書化 intent」 から **「ユーザー承認済みの real-need」 に格上げ**された ([ADR-0009](0009-cgp-integration.md) の consumer-signal-driven 規律を満たす)。 VP は **nostos 先行** sequencing を選択 (`0.2.0` を先に出し、 その上に Bastet を載せる) → 本 ADR を `accepted` 化し `0.2.0` 実装に進む。 concrete API は **Bastet の実コード (`bastet.rs`) と擦り合わせて確定**する。

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

**VP 実態 (2026-06-22, 最終)**: dyn 不要が確定した (D8) ことで、 VP は **concrete `MidiDeviceBracket` を inline drive する** ── この限り `Send` は **compiler 推論で足り、 `trait_variant` は不要**な見込みに戻った (registry が保持するのは uniform な `Active = ConnectedDevice` であって `dyn` ではない)。 `trait_variant` が要るのは将来 dyn を使う時 (D8) のみ。 D7 の 「`Send` は consumer 側」 方針は不変で、 当面 consumer の手間も最小。

### D8 — dyn 対応 = `alloc` feature 裏の boxed variant (将来枠、 `0.2.0` critical path 外)

**経緯の訂正 (2026-06-22, VP 再訂正)**: VP は一旦 「Bastet registry が heterogeneous device を `dyn` 保持するので `dyn AsyncBracket` 必須」 と回答したが、 これは **過剰訂正**だった。 指していた `bastet.rs:78 Box<dyn DeviceInput + Send>` は **sync の input parser の dyn** (byte→event、 `AsyncBracket` とは無関係) で、 device lifecycle 自体は **uniform/concrete にできる** (下記 「0.2.0 critical path と Bastet concrete shape」)。 → **`dyn AsyncBracket` は VP の critical path では不要**。

ただし dyn 対応は将来枠として価値が残る ── **非 MIDI device (ESP32/network 等) を同 registry に混ぜ `Active` 型が機種で別になる**なら、 そこで初めて heterogeneous `Active` の型消去に `dyn AsyncBracket` が要る。 MIDI 艦隊だけなら uniform/concrete で済む。 よって **D8 は good-to-have / 後追いとし、 `0.2.0` を塞がない**。

以下は将来実装する場合の設計 (保持)。 dyn = **boxing 必須** (`Pin<Box<dyn Future>>` ── 戻り future は impl 毎に型もサイズも違うので vtable に直接載らず、 箱詰めで型消去するしかない)。 `Box` は **`alloc` を要求**するので:

- **core default** = bare `AsyncBracket` (AFIT, `no_std`, alloc なし, `Send` 境界なし) ── ここまでが `0.2.0`。
- **`alloc` feature** 裏に object-safe な **`DynAsyncBracket`** (boxed) を置く (後追い)。 `alloc` は std 提供で**第三者依存ではない**ため **依存ゼロは保持**。

shape (将来実装の叩き台):

```rust
#[cfg(feature = "alloc")]
pub trait DynAsyncBracket {
    type Input; type Active; type Done; type Reborn; type Failed;
    fn enter<'a>(&'a self, input: Self::Input)
        -> Pin<Box<dyn Future<Output = Self::Active> + Send + 'a>>;
    fn exit<'a>(&'a self, active: Self::Active)
        -> Pin<Box<dyn Future<Output = Outcome<Self::Done, Self::Reborn, Self::Failed>> + Send + 'a>>;
}
```

**未決の核 (VP と叩く)**: bare `AsyncBracket` → `DynAsyncBracket` への **blanket impl が stable では書けない** ── blanket には 「`enter` の戻り future が `Send`」 と書く必要があるが、 その境界記法 (RTN) が未 stable (D7)。 → 2 案:

- **(A) 手書き boxed idiom**: VP が device 型に `DynAsyncBracket` を直接 impl (`Box::pin(async move { … })` 数行)。 stable で確実に動き、 core は依存ゼロのまま。 VP に少量の boilerplate。
- **(B) consumer-side trait_variant bridge**: VP が `trait_variant` で `Send` 版 `AsyncBracket` を生成 → core が 「`Send` 版からの blanket impl」 を提供。 boilerplate は減るが、 core と VP マクロの境界調整が要る。

**推奨は (A) から**着手 (最小・確実)、 `bastet.rs` の実装感触を見て (B) を検討。 `Send` は D7 どおり consumer 側で担保する。

## Consequences

### Positive
- VP **Bastet** の MIDI device lifecycle (enclose / reconnect heal loop) が型で表現できる ── real-need を満たす。 destroy-side reclaim (doc 24 §4.6) も同じ substrate に乗る。
- async が **2 つ目の consumer** として bracket/Outcome 抽象の generality を裏取りする。
- `async-trait` crate 不要 (AFIT/AsyncFn) ── core default の no_std / 依存ゼロを維持。
- sync API 無変更 ── VP worktree retry を壊さない (additive `0.2.0`)。

### Negative
- trait が sync/async で二重化 (`Bracket` / `AsyncBracket`、 `Driver` / `AsyncDriver`)。 共通ロジックの重複が生じうる。
- dyn 対応 (D8) は `alloc` feature を要し、 かつ stable では bare AFIT → `DynAsyncBracket` の blanket impl が書けない ── consumer に手書き boxed idiom (案 A) の boilerplate を要求する。
- `Send` 境界の記法 (RTN) が stable に無い (D7) ── multithread executor で回す consumer は `trait_variant` を被せる手間を負う。 core の依存ゼロとのトレードオフで consumer 側に倒した。
- RAII 契約が async で弱まる (D6) ── 利用者に 「heal loop 前提」 の理解を要求する。

### Neutral
- `0.2.0` の concrete API (特に D8 boxed variant の shape・blanket 案 A/B) は VP `bastet.rs` の実コードと擦り合わせて確定。
- effect-generic は将来 (stable + signal) に再考の余地を残す。

## Resolved Questions (VP review 2026-06-22 で回答)

1. **D-OPEN-1 — dyn 互換性** → **不要 (2 度の訂正を経て確定)**。 一旦 「要る」 に反転したが、 VP 再訂正で device lifecycle は uniform/concrete (単一 `MidiDeviceBracket`、 `Active = ConnectedDevice`) と判明。 `dyn AsyncBracket` は VP critical path 外 ── 将来の heterogeneous device 枠として **D8 に後追いで保持**。
2. **D-OPEN-2 — 命名** → **suffix 方式 (`drive_async` / `AsyncBracket`)、 `async` feature gate なし**。 async は AFIT で依存ゼロ追加のため feature gate しても得る物が無い (cfg 表面が増えるだけ)。 discoverability も suffix が素直。 (boxed variant のみ `alloc` gate ── D8。)
3. **D-OPEN-3 — spawn_blocking vs async subprocess** → **nostos agnostic を確認**。 VP の device I/O は CoreMIDI notify (async) / midir open-close (sync) の混在、 destroy-side は `tokio::process` 寄り。 いずれも nostos は step を `AsyncFnMut` で受けるだけで非依存 ── どう回すかは **VP 内部選択**であり nostos の設計 driver ではない。
4. **D-OPEN-4 — heal loop の責務分界** → **doc 規約に留め、 型強制しない**。 「reconcile 可能状態」 の定義は VP ドメイン依存 (lane/worktree/device の reconcile 意味論次第) で、 nostos が型で課すと over-constrain。 nostos は `Outcome` (Done/Reborn/Failed) の seam を提供し、 reconcile 意味論は consumer が定義する ── Minimum 原則と整合。

## 0.2.0 critical path と Bastet concrete shape

VP 再訂正で確定した **`0.2.0` の最小 critical path** (これだけで Bastet が載る):

- **bare `AsyncBracket`** (AFIT、 `no_std`、 alloc なし、 `Send` 境界なし)
- **`drive_async` / `drive_bounded_async`** (`AsyncFnMut(I) -> Outcome` step)
- **`AsyncDriver`** (async `next` + default async `run`)

→ `DynAsyncBracket` (D8)・`alloc` feature は **0.2.0 に含めない** (後追い)。

Bastet が載せる concrete shape (擦り合わせ済 ── 単一 concrete 型、 機種差は data):

```rust
struct MidiDeviceBracket;          // device 共通の 1 型
type Input  = DeviceDescriptor;    // displayName + kind/profile
type Active = ConnectedDevice;     // 全機種 uniform (port 名 / in・out 有無 / connected_at)
type Done   = ();
type Reborn = DeviceDescriptor;    // hot-unplug → 同 descriptor 再接続 (reconnect heal loop)
type Failed = ConnectError;
// enter = port open + profile.handshake() / exit = close。 registry は HashMap<name, ConnectedDevice>。
```

機種差 (handshake / parse) は `DeviceProfile` / `DeviceInput` を **data として保持**し、 Bracket は単一 concrete 型に保つ ── nostos の Minimum 原則 (dyn より concrete + data) と一致。

## 着手条件 (D5 の規律) → 充足

`accepted` (2026-06-22) かつ **real-need consumer (Bastet) 確定** で着手条件を充足。 VP は **nostos 先行** sequencing を選択。 → 上記 critical path を `0.2.0` として実装に進む。

---

> **draft note** (2026-06-21): VP handoff (PR #572 完了報告への返答) を受けた framing draft。 sync で proven な Outcome/Bracket/drive を、 no_std/依存ゼロ・additive を保ったまま async へ持ち上げる方針。 D1 (別 `AsyncBracket` trait via AFIT)・D6 (RAII の async 境界)・D7 (`Send` は consumer 側) が擦り合わせの核。
>
> **accepted note** (2026-06-22): VP review で D7 go / D1・D6 同意 / D-OPEN 全回答。 さらに **real-need consumer (Bastet MIDI device lifecycle、 ユーザー承認済) が確定**し、 VP は nostos 先行 sequencing を選択 ── `accepted` 化。
>
> **訂正 note** (2026-06-22, 同日): VP が dyn 回答を再訂正 ── `bastet.rs:78` の `dyn` は sync parser のもので `AsyncBracket` 無関係、 device lifecycle は uniform/concrete (`Active = ConnectedDevice`)。 → **`0.2.0` critical path = bare `AsyncBracket` + `drive_async`/`drive_bounded_async` + `AsyncDriver` のみ**。 D8 (`DynAsyncBracket`/`alloc`) は **将来枠に降格** (非 MIDI device で `Active` が heterogeneous になる時)、 `0.2.0` を塞がない。 D7 の Send も concrete inline drive で推論される見込みに緩和。
>
> **裏どり** (2026-06-21): toolchain 前提を web で検証。 AFIT (1.75) / AsyncFn family (1.85) は stable、 **RTN (Send 境界) と AsyncDrop は未 stable** を確認 ── これが D7 と D6 の根拠。 sources: [RTN 標準化 PR #138424 (close)](https://github.com/rust-lang/rust/pull/138424) / [AsyncDrop tracking #126482](https://github.com/rust-lang/rust/issues/126482) / [AsyncDrop+Drop 必須 #142606](https://github.com/rust-lang/rust/pull/142606) / [AFIT announce](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)。
>
> **裏どり** (2026-06-21): toolchain 前提を web で検証。 AFIT (1.75) / AsyncFn family (1.85) は stable、 **RTN (Send 境界) と AsyncDrop は未 stable** を確認 ── これが D7 と D6 の根拠。 sources: [RTN 標準化 PR #138424 (close)](https://github.com/rust-lang/rust/pull/138424) / [AsyncDrop tracking #126482](https://github.com/rust-lang/rust/issues/126482) / [AsyncDrop+Drop 必須 #142606](https://github.com/rust-lang/rust/pull/142606) / [AFIT announce](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)。
</content>
