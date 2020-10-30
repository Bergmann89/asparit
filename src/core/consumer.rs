use super::{Folder, Reducer};

/// A consumer is effectively a [generalized "fold" operation][fold],
/// and in fact each consumer will eventually be converted into a
/// [`Folder`]. What makes a consumer special is that, like a
/// [`Producer`], it can be **split** into multiple consumers using
/// the `split_off_left` method. When a consumer is split, it produces two
/// consumers, as well as a **reducer**. The two consumers can be fed
/// items independently, and when they are done the reducer is used to
/// combine their two results into one. See [the README][r] for further
/// details.
///
/// [r]: README.md
/// [fold]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold
/// [`Folder`]: trait.Folder.html
/// [`Producer`]: trait.Producer.html
pub trait Consumer<I>: Send + Sized {
    /// The type of folder that this consumer can be converted into.
    type Folder: Folder<I, Result = Self::Result>;

    /// The type of reducer that is produced if this consumer is split.
    type Reducer: Reducer<Self::Result>;

    /// The type of result that this consumer will ultimately produce.
    type Result: Send;

    /// Splits off a "left" consumer and returns it. The `self`
    /// consumer should then be used to consume the "right" portion of
    /// the data. (The ordering matters for methods like find_first --
    /// values produced by the returned value are given precedence
    /// over values produced by `self`.) Once the left and right
    /// halves have been fully consumed, you should reduce the results
    /// with the result of `to_reducer`.
    fn split_off_left(&self) -> (Self, Self::Reducer);

    /// Convert the consumer into a folder that can consume items
    /// sequentially, eventually producing a final result.
    fn into_folder(self) -> Self::Folder;

    /// Hint whether this `Consumer` would like to stop processing
    /// further items, e.g. if a search has been completed.
    fn is_full(&self) -> bool {
        false
    }
}

/// A stateless consumer can be freely copied. These consumers can be
/// used like regular consumers, but they also support a
/// `split_at` method that does take an index to split.
pub trait IndexedConsumer<I>: Consumer<I> {
    /// Divide the consumer into two consumers, one processing items
    /// `0..index` and one processing items from `index..`. Also
    /// produces a reducer that can be used to reduce the results at
    /// the end.
    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer);
}
