use super::{
    Consumer, Executor, FromParallelIterator, IndexedProducerCallback, ProducerCallback, Reducer,
};

use crate::{
    inner::{
        cloned::Cloned,
        collect::Collect,
        copied::Copied,
        filter::Filter,
        for_each::ForEach,
        inspect::Inspect,
        map::Map,
        map_init::MapInit,
        map_with::MapWith,
        reduce::Reduce,
        try_for_each::{TryForEach, TryForEachInit, TryForEachWith},
        try_reduce::TryReduce,
        update::Update,
    },
    misc::Try,
};

/// Parallel version of the standard iterator trait.
///
/// The combinators on this trait are available on **all** parallel
/// iterators.  Additional methods can be found on the
/// [`IndexedParallelIterator`] trait: those methods are only
/// available for parallel iterators where the number of items is
/// known in advance (so, e.g., after invoking `filter`, those methods
/// become unavailable).
///
/// For examples of using parallel iterators, see [the docs on the
/// `iter` module][iter].
///
/// [iter]: index.html
/// [`IndexedParallelIterator`]: trait.IndexedParallelIterator.html
pub trait ParallelIterator<'a>: Sized + Send {
    /// The type of item that this parallel iterator produces.
    /// For example, if you use the [`for_each`] method, this is the type of
    /// item that your closure will be invoked with.
    ///
    /// [`for_each`]: #method.for_each
    type Item: Send;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method causes the iterator `self` to start producing
    /// items and to feed them to the consumer `consumer` one by one.
    /// It may split the consumer before doing so to create the
    /// opportunity to produce in parallel.
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method converts the iterator into a producer P and then
    /// invokes `callback.callback()` with P. Note that the type of
    /// this producer is not defined as part of the API, since
    /// `callback` must be defined generically for all producers. This
    /// allows the producer type to contain references; it also means
    /// that parallel iterators can adjust that type without causing a
    /// breaking change.
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// Returns the number of items produced by this iterator, if known
    /// statically. This can be used by consumers to trigger special fast
    /// paths. Therefore, if `Some(_)` is returned, this iterator must only
    /// use the (indexed) `Consumer` methods when driving a consumer, such
    /// as `split_at()`. Calling `UnindexedConsumer::split_off_left()` or
    /// other `UnindexedConsumer` methods -- or returning an inaccurate
    /// value -- may result in panics.
    ///
    /// This method is currently used to optimize `collect` for want
    /// of true Rust specialization; it may be removed when
    /// specialization is stable.
    fn len_hint_opt(&self) -> Option<usize> {
        None
    }

    /// Executes `operation` on each item produced by the iterator, in parallel.
    ///
    /// # Examples
    ///
    /// ```
    /// use asparit::*;
    ///
    /// (0..100).into_par_iter().for_each(|x| println!("{:?}", x));
    /// ```
    fn for_each<O>(self, operation: O) -> ForEach<Self, O>
    where
        O: Fn(Self::Item),
    {
        ForEach::new(self, operation)
    }

    /// Executes `operation` on the given `init` value with each item produced by
    /// the iterator, in parallel.
    ///
    /// The `init` value will be cloned only as needed to be paired with
    /// the group of items in each rayon job.  It does not require the type
    /// to be `Sync`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::mpsc::channel;
    /// use rayon::prelude::*;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// (0..5).into_par_iter().for_each_with(sender, |s, x| s.send(x).unwrap());
    ///
    /// let mut res: Vec<_> = receiver.iter().collect();
    ///
    /// res.sort();
    ///
    /// assert_eq!(&res[..], &[0, 1, 2, 3, 4])
    /// ```
    fn for_each_with<O, T>(self, init: T, operation: O) -> Collect<MapWith<Self, T, O>, ()>
    where
        O: Fn(&mut T, Self::Item) + Clone + Send + Sync + 'a,
        T: Clone + Send + 'a,
    {
        self.map_with(init, operation).collect()
    }

    /// Executes `operation` on a value returned by `init` with each item produced by
    /// the iterator, in parallel.
    ///
    /// The `init` function will be called only as needed for a value to be
    /// paired with the group of items in each rayon job.  There is no
    /// constraint on that returned type at all!
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::Rng;
    /// use rayon::prelude::*;
    ///
    /// let mut v = vec![0u8; 1_000_000];
    ///
    /// v.par_chunks_mut(1000)
    ///     .for_each_init(
    ///         || rand::thread_rng(),
    ///         |rng, chunk| rng.fill(chunk),
    ///     );
    ///
    /// // There's a remote chance that this will fail...
    /// for i in 0u8..=255 {
    ///     assert!(v.contains(&i));
    /// }
    /// ```
    fn for_each_init<O, S, T>(self, init: S, operation: O) -> Collect<MapInit<Self, S, O>, ()>
    where
        O: Fn(&mut T, Self::Item) + Clone + Sync + Send + 'a,
        S: Fn() -> T + Clone + Sync + Send + 'a,
    {
        self.map_init(init, operation).collect()
    }

    /// Executes a fallible `operation` on each item produced by the iterator, in parallel.
    ///
    /// If the `operation` returns `Result::Err` or `Option::None`, we will attempt to
    /// stop processing the rest of the items in the iterator as soon as
    /// possible, and we will return that terminating value.  Otherwise, we will
    /// return an empty `Result::Ok(())` or `Option::Some(())`.  If there are
    /// multiple errors in parallel, it is not specified which will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use std::io::{self, Write};
    ///
    /// // This will stop iteration early if there's any write error, like
    /// // having piped output get closed on the other end.
    /// (0..100).into_par_iter()
    ///     .try_for_each(|x| writeln!(io::stdout(), "{:?}", x))
    ///     .expect("expected no write errors");
    /// ```
    fn try_for_each<O, T>(self, operation: O) -> TryForEach<Self, O>
    where
        O: Fn(Self::Item) -> T + Clone + Sync + Send,
        T: Try<Ok = ()> + Send,
    {
        TryForEach::new(self, operation)
    }

    /// Executes a fallible `operation` on the given `init` value with each item
    /// produced by the iterator, in parallel.
    ///
    /// This combines the `init` semantics of [`for_each_with()`] and the
    /// failure semantics of [`try_for_each()`].
    ///
    /// [`for_each_with()`]: #method.for_each_with
    /// [`try_for_each()`]: #method.try_for_each
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::mpsc::channel;
    /// use rayon::prelude::*;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// (0..5).into_par_iter()
    ///     .try_for_each_with(sender, |s, x| s.send(x))
    ///     .expect("expected no send errors");
    ///
    /// let mut res: Vec<_> = receiver.iter().collect();
    ///
    /// res.sort();
    ///
    /// assert_eq!(&res[..], &[0, 1, 2, 3, 4])
    /// ```
    fn try_for_each_with<O, S, T>(self, init: S, operation: O) -> TryForEachWith<Self, S, O>
    where
        S: Clone + Send + 'a,
        O: Fn(&mut S, Self::Item) -> T + Clone + Sync + Send + 'a,
        T: Try<Ok = ()> + Send + 'a,
    {
        TryForEachWith::new(self, init, operation)
    }

    /// Executes a fallible `operation` on a value returned by `init` with each item
    /// produced by the iterator, in parallel.
    ///
    /// This combines the `init` semantics of [`for_each_init()`] and the
    /// failure semantics of [`try_for_each()`].
    ///
    /// [`for_each_init()`]: #method.for_each_init
    /// [`try_for_each()`]: #method.try_for_each
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::Rng;
    /// use rayon::prelude::*;
    ///
    /// let mut v = vec![0u8; 1_000_000];
    ///
    /// v.par_chunks_mut(1000)
    ///     .try_for_each_init(
    ///         || rand::thread_rng(),
    ///         |rng, chunk| rng.try_fill(chunk),
    ///     )
    ///     .expect("expected no rand errors");
    ///
    /// // There's a remote chance that this will fail...
    /// for i in 0u8..=255 {
    ///     assert!(v.contains(&i));
    /// }
    /// ```
    fn try_for_each_init<O, S, T, U>(self, init: S, operation: O) -> TryForEachInit<Self, S, O>
    where
        O: Fn(&mut U, Self::Item) -> T + Clone + Sync + Send + 'a,
        S: Fn() -> U + Clone + Send + Sync + 'a,
        T: Try<Ok = ()> + Send + 'a,
    {
        TryForEachInit::new(self, init, operation)
    }

    /// Applies `operation` to each item of this iterator, producing a new
    /// iterator with the results.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut par_iter = (0..5).into_par_iter().map(|x| x * 2);
    ///
    /// let doubles: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&doubles[..], &[0, 2, 4, 6, 8]);
    /// ```
    fn map<O, T>(self, operation: O) -> Map<Self, O>
    where
        O: Fn(Self::Item) -> T + Sync + Send,
        T: Send,
    {
        Map::new(self, operation)
    }

    /// Applies `operation` to the given `init` value with each item of this
    /// iterator, producing a new iterator with the results.
    ///
    /// The `init` value will be cloned only as needed to be paired with
    /// the group of items in each rayon job.  It does not require the type
    /// to be `Sync`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::mpsc::channel;
    /// use rayon::prelude::*;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// let a: Vec<_> = (0..5)
    ///     .into_par_iter()            // iterating over i32
    ///     .map_with(sender, |s, x| {
    ///         s.send(x).unwrap();     // sending i32 values through the channel
    ///         x                       // returning i32
    ///     })
    ///     .collect();                 // collecting the returned values into a vector
    ///
    /// let mut b: Vec<_> = receiver
    ///     .iter()                     // iterating over the values in the channel
    ///     .collect();                 // and collecting them
    /// b.sort();
    ///
    /// assert_eq!(a, b);
    /// ```
    fn map_with<O, T, S>(self, init: S, operation: O) -> MapWith<Self, S, O>
    where
        O: Fn(&mut S, Self::Item) -> T + Clone + Sync + Send,
        S: Send + Clone,
        T: Send,
    {
        MapWith::new(self, init, operation)
    }

    /// Applies `operation` to a value returned by `init` with each item of this
    /// iterator, producing a new iterator with the results.
    ///
    /// The `init` function will be called only as needed for a value to be
    /// paired with the group of items in each rayon job.  There is no
    /// constraint on that returned type at all!
    ///
    /// # Examples
    ///
    /// ```
    /// use rand::Rng;
    /// use rayon::prelude::*;
    ///
    /// let a: Vec<_> = (1i32..1_000_000)
    ///     .into_par_iter()
    ///     .map_init(
    ///         || rand::thread_rng(),  // get the thread-local RNG
    ///         |rng, x| if rng.gen() { // randomly negate items
    ///             -x
    ///         } else {
    ///             x
    ///         },
    ///     ).collect();
    ///
    /// // There's a remote chance that this will fail...
    /// assert!(a.iter().any(|&x| x < 0));
    /// assert!(a.iter().any(|&x| x > 0));
    /// ```
    fn map_init<O, T, S, U>(self, init: S, operation: O) -> MapInit<Self, S, O>
    where
        O: Fn(&mut U, Self::Item) -> T + Sync + Send,
        S: Fn() -> U + Sync + Send,
        T: Send,
    {
        MapInit::new(self, init, operation)
    }

    /// Creates an iterator which clones all of its elements.  This may be
    /// useful when you have an iterator over `&T`, but you need `T`, and
    /// that type implements `Clone`. See also [`copied()`].
    ///
    /// [`copied()`]: #method.copied
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3];
    ///
    /// let v_cloned: Vec<_> = a.par_iter().cloned().collect();
    ///
    /// // cloned is the same as .map(|&x| x), for integers
    /// let v_map: Vec<_> = a.par_iter().map(|&x| x).collect();
    ///
    /// assert_eq!(v_cloned, vec![1, 2, 3]);
    /// assert_eq!(v_map, vec![1, 2, 3]);
    /// ```
    fn cloned<T>(self) -> Cloned<Self>
    where
        T: Clone + Send + 'a,
        Self: ParallelIterator<'a, Item = &'a T>,
    {
        Cloned::new(self)
    }

    /// Creates an iterator which copies all of its elements.  This may be
    /// useful when you have an iterator over `&T`, but you need `T`, and
    /// that type implements `Copy`. See also [`cloned()`].
    ///
    /// [`cloned()`]: #method.cloned
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3];
    ///
    /// let v_copied: Vec<_> = a.par_iter().copied().collect();
    ///
    /// // copied is the same as .map(|&x| x), for integers
    /// let v_map: Vec<_> = a.par_iter().map(|&x| x).collect();
    ///
    /// assert_eq!(v_copied, vec![1, 2, 3]);
    /// assert_eq!(v_map, vec![1, 2, 3]);
    /// ```
    fn copied<T>(self) -> Copied<Self>
    where
        T: Copy + Send + 'a,
        Self: ParallelIterator<'a, Item = &'a T>,
    {
        Copied::new(self)
    }

    /// Applies `operation` to a reference to each item of this iterator,
    /// producing a new iterator passing through the original items.  This is
    /// often useful for debugging to see what's happening in iterator stages.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 4, 2, 3];
    ///
    /// // this iterator sequence is complex.
    /// let sum = a.par_iter()
    ///             .cloned()
    ///             .filter(|&x| x % 2 == 0)
    ///             .reduce(|| 0, |sum, i| sum + i);
    ///
    /// println!("{}", sum);
    ///
    /// // let's add some inspect() calls to investigate what's happening
    /// let sum = a.par_iter()
    ///             .cloned()
    ///             .inspect(|x| println!("about to filter: {}", x))
    ///             .filter(|&x| x % 2 == 0)
    ///             .inspect(|x| println!("made it through filter: {}", x))
    ///             .reduce(|| 0, |sum, i| sum + i);
    ///
    /// println!("{}", sum);
    /// ```
    fn inspect<O>(self, operation: O) -> Inspect<Self, O>
    where
        O: Fn(&Self::Item) + Clone + Send + 'a,
    {
        Inspect::new(self, operation)
    }

    /// Mutates each item of this iterator before yielding it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let par_iter = (0..5).into_par_iter().update(|x| {*x *= 2;});
    ///
    /// let doubles: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&doubles[..], &[0, 2, 4, 6, 8]);
    /// ```
    fn update<O>(self, operation: O) -> Update<Self, O>
    where
        O: Fn(&mut Self::Item) + Clone + Send + 'a,
    {
        Update::new(self, operation)
    }

    /// Applies `operation` to each item of this iterator, producing a new
    /// iterator with only the items that gave `true` results.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut par_iter = (0..10).into_par_iter().filter(|x| x % 2 == 0);
    ///
    /// let even_numbers: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&even_numbers[..], &[0, 2, 4, 6, 8]);
    /// ```
    fn filter<O>(self, operation: O) -> Filter<Self, O>
    where
        O: Fn(&Self::Item) -> bool + Clone + Send + 'a,
    {
        Filter::new(self, operation)
    }

    /// Reduces the items in the iterator into one item using `operation`.
    /// The argument `identity` should be a closure that can produce
    /// "identity" value which may be inserted into the sequence as
    /// needed to create opportunities for parallel execution. So, for
    /// example, if you are doing a summation, then `identity()` ought
    /// to produce something that represents the zero for your type
    /// (but consider just calling `sum()` in that case).
    ///
    /// # Examples
    ///
    /// ```
    /// // Iterate over a sequence of pairs `(x0, y0), ..., (xN, yN)`
    /// // and use reduce to compute one pair `(x0 + ... + xN, y0 + ... + yN)`
    /// // where the first/second elements are summed separately.
    /// use rayon::prelude::*;
    /// let sums = [(0, 1), (5, 6), (16, 2), (8, 9)]
    ///            .par_iter()        // iterating over &(i32, i32)
    ///            .cloned()          // iterating over (i32, i32)
    ///            .reduce(|| (0, 0), // the "identity" is 0 in both columns
    ///                    |a, b| (a.0 + b.0, a.1 + b.1));
    /// assert_eq!(sums, (0 + 5 + 16 + 8, 1 + 6 + 2 + 9));
    /// ```
    ///
    /// **Note:** unlike a sequential `fold` operation, the order in
    /// which `operation` will be applied to reduce the result is not fully
    /// specified. So `operation` should be [associative] or else the results
    /// will be non-deterministic. And of course `identity()` should
    /// produce a true identity.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    fn reduce<S, O>(self, identity: S, operation: O) -> Reduce<Self, S, O>
    where
        S: Fn() -> Self::Item + Clone + Send + 'a,
        O: Fn(Self::Item, Self::Item) -> Self::Item + Clone + Send + 'a,
    {
        Reduce::new(self, identity, operation)
    }

    /// Reduces the items in the iterator into one item using a fallible `operation`.
    /// The `identity` argument is used the same way as in [`reduce()`].
    ///
    /// [`reduce()`]: #method.reduce
    ///
    /// If a `Result::Err` or `Option::None` item is found, or if `operation` reduces
    /// to one, we will attempt to stop processing the rest of the items in the
    /// iterator as soon as possible, and we will return that terminating value.
    /// Otherwise, we will return the final reduced `Result::Ok(T)` or
    /// `Option::Some(T)`.  If there are multiple errors in parallel, it is not
    /// specified which will be returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// // Compute the sum of squares, being careful about overflow.
    /// fn sum_squares<I: IntoParallelIterator<Item = i32>>(iter: I) -> Option<i32> {
    ///     iter.into_par_iter()
    ///         .map(|i| i.checked_mul(i))            // square each item,
    ///         .try_reduce(|| 0, i32::checked_add)   // and add them up!
    /// }
    /// assert_eq!(sum_squares(0..5), Some(0 + 1 + 4 + 9 + 16));
    ///
    /// // The sum might overflow
    /// assert_eq!(sum_squares(0..10_000), None);
    ///
    /// // Or the squares might overflow before it even reaches `try_reduce`
    /// assert_eq!(sum_squares(1_000_000..1_000_001), None);
    /// ```
    fn try_reduce<S, O, T>(self, identity: S, operation: O) -> TryReduce<Self, S, O>
    where
        S: Fn() -> T + Sync + Send,
        O: Fn(T, T) -> Self::Item + Sync + Send,
        Self::Item: Try<Ok = T>,
    {
        TryReduce::new(self, identity, operation)
    }

    /// Creates a fresh collection containing all the elements produced
    /// by this parallel iterator.
    ///
    /// You may prefer [`collect_into_vec()`] implemented on
    /// [`IndexedParallelIterator`], if your underlying iterator also implements
    /// it. [`collect_into_vec()`] allocates efficiently with precise knowledge
    /// of how many elements the iterator contains, and even allows you to reuse
    /// an existing vector's backing store rather than allocating a fresh vector.
    ///
    /// [`IndexedParallelIterator`]: trait.IndexedParallelIterator.html
    /// [`collect_into_vec()`]:
    ///     trait.IndexedParallelIterator.html#method.collect_into_vec
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let sync_vec: Vec<_> = (0..100).into_iter().collect();
    ///
    /// let async_vec: Vec<_> = (0..100).into_par_iter().collect();
    ///
    /// assert_eq!(sync_vec, async_vec);
    /// ```
    fn collect<T>(self) -> Collect<Self, T>
    where
        T: FromParallelIterator<Self::Item>,
    {
        Collect::new(self)
    }
}

