use super::{Executor, IntoParallelIterator};

/// `FromParallelIterator` implements the creation of a collection
/// from a [`ParallelIterator`]. By implementing
/// `FromParallelIterator` for a given type, you define how it will be
/// created from an iterator.
///
/// `FromParallelIterator` is used through [`ParallelIterator`]'s [`collect()`] method.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`collect()`]: trait.ParallelIterator.html#method.collect
///
/// # Examples
///
/// Implementing `FromParallelIterator` for your type:
///
/// ```
/// use rayon::prelude::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<T: Send> FromParallelIterator<T> for BlackHole {
///     fn from_par_iter<I>(iterator: I) -> Self
///         where I: IntoParallelIterator<Item = T>
///     {
///         let iterator = iterator.into_par_iter();
///         BlackHole {
///             mass: iterator.count() * mem::size_of::<T>(),
///         }
///     }
/// }
///
/// let bh: BlackHole = (0i32..1000).into_par_iter().collect();
/// assert_eq!(bh.mass, 4000);
/// ```
pub trait FromParallelIterator<'a, I>: Send + Sized
where
    I: Send + 'a,
{
    type ExecutorItem2: Send + 'a;
    type ExecutorItem3: Send + 'a;

    /// Creates an instance of the collection from the parallel iterator `iterator`.
    ///
    /// If your collection is not naturally parallel, the easiest (and
    /// fastest) way to do this is often to collect `iterator` into a
    /// [`LinkedList`] or other intermediate data structure and then
    /// sequentially extend your collection. However, a more 'native'
    /// technique is to use the [`iterator.fold`] or
    /// [`iterator.fold_with`] methods to create the collection.
    /// Alternatively, if your collection is 'natively' parallel, you
    /// can use `iterator.for_each` to process each element in turn.
    ///
    /// [`LinkedList`]: https://doc.rust-lang.org/std/collections/struct.LinkedList.html
    /// [`iterator.fold`]: trait.ParallelIterator.html#method.fold
    /// [`iterator.fold_with`]: trait.ParallelIterator.html#method.fold_with
    /// [`iterator.for_each`]: trait.ParallelIterator.html#method.for_each
    fn from_par_iter<E, X>(executor: E, iterator: X) -> E::Result
    where
        E: Executor<'a, Self, Self::ExecutorItem2, Self::ExecutorItem3>,
        X: IntoParallelIterator<'a, Item = I>;
}

impl<'a> FromParallelIterator<'a, ()> for () {
    type ExecutorItem2 = ();
    type ExecutorItem3 = ();

    fn from_par_iter<E, X>(executor: E, iterator: X) -> E::Result
    where
        E: Executor<'a, Self>,
        X: IntoParallelIterator<'a, Item = ()>,
    {
        use crate::{iter::noop::NoOpConsumer, ParallelIterator};

        iterator.into_par_iter().drive(executor, NoOpConsumer)
    }
}
