/// The `Folder` trait encapsulates [the standard fold
/// operation][fold]. It can be fed many items using the `consume`
/// method. At the end, once all items have been consumed, it can then
/// be converted (using `complete`) into a final value.
///
/// [fold]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold
pub trait Folder<Item>: Sized {
    /// The type of result that will ultimately be produced by the folder.
    type Result;

    /// Consume next item and return new sequential state.
    fn consume(self, item: Item) -> Self;

    /// Consume items from the iterator until full, and return new sequential state.
    ///
    /// This method is **optional**. The default simply iterates over
    /// `iter`, invoking `consume` and checking after each iteration
    /// whether `full` returns false.
    ///
    /// The main reason to override it is if you can provide a more
    /// specialized, efficient implementation.
    fn consume_iter<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = Item>,
    {
        for item in iter {
            self = self.consume(item);
            if self.is_full() {
                break;
            }
        }
        self
    }

    /// Finish consuming items, produce final result.
    fn complete(self) -> Self::Result;

    /// Hint whether this `Folder` would like to stop processing
    /// further items, e.g. if a search has been completed.
    fn is_full(&self) -> bool {
        false
    }
}