/// An iterator that supports "random access" to its data, meaning
/// that you can split it at arbitrary indices and draw data from
/// those points.
///
/// **Note:** Not implemented for `u64`, `i64`, `u128`, or `i128` ranges
pub trait IndexedParallelIterator<'a>: ParallelIterator<'a> {
    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method causes the iterator `self` to start producing
    /// items and to feed them to the consumer `consumer` one by one.
    /// It may split the consumer before doing so to create the
    /// opportunity to produce in parallel. If a split does happen, it
    /// will inform the consumer of the index where the split should
    /// occur (unlike `ParallelIterator::drive_unindexed()`).
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send;

    /// Internal method used to define the behavior of this parallel
    /// iterator. You should not need to call this directly.
    ///
    /// This method converts the iterator into a producer P and then
    /// invokes `callback.callback()` with P. Note that the type of
    /// this producer is not defined as part of the API, since
    /// `callback` must be defined generically for all producers. This
    /// allows the producer type to contain references; it also means
    /// that parallel iterators can adjust that type without causing a
    /// breaking change.
    ///
    /// See the [README] for more details on the internals of parallel
    /// iterators.
    ///
    /// [README]: README.md
    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>;

    /// Produces an exact count of how many items this iterator will
    /// produce, presuming no panic occurs.
    ///
    /// # Examples
    ///
    /// ```
    /// use asparit::*;
    ///
    /// let par_iter = (0..100).into_par_iter().zip(vec![0; 10]);
    /// assert_eq!(par_iter.len(), 10);
    ///
    /// let vec: Vec<_> = par_iter.collect();
    /// assert_eq!(vec.len(), 10);
    /// ```
    fn len_hint(&self) -> usize;
}
