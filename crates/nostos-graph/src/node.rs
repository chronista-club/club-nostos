//! [`Node`] trait と [`Bracket`] → [`Node`] adapter。

use nostos::{Bracket, Voyage};

/// graph の node ── 入力 `V` を受け [`Voyage`] を返す。
///
/// [`Bracket`] の `enter` + `exit` を 1 メソッド `drive` に畳んだ object-safe な層。
/// graph から見て Active 相は node 内部の不透明事 ── 「`V` を入れて `Voyage` が出る」
/// だけが graph の関心。 固定 `V` で object-safe なので `Box<dyn Node<V>>` が作れる。
pub trait Node<V> {
    /// 入力を受け、 [`Voyage`] を返す。
    fn drive(&self, input: V) -> Voyage<V, V, V>;
}

/// static な [`Bracket`] を [`Node`] に adapt する wrapper。
///
/// [`Bracket`] の関連型 `Input` / `Done` / `Reborn` / `Failed` が graph value 型 `V` に
/// collapse する Bracket を node 化する (`Active` は内部に隠れる)。 Bracket は
/// RoundTrip 専用 primitive のため、 Bracket-backed node は常に `RoundTrip` を emit する。
///
/// # Examples
///
/// ```
/// use nostos::{Bracket, Outcome, Voyage};
/// use nostos_graph::{BracketNode, Node};
///
/// struct AddOne;
/// impl Bracket for AddOne {
///     type Input = i32;
///     type Active = i32;
///     type Done = i32;
///     type Reborn = i32;
///     type Failed = i32;
///     fn enter(&self, input: i32) -> i32 {
///         input
///     }
///     fn exit(&self, active: i32) -> Outcome<i32, i32, i32> {
///         Outcome::Done(active + 1)
///     }
/// }
///
/// let node = BracketNode(AddOne);
/// assert_eq!(node.drive(5), Voyage::RoundTrip(Outcome::Done(6)));
/// ```
pub struct BracketNode<B>(pub B);

impl<B, V> Node<V> for BracketNode<B>
where
    B: Bracket<Input = V, Done = V, Reborn = V, Failed = V>,
{
    fn drive(&self, input: V) -> Voyage<V, V, V> {
        Voyage::RoundTrip(self.0.exit(self.0.enter(input)))
    }
}
