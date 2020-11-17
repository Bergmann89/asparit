use super::{Folder, Reducer, WithSetup};

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
pub trait Consumer<I>: WithSetup + Send + Sized {
    /// The type of folder that this consumer can be converted into.
    type Folder: Folder<I, Result = Self::Result>;

    /// The type of reducer that is produced if this consumer is split.
    type Reducer: Reducer<Self::Result>;

    /// The type of result that this consumer will ultimately produce.
    type Result: Send;

    /// Divide the consumer into two consumers, one processing the left items
    /// and one processing the right items from. Also produces a reducer that
    /// can be used to reduce the results at the end.
    fn split(self) -> (Self, Self, Self::Reducer) {
        panic!("Consumer could not be used in unindexed mode!");
    }

    /// Divide the consumer into two consumers, one processing items
    /// `0..index` and one processing items from `index..`. Also
    /// produces a reducer that can be used to reduce the results at
    /// the end.
    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        panic!("Consumer could not be used in indexed mode!");
    }

    /// Convert the consumer into a folder that can consume items
    /// sequentially, eventually producing a final result.
    fn into_folder(self) -> Self::Folder;

    /// Hint whether this `Consumer` would like to stop processing
    /// further items, e.g. if a search has been completed.
    fn is_full(&self) -> bool {
        false
    }
}
