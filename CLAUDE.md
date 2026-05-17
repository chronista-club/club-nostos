# club-nostos — AI 開発ガイド

> bracket / lifecycle / loop primitives for Rust。 atlas **νόστος** = 「変容を経た帰還」。

## 基本方針

- **丁寧さ > 速度**: 急がず、 質の高いコード・ドキュメントを残す
- **Legacy は残さない**: deprecated / 後方互換のためだけの実装は不要。 不要なコードは削除する
- **Minimum を保つ**: 必要最小限の状態を維持する。 過剰な抽象化を避ける

## 現状 — `v0.1.0` founding scaffold

trait 実装は未着手。 設計判断は `docs/adr/` に記録してから着手する。
ADR-0001 (bracket + Outcome ADT) が `proposed` 段階で、 具体 signature は後続 ADR で個別に深掘る。

## 命名規則 (chronista-club ecosystem)

`club-` prefix 命名規則に従う。 prefix は crates.io / GitHub / Finder に listed される識別子に付き、 **lib 名は bare name** (例: `nostos`)。

| Layer | Name |
|-------|------|
| creo-memories atlas | name `nostos`、 display name `Nostos Club` |
| 内部呼称 (project 通称) | `nostos` |
| GitHub repo | `chronista-club/club-nostos` |
| local checkout | `~/repos/club-nostos/` |
| crates.io package | `club-nostos` |
| Rust crate identifier (`use ...`) | `nostos` |

新規 crate を足す時は `[package].name = "club-<name>"` / `[lib].name = "<name>"` (bare、 prefix なし) とする。

## アーキテクチャ scope

- **Bracket** — `enter` / `active` / `exit` の lifecycle primitive
- **Outcome ADT** — `Done(O)` / `Reborn(I)` / `Failed(E)` の帰還三相
- lifecycle ↔ loop dual view / node graph editor との同型 substrate / CGP-style component composition

詳細と設計 axes は [`docs/adr/0001-bracket-and-outcome.md`](docs/adr/0001-bracket-and-outcome.md)。

## テスト

```bash
cargo test --workspace
cargo clippy --lib --workspace -- -D warnings
cargo fmt --all -- --check
```

## ドキュメント構造

| ディレクトリ | 用途 |
|-------------|------|
| `docs/adr/` | Architecture Decision Records |

将来 spec (What & Why) / guides (使い方) を足す時は unison precedent (`spec/` `design/` `guides/`) に揃える。 Living Documentation 原則: ドキュメントとコードは常に同期させる。

## creo-memories

- atlas: name `nostos` / display name `Nostos Club` (`atl_1Cb35Uxd7zymxS6e38hJ23`)
- 起点 memory: founding decision `mem_1Cb35YiGHG1f7UdXyyt16L` / priority advisory `mem_1Cb39WrS5tfUsaMV8yzsLR` / scaffold landing `mem_1Cb5v4CDxEovtuStmhts7u`
