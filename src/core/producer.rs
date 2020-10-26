use super::Folder;

/// A variant on `Producer` which does not know its exact length or
/// cannot represent it in a `usize`. These producers act like
/// ordinary producers except that they cannot be told to split at a
/// particular point. Instead, you just ask them to split 'somewhere'.
///
/// (In principle, `Producer` could extend this trait; however, it
/// does not because to do so would require producers to carry their
/// own length with them.)
pub trait Producer: Send + Sized {
    /// The type of item returned by this producer.
    type Item;

    /// Split midway into a new producer if possible, otherwise return `None`.
    fn split(self) -> (Self, Option<Self>);

    /// Iterate the producer, feeding each element to `folder`, and
    /// stop when the folder is full (or all elements have been consumed).
    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>;
}

/// A `Producer` is effectively a "splittable `IntoIterator`". That
/// is, a producer is a value which can be converted into an iterator
/// at any time: at that point, it simply produces items on demand,
/// like any iterator. But what makes a `Producer` special is that,
/// *before* we convert to an iterator, we can also **split** it at a
/// particular point using the `split_at` method. This will yield up
/// two producers, one producing the items before that point, and one
/// producing the items after that point (these two producers can then
/// independently be split further, or be converted into iterators).
/// In Rayon, this splitting is used to divide between threads.
/// See [the `plumbing` README][r] for further details.
///
/// Note that each producer will always produce a fixed number of
/// items N. However, this number N is not queryable through the API;
/// the consumer is expected to track it.
///
/// NB. You might expect `Producer` to extend the `IntoIterator`
/// trait.  However, [rust-lang/rust#20671][20671] prevents us from
/// declaring the DoubleEndedIterator and ExactSizeIterator
/// constraints on a required IntoIterator trait, so we inline
/// IntoIterator here until that issue is fixed.
///
/// [r]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md
/// [20671]: https://github.com/rust-lang/rust/issues/20671
pub trait IndexedProducer: Send + Sized {
    /// The type of item that will be produced by this producer once
    /// it is converted into an iterator.
    type Item;

    /// The type of iterator we will become.
    type IntoIter: Iterator<Item = Self::Item> + DoubleEndedIterator + ExactSizeIterator;

    /// Convert `self` into an iterator; at this point, no more parallel splits
    /// are possible.
    fn into_iter(self) -> Self::IntoIter;

    /// The minimum number of items that we will process
    /// sequentially. Defaults to 1, which means that we will split
    /// all the way down to a single item. This can be raised higher
    /// using the [`with_min_len`] method, which will force us to
    /// create sequential tasks at a larger granularity. Note that
    /// Rayon automatically normally attempts to adjust the size of
    /// parallel splits to reduce overhead, so this should not be
    /// needed.
    ///
    /// [`with_min_len`]: ../trait.IndexedParallelIterator.html#method.with_min_len
    fn min_len(&self) -> usize {
        1
    }

    /// The maximum number of items that we will process
    /// sequentially. Defaults to MAX, which means that we can choose
    /// not to split at all. This can be lowered using the
    /// [`with_max_len`] method, which will force us to create more
    /// parallel tasks. Note that Rayon automatically normally
    /// attempts to adjust the size of parallel splits to reduce
    /// overhead, so this should not be needed.
    ///
    /// [`with_max_len`]: ../trait.IndexedParallelIterator.html#method.with_max_len
    fn max_len(&self) -> usize {
        usize::MAX
    }

    /// Split into two producers; one produces items `0..index`, the
    /// other `index..N`. Index must be less than or equal to `N`.
    fn split_at(self, index: usize) -> (Self, Self);

    /// Iterate the producer, feeding each element to `folder`, and
    /// stop when the folder is full (or all elements have been consumed).
    ///
    /// The provided implementation is sufficient for most iterables.
    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        folder.consume_iter(self.into_iter())
    }
}

/// The `ProducerCallback` trait is a kind of generic closure,
/// [analogous to `FnOnce`][FnOnce]. See [the corresponding section in
/// the plumbing README][r] for more details.
///
/// [r]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md#producer-callback
/// [FnOnce]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html
pub trait ProducerCallback<T> {
    /// The type of value returned by this callback. Analogous to
    /// [`Output` from the `FnOnce` trait][Output].
    ///
    /// [Output]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html#associatedtype.Output
    type Output;

    /// Invokes the callback with the given producer as argument. The
    /// key point of this trait is that this method is generic over
    /// `P`, and hence implementors must be defined for any producer.
    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T>;
}

/// The `IndexedProducerCallback` trait is a kind of generic closure,
/// [analogous to `FnOnce`][FnOnce]. See [the corresponding section in
/// the plumbing README][r] for more details.
///
/// [r]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md#producer-callback
/// [FnOnce]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html
pub trait IndexedProducerCallback<T> {
    /// The type of value returned by this callback. Analogous to
    /// [`Output` from the `FnOnce` trait][Output].
    ///
    /// [Output]: https://doc.rust-lang.org/std/ops/trait.FnOnce.html#associatedtype.Output
    type Output;

    /// Invokes the callback with the given producer as argument. The
    /// key point of this trait is that this method is generic over
    /// `P`, and hence implementors must be defined for any producer.
    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = T>;
}
