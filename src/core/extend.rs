use super::{Consumer, Driver, Executor, IntoParallelIterator, ParallelIterator, Reducer};

/// `ParallelExtend` extends an existing collection with items from a [`ParallelIterator`].
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
///
/// # Examples
///
/// Implementing `ParallelExtend` for your type:
///
/// ```
/// use asparit::*;
/// use std::mem;
///
/// struct BlackHole {
///     mass: usize,
/// }
///
/// impl<'a, I> ParallelExtend<'a, I, BlackHoleFolder<'a>> for &'a mut BlackHole
/// where
///     I: Send + 'a,
/// {
///     type Consumer = BlackHoleConsumer<'a>;
///
///     fn into_consumer(self) -> Self::Consumer {
///         BlackHoleConsumer(Some(self))
///     }
///
///     fn map_result(ret: BlackHoleFolder<'a>) -> Self {
///         let bh = ret.bh.unwrap();
///
///         bh.mass += ret.mass;
///
///         bh
///     }
/// }
///
/// struct BlackHoleConsumer<'a>(Option<&'a mut BlackHole>);
///
/// impl WithSetup for BlackHoleConsumer<'_> {}
///
/// impl<'a, I> Consumer<I> for BlackHoleConsumer<'a> {
///     type Folder = BlackHoleFolder<'a>;
///     type Reducer = BlackHoleReducer;
///     type Result = BlackHoleFolder<'a>;
///
///     fn split(self) -> (Self, Self, Self::Reducer) {
///         (
///             BlackHoleConsumer(self.0),
///             BlackHoleConsumer(None),
///             BlackHoleReducer,
///         )
///     }
///
///     fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
///         (
///             BlackHoleConsumer(self.0),
///             BlackHoleConsumer(None),
///             BlackHoleReducer,
///         )
///     }
///
///     fn into_folder(self) -> Self::Folder {
///         BlackHoleFolder {
///             bh: self.0,
///             mass: 0,
///         }
///     }
/// }
///
/// struct BlackHoleFolder<'a> {
///     bh: Option<&'a mut BlackHole>,
///     mass: usize,
/// }
///
/// impl<'a, I> Folder<I> for BlackHoleFolder<'a> {
///     type Result = Self;
///
///     fn consume(mut self, _item: I) -> Self {
///         self.mass += std::mem::size_of::<I>();
///
///         self
///     }
///
///     fn consume_iter<X>(mut self, iter: X) -> Self
///     where
///         X: IntoIterator<Item = I>,
///     {
///         self.mass += iter.into_iter().count() * std::mem::size_of::<I>();
///
///         self
///     }
///
///     fn complete(self) -> Self::Result {
///         self
///     }
/// }
///
/// struct BlackHoleReducer;
///
/// impl<'a> Reducer<BlackHoleFolder<'a>> for BlackHoleReducer {
///     fn reduce(
///         self,
///         left: BlackHoleFolder<'a>,
///         right: BlackHoleFolder<'a>,
///     ) -> BlackHoleFolder<'a> {
///         BlackHoleFolder {
///             bh: left.bh.or(right.bh),
///             mass: left.mass + right.mass,
///         }
///     }
/// }
///
/// let mut bh = BlackHole { mass: 0 };
///
/// bh.par_extend(0i32..1000).exec_with(SimpleExecutor);
/// assert_eq!(bh.mass, 4000);
///
/// bh.par_extend(0i64..10).exec_with(SimpleExecutor);
/// assert_eq!(bh.mass, 4080);
/// ```
pub trait ParallelExtend<'a, I, T>: Send + Sized
where
    I: Send + 'a,
    T: Send,
    <Self::Consumer as Consumer<I>>::Reducer: Reducer<T>,
{
    type Consumer: Consumer<I, Result = T> + 'a;

    fn par_extend<X>(self, iter: X) -> ParallelExtendDriver<'a, X::Iter, Self, T>
    where
        X: IntoParallelIterator<'a, Item = I>,
    {
        ParallelExtendDriver {
            iterator: iter.into_par_iter(),
            consumer: self.into_consumer(),
        }
    }

    /// Creates a consumer that is used to handle the items from the iterator.
    fn into_consumer(self) -> Self::Consumer;

    /// Converts the result of the consumer into the final type
    fn map_result(inner: <Self::Consumer as Consumer<I>>::Result) -> Self;
}

pub struct ParallelExtendDriver<'a, X, S, T>
where
    X: ParallelIterator<'a>,
    S: ParallelExtend<'a, X::Item, T>,
    <S::Consumer as Consumer<X::Item>>::Reducer: Reducer<T>,
    T: Send,
{
    iterator: X,
    consumer: S::Consumer,
}

impl<'a, X, S, T> Driver<'a, S, T> for ParallelExtendDriver<'a, X, S, T>
where
    X: ParallelIterator<'a>,
    S: ParallelExtend<'a, X::Item, T> + Send + 'a,
    <S::Consumer as Consumer<X::Item>>::Reducer: Reducer<T> + Send,
    T: Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, S, T>,
    {
        let iterator = self.iterator;
        let consumer = self.consumer;
        let executor = executor.into_inner();

        let ret = iterator.drive(executor, consumer);

        E::map(ret, S::map_result)
    }
}
