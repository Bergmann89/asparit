use super::{Consumer, Reducer};

/// `ParallelExtend` extends an existing collection with items from a [`ParallelIterator`].
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
///
/// # Examples
///
/// Implementing `ParallelExtend` for your type:
///
/// ```
/// use rayon::prelude::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<T: Send> ParallelExtend<T> for BlackHole {
///     fn par_extend<I>(&mut self, par_iter: I)
///         where I: IntoParallelIterator<Item = T>
///     {
///         let par_iter = par_iter.into_par_iter();
///         self.mass += par_iter.count() * mem::size_of::<T>();
///     }
/// }
///
/// let mut bh = BlackHole { mass: 0 };
/// bh.par_extend(0i32..1000);
/// assert_eq!(bh.mass, 4000);
/// bh.par_extend(0i64..10);
/// assert_eq!(bh.mass, 4080);
/// ```
pub trait ParallelExtend<'a, I, T>: Send + Sized
where
    I: Send + 'a,
    T: Send,
    <Self::Consumer as Consumer<I>>::Reducer: Reducer<T>,
{
    type Consumer: Consumer<I, Result = T> + 'a;

    /// Creates a consumer that is used to handle the items from the iterator.
    fn into_consumer(self) -> Self::Consumer;

    /// Converts the result of the consumer into the final type
    fn map_result(inner: T) -> Self;
}
