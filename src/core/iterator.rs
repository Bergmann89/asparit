use std::cmp::{Ord, Ordering};
use std::iter::IntoIterator;

use super::{Consumer, Executor, FromParallelIterator, IntoParallelIterator, Reducer};

use crate::{
    iter::{
        chain::Chain,
        cloned::Cloned,
        collect::Collect,
        copied::Copied,
        count::Count,
        filter::Filter,
        filter_map::FilterMap,
        find::{All, Any, Find, FindMap, FindMatch},
        flatten::{FlatMapIter, FlattenIter},
        fold::{Fold, FoldWith},
        for_each::ForEach,
        inspect::Inspect,
        intersperse::Intersperse,
        map::Map,
        map_init::MapInit,
        map_with::MapWith,
        max::{Max, MaxBy, MaxByKey},
        min::{Min, MinBy, MinByKey},
        panic_fuse::PanicFuse,
        partition::{Partition, PartitionMap},
        product::Product,
        reduce::{Reduce, ReduceWith},
        splits::Splits,
        sum::Sum,
        try_fold::{TryFold, TryFoldWith},
        try_for_each::{TryForEach, TryForEachInit, TryForEachWith},
        try_reduce::{TryReduce, TryReduceWith},
        unzip::Unzip,
        update::Update,
        while_some::WhileSome,
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
    type Item: Send + 'a;

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
        D: Send + 'a,
        R: Reducer<D> + Send + 'a;

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
        O: Fn(&mut T, Self::Item) + Clone + Send + 'a,
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
        O: Fn(&mut T, Self::Item) + Clone + Send + 'a,
        S: Fn() -> T + Clone + Send + 'a,
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
        O: Fn(Self::Item) -> T + Clone + Send,
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
        O: Fn(&mut S, Self::Item) -> T + Clone + Send + 'a,
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
        O: Fn(&mut U, Self::Item) -> T + Clone + Send + 'a,
        S: Fn() -> U + Clone + Send + 'a,
        T: Try<Ok = ()> + Send + 'a,
    {
        TryForEachInit::new(self, init, operation)
    }

    /// Counts the number of items in this parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let count = (0..100).into_par_iter().count();
    ///
    /// assert_eq!(count, 100);
    /// ```
    fn count(self) -> Count<Self> {
        Count::new(self)
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
        O: Fn(Self::Item) -> T + Clone + Send,
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
        O: Fn(&mut S, Self::Item) -> T + Clone + Send,
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
        O: Fn(&mut U, Self::Item) -> T + Send,
        S: Fn() -> U + Send,
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

    /// Applies `operation` to each item of this iterator to get an `Option`,
    /// producing a new iterator with only the items from `Some` results.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let mut par_iter = (0..10).into_par_iter()
    ///                         .filter_map(|x| {
    ///                             if x % 2 == 0 { Some(x * 3) }
    ///                             else { None }
    ///                         });
    ///
    /// let even_numbers: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&even_numbers[..], &[0, 6, 12, 18, 24]);
    /// ```
    fn filter_map<O, S>(self, operation: O) -> FilterMap<Self, O>
    where
        O: Fn(Self::Item) -> Option<S> + Clone + Send + 'a,
    {
        FilterMap::new(self, operation)
    }

    /// Applies `operation` to each item of this iterator to get nested serial iterators,
    /// producing a new parallel iterator that flattens these back into one.
    ///
    /// # `flat_map_iter` versus `flat_map`
    ///
    /// These two methods are similar but behave slightly differently. With [`flat_map`],
    /// each of the nested iterators must be a parallel iterator, and they will be further
    /// split up with nested parallelism. With `flat_map_iter`, each nested iterator is a
    /// sequential `Iterator`, and we only parallelize _between_ them, while the items
    /// produced by each nested iterator are processed sequentially.
    ///
    /// When choosing between these methods, consider whether nested parallelism suits the
    /// potential iterators at hand. If there's little computation involved, or its length
    /// is much less than the outer parallel iterator, then it may perform better to avoid
    /// the overhead of parallelism, just flattening sequentially with `flat_map_iter`.
    /// If there is a lot of computation, potentially outweighing the outer parallel
    /// iterator, then the nested parallelism of `flat_map` may be worthwhile.
    ///
    /// [`flat_map`]: #method.flat_map
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use std::cell::RefCell;
    ///
    /// let a = [[1, 2], [3, 4], [5, 6], [7, 8]];
    ///
    /// let par_iter = a.par_iter().flat_map_iter(|a| {
    ///     // The serial iterator doesn't have to be thread-safe, just its items.
    ///     let cell_iter = RefCell::new(a.iter().cloned());
    ///     std::iter::from_fn(move || cell_iter.borrow_mut().next())
    /// });
    ///
    /// let vec: Vec<_> = par_iter.collect();
    ///
    /// assert_eq!(&vec[..], &[1, 2, 3, 4, 5, 6, 7, 8]);
    /// ```
    fn flat_map_iter<O, SI>(self, operation: O) -> FlatMapIter<Self, O>
    where
        O: Fn(Self::Item) -> SI + Clone + Send + 'a,
        SI: IntoIterator,
        SI::Item: Send,
    {
        FlatMapIter::new(self, operation)
    }

    /// An adaptor that flattens serial-iterable `Item`s into one large iterator.
    ///
    /// See also [`flatten`](#method.flatten) and the analagous comparison of
    /// [`flat_map_iter` versus `flat_map`](#flat_map_iter-versus-flat_map).
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let x: Vec<Vec<_>> = vec![vec![1, 2], vec![3, 4]];
    /// let iters: Vec<_> = x.into_iter().map(Vec::into_iter).collect();
    /// let y: Vec<_> = iters.into_par_iter().flatten_iter().collect();
    ///
    /// assert_eq!(y, vec![1, 2, 3, 4]);
    /// ```
    fn flatten_iter(self) -> FlattenIter<Self>
    where
        Self::Item: IntoIterator + Send,
        <Self::Item as IntoIterator>::Item: Send,
    {
        FlattenIter::new(self)
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

    /// Reduces the items in the iterator into one item using `operation`.
    /// If the iterator is empty, `None` is returned; otherwise,
    /// `Some` is returned.
    ///
    /// This version of `reduce` is simple but somewhat less
    /// efficient. If possible, it is better to call `reduce()`, which
    /// requires an identity element.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// let sums = [(0, 1), (5, 6), (16, 2), (8, 9)]
    ///            .par_iter()        // iterating over &(i32, i32)
    ///            .cloned()          // iterating over (i32, i32)
    ///            .reduce_with(|a, b| (a.0 + b.0, a.1 + b.1))
    ///            .unwrap();
    /// assert_eq!(sums, (0 + 5 + 16 + 8, 1 + 6 + 2 + 9));
    /// ```
    ///
    /// **Note:** unlike a sequential `fold` operation, the order in
    /// which `operation` will be applied to reduce the result is not fully
    /// specified. So `operation` should be [associative] or else the results
    /// will be non-deterministic.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    fn reduce_with<O>(self, operation: O) -> ReduceWith<Self, O>
    where
        O: Fn(Self::Item, Self::Item) -> Self::Item + Clone + Send + 'a,
    {
        ReduceWith::new(self, operation)
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
        Self::Item: Try<Ok = T>,
        S: Fn() -> T + Clone + Send + 'a,
        O: Fn(T, T) -> Self::Item + Clone + Send + 'a,
    {
        TryReduce::new(self, identity, operation)
    }

    /// Reduces the items in the iterator into one item using a fallible `operation`.
    ///
    /// Like [`reduce_with()`], if the iterator is empty, `None` is returned;
    /// otherwise, `Some` is returned.  Beyond that, it behaves like
    /// [`try_reduce()`] for handling `Err`/`None`.
    ///
    /// [`reduce_with()`]: #method.reduce_with
    /// [`try_reduce()`]: #method.try_reduce
    ///
    /// For instance, with `Option` items, the return value may be:
    /// - `None`, the iterator was empty
    /// - `Some(None)`, we stopped after encountering `None`.
    /// - `Some(Some(x))`, the entire iterator reduced to `x`.
    ///
    /// With `Result` items, the nesting is more obvious:
    /// - `None`, the iterator was empty
    /// - `Some(Err(e))`, we stopped after encountering an error `e`.
    /// - `Some(Ok(x))`, the entire iterator reduced to `x`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let files = ["/dev/null", "/does/not/exist"];
    ///
    /// // Find the biggest file
    /// files.into_par_iter()
    ///     .map(|path| std::fs::metadata(path).map(|m| (path, m.len())))
    ///     .try_reduce_with(|a, b| {
    ///         Ok(if a.1 >= b.1 { a } else { b })
    ///     })
    ///     .expect("Some value, since the iterator is not empty")
    ///     .expect_err("not found");
    /// ```
    fn try_reduce_with<O, T>(self, operation: O) -> TryReduceWith<Self, O>
    where
        Self::Item: Try<Ok = T>,
        O: Fn(T, T) -> Self::Item + Clone + Send + 'a,
    {
        TryReduceWith::new(self, operation)
    }

    /// Parallel fold is similar to sequential fold except that the
    /// sequence of items may be subdivided before it is
    /// folded. Consider a list of numbers like `22 3 77 89 46`. If
    /// you used sequential fold to add them (`fold(0, |a,b| a+b)`,
    /// you would wind up first adding 0 + 22, then 22 + 3, then 25 +
    /// 77, and so forth. The **parallel fold** works similarly except
    /// that it first breaks up your list into sublists, and hence
    /// instead of yielding up a single sum at the end, it yields up
    /// multiple sums. The number of results is nondeterministic, as
    /// is the point where the breaks occur.
    ///
    /// So if did the same parallel fold (`fold(0, |a,b| a+b)`) on
    /// our example list, we might wind up with a sequence of two numbers,
    /// like so:
    ///
    /// ```notrust
    /// 22 3 77 89 46
    ///       |     |
    ///     102   135
    /// ```
    ///
    /// Or perhaps these three numbers:
    ///
    /// ```notrust
    /// 22 3 77 89 46
    ///       |  |  |
    ///     102 89 46
    /// ```
    ///
    /// In general, Rayon will attempt to find good breaking points
    /// that keep all of your cores busy.
    ///
    /// ### Fold versus reduce
    ///
    /// The `fold()` and `reduce()` methods each take an identity element
    /// and a combining function, but they operate rather differently.
    ///
    /// `reduce()` requires that the identity function has the same
    /// type as the things you are iterating over, and it fully
    /// reduces the list of items into a single item. So, for example,
    /// imagine we are iterating over a list of bytes `bytes: [128_u8,
    /// 64_u8, 64_u8]`. If we used `bytes.reduce(|| 0_u8, |a: u8, b:
    /// u8| a + b)`, we would get an overflow. This is because `0`,
    /// `a`, and `b` here are all bytes, just like the numbers in the
    /// list (I wrote the types explicitly above, but those are the
    /// only types you can use). To avoid the overflow, we would need
    /// to do something like `bytes.map(|b| b as u32).reduce(|| 0, |a,
    /// b| a + b)`, in which case our result would be `256`.
    ///
    /// In contrast, with `fold()`, the identity function does not
    /// have to have the same type as the things you are iterating
    /// over, and you potentially get back many results. So, if we
    /// continue with the `bytes` example from the previous paragraph,
    /// we could do `bytes.fold(|| 0_u32, |a, b| a + (b as u32))` to
    /// convert our bytes into `u32`. And of course we might not get
    /// back a single sum.
    ///
    /// There is a more subtle distinction as well, though it's
    /// actually implied by the above points. When you use `reduce()`,
    /// your reduction function is sometimes called with values that
    /// were never part of your original parallel iterator (for
    /// example, both the left and right might be a partial sum). With
    /// `fold()`, in contrast, the left value in the fold function is
    /// always the accumulator, and the right value is always from
    /// your original sequence.
    ///
    /// ### Fold vs Map/Reduce
    ///
    /// Fold makes sense if you have some operation where it is
    /// cheaper to create groups of elements at a time. For example,
    /// imagine collecting characters into a string. If you were going
    /// to use map/reduce, you might try this:
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let s =
    ///     ['a', 'b', 'c', 'd', 'e']
    ///     .par_iter()
    ///     .map(|c: &char| format!("{}", c))
    ///     .reduce(|| String::new(),
    ///             |mut a: String, b: String| { a.push_str(&b); a });
    ///
    /// assert_eq!(s, "abcde");
    /// ```
    ///
    /// Because reduce produces the same type of element as its input,
    /// you have to first map each character into a string, and then
    /// you can reduce them. This means we create one string per
    /// element in our iterator -- not so great. Using `fold`, we can
    /// do this instead:
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let s =
    ///     ['a', 'b', 'c', 'd', 'e']
    ///     .par_iter()
    ///     .fold(|| String::new(),
    ///             |mut s: String, c: &char| { s.push(*c); s })
    ///     .reduce(|| String::new(),
    ///             |mut a: String, b: String| { a.push_str(&b); a });
    ///
    /// assert_eq!(s, "abcde");
    /// ```
    ///
    /// Now `fold` will process groups of our characters at a time,
    /// and we only make one string per group. We should wind up with
    /// some small-ish number of strings roughly proportional to the
    /// number of CPUs you have (it will ultimately depend on how busy
    /// your processors are). Note that we still need to do a reduce
    /// afterwards to combine those groups of strings into a single
    /// string.
    ///
    /// You could use a similar trick to save partial results (e.g., a
    /// cache) or something similar.
    ///
    /// ### Combining fold with other operations
    ///
    /// You can combine `fold` with `reduce` if you want to produce a
    /// single value. This is then roughly equivalent to a map/reduce
    /// combination in effect:
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let bytes = 0..22_u8;
    /// let sum = bytes.into_par_iter()
    ///                .fold(|| 0_u32, |a: u32, b: u8| a + (b as u32))
    ///                .sum::<u32>();
    ///
    /// assert_eq!(sum, (0..22).sum()); // compare to sequential
    /// ```
    fn fold<S, O, U>(self, init: S, operation: O) -> Fold<Self, S, O>
    where
        S: Fn() -> U + Clone + Send + 'a,
        O: Fn(U, Self::Item) -> U + Clone + Send + 'a,
        U: Send,
    {
        Fold::new(self, init, operation)
    }

    /// Applies `operation` to the given `init` value with each item of this
    /// iterator, finally producing the value for further use.
    ///
    /// This works essentially like `fold(|| init.clone(), operation)`, except
    /// it doesn't require the `init` type to be `Sync`, nor any other form
    /// of added synchronization.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let bytes = 0..22_u8;
    /// let sum = bytes.into_par_iter()
    ///                .fold_with(0_u32, |a: u32, b: u8| a + (b as u32))
    ///                .sum::<u32>();
    ///
    /// assert_eq!(sum, (0..22).sum()); // compare to sequential
    /// ```
    fn fold_with<U, O>(self, init: U, operation: O) -> FoldWith<Self, U, O>
    where
        U: Clone + Send + 'a,
        O: Fn(U, Self::Item) -> U + Clone + Send + 'a,
    {
        FoldWith::new(self, init, operation)
    }

    /// Performs a fallible parallel fold.
    ///
    /// This is a variation of [`fold()`] for operations which can fail with
    /// `Option::None` or `Result::Err`.  The first such failure stops
    /// processing the local set of items, without affecting other folds in the
    /// iterator's subdivisions.
    ///
    /// Often, `try_fold()` will be followed by [`try_reduce()`]
    /// for a final reduction and global short-circuiting effect.
    ///
    /// [`fold()`]: #method.fold
    /// [`try_reduce()`]: #method.try_reduce
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let bytes = 0..22_u8;
    /// let sum = bytes.into_par_iter()
    ///                .try_fold(|| 0_u32, |a: u32, b: u8| a.checked_add(b as u32))
    ///                .try_reduce(|| 0, u32::checked_add);
    ///
    /// assert_eq!(sum, Some((0..22).sum())); // compare to sequential
    /// ```
    fn try_fold<S, O, U, T>(self, init: S, operation: O) -> TryFold<Self, S, O, T>
    where
        S: Fn() -> U + Clone + Send + 'a,
        O: Fn(U, Self::Item) -> T + Clone + Send + 'a,
        T: Try<Ok = U> + Send,
    {
        TryFold::new(self, init, operation)
    }

    /// Performs a fallible parallel fold with a cloneable `init` value.
    ///
    /// This combines the `init` semantics of [`fold_with()`] and the failure
    /// semantics of [`try_fold()`].
    ///
    /// [`fold_with()`]: #method.fold_with
    /// [`try_fold()`]: #method.try_fold
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let bytes = 0..22_u8;
    /// let sum = bytes.into_par_iter()
    ///                .try_fold_with(0_u32, |a: u32, b: u8| a.checked_add(b as u32))
    ///                .try_reduce(|| 0, u32::checked_add);
    ///
    /// assert_eq!(sum, Some((0..22).sum())); // compare to sequential
    /// ```
    fn try_fold_with<U, O, T>(self, init: U, operation: O) -> TryFoldWith<Self, U, O, T>
    where
        U: Clone + Send + 'a,
        O: Fn(U, Self::Item) -> T + Clone + Send + 'a,
        T: Try<Ok = U>,
    {
        TryFoldWith::new(self, init, operation)
    }

    /// Sums up the items in the iterator.
    ///
    /// Note that the order in items will be reduced is not specified,
    /// so if the `+` operator is not truly [associative] \(as is the
    /// case for floating point numbers), then the results are not
    /// fully deterministic.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    ///
    /// Basically equivalent to `self.reduce(|| 0, |a, b| a + b)`,
    /// except that the type of `0` and the `+` operation may vary
    /// depending on the type of value being produced.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 5, 7];
    ///
    /// let sum: i32 = a.par_iter().sum();
    ///
    /// assert_eq!(sum, 13);
    /// ```
    fn sum<S>(self) -> Sum<Self, S>
    where
        S: std::iter::Sum<Self::Item> + std::iter::Sum<S> + Send,
    {
        Sum::new(self)
    }

    /// Multiplies all the items in the iterator.
    ///
    /// Note that the order in items will be reduced is not specified,
    /// so if the `*` operator is not truly [associative] \(as is the
    /// case for floating point numbers), then the results are not
    /// fully deterministic.
    ///
    /// [associative]: https://en.wikipedia.org/wiki/Associative_property
    ///
    /// Basically equivalent to `self.reduce(|| 1, |a, b| a * b)`,
    /// except that the type of `1` and the `*` operation may vary
    /// depending on the type of value being produced.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// fn factorial(n: u32) -> u32 {
    ///    (1..n+1).into_par_iter().product()
    /// }
    ///
    /// assert_eq!(factorial(0), 1);
    /// assert_eq!(factorial(1), 1);
    /// assert_eq!(factorial(5), 120);
    /// ```
    fn product<P>(self) -> Product<Self, P>
    where
        P: std::iter::Product<Self::Item> + std::iter::Product<P> + Send,
    {
        Product::new(self)
    }

    /// Computes the minimum of all the items in the iterator. If the
    /// iterator is empty, `None` is returned; otherwise, `Some(min)`
    /// is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// Basically equivalent to `self.reduce_with(|a, b| cmp::min(a, b))`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [45, 74, 32];
    ///
    /// assert_eq!(a.par_iter().min(), Some(&32));
    ///
    /// let b: [i32; 0] = [];
    ///
    /// assert_eq!(b.par_iter().min(), None);
    /// ```
    fn min(self) -> Min<Self>
    where
        Self::Item: Ord,
    {
        Min::new(self)
    }

    /// Computes the minimum of all the items in the iterator with respect to
    /// the given comparison function. If the iterator is empty, `None` is
    /// returned; otherwise, `Some(min)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the comparison function is not associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 77, 53, 240, -1];
    ///
    /// assert_eq!(a.par_iter().min_by(|x, y| x.cmp(y)), Some(&-3));
    /// ```
    fn min_by<O>(self, operation: O) -> MinBy<Self, O>
    where
        O: Fn(&Self::Item, &Self::Item) -> Ordering + Clone + Send + Sync + 'a,
    {
        MinBy::new(self, operation)
    }

    /// Computes the item that yields the minimum value for the given
    /// function. If the iterator is empty, `None` is returned;
    /// otherwise, `Some(item)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 34, 2, 5, -10, -3, -23];
    ///
    /// assert_eq!(a.par_iter().min_by_key(|x| x.abs()), Some(&2));
    /// ```
    fn min_by_key<O, K>(self, operation: O) -> MinByKey<Self, O>
    where
        O: Fn(&Self::Item) -> K + Clone + Send + 'a,
        K: Ord + Send,
    {
        MinByKey::new(self, operation)
    }

    /// Computes the maximum of all the items in the iterator. If the
    /// iterator is empty, `None` is returned; otherwise, `Some(max)`
    /// is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// Basically equivalent to `self.reduce_with(|a, b| cmp::max(a, b))`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [45, 74, 32];
    ///
    /// assert_eq!(a.par_iter().max(), Some(&74));
    ///
    /// let b: [i32; 0] = [];
    ///
    /// assert_eq!(b.par_iter().max(), None);
    /// ```
    fn max(self) -> Max<Self>
    where
        Self::Item: Ord,
    {
        Max::new(self)
    }

    /// Computes the maximum of all the items in the iterator with respect to
    /// the given comparison function. If the iterator is empty, `None` is
    /// returned; otherwise, `Some(min)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the comparison function is not associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 77, 53, 240, -1];
    ///
    /// assert_eq!(a.par_iter().max_by(|x, y| x.abs().cmp(&y.abs())), Some(&240));
    /// ```
    fn max_by<O>(self, operation: O) -> MaxBy<Self, O>
    where
        O: Fn(&Self::Item, &Self::Item) -> Ordering + Clone + Send + Sync + 'a,
    {
        MaxBy::new(self, operation)
    }

    /// Computes the item that yields the maximum value for the given
    /// function. If the iterator is empty, `None` is returned;
    /// otherwise, `Some(item)` is returned.
    ///
    /// Note that the order in which the items will be reduced is not
    /// specified, so if the `Ord` impl is not truly associative, then
    /// the results are not deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [-3_i32, 34, 2, 5, -10, -3, -23];
    ///
    /// assert_eq!(a.par_iter().max_by_key(|x| x.abs()), Some(&34));
    /// ```
    fn max_by_key<O, K>(self, operation: O) -> MaxByKey<Self, O>
    where
        O: Fn(&Self::Item) -> K + Clone + Send + 'a,
        K: Ord + Send,
    {
        MaxByKey::new(self, operation)
    }

    /// Takes two iterators and creates a new iterator over both.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [0, 1, 2];
    /// let b = [9, 8, 7];
    ///
    /// let par_iter = a.par_iter().chain(b.par_iter());
    ///
    /// let chained: Vec<_> = par_iter.cloned().collect();
    ///
    /// assert_eq!(&chained[..], &[0, 1, 2, 9, 8, 7]);
    /// ```
    fn chain<C>(self, chain: C) -> Chain<Self, C::Iter>
    where
        C: IntoParallelIterator<'a, Item = Self::Item>,
    {
        Chain::new(self, chain.into_par_iter())
    }

    /// Searches for **some** item in the parallel iterator that
    /// matches the given operation and returns it. This operation
    /// is similar to [`find` on sequential iterators][find] but
    /// the item returned may not be the **first** one in the parallel
    /// sequence which matches, since we search the entire sequence in parallel.
    ///
    /// Once a match is found, we will attempt to stop processing
    /// the rest of the items in the iterator as soon as possible
    /// (just as `find` stops iterating once a match is found).
    ///
    /// [find]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.find
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3, 3];
    ///
    /// assert_eq!(a.par_iter().find_any(|&&x| x == 3), Some(&3));
    ///
    /// assert_eq!(a.par_iter().find_any(|&&x| x == 100), None);
    /// ```
    fn find_any<O>(self, operation: O) -> Find<Self, O>
    where
        O: Fn(&Self::Item) -> bool + Clone + Send + 'a,
    {
        Find::new(self, operation, FindMatch::Any)
    }

    /// Searches for the sequentially **first** item in the parallel iterator
    /// that matches the given operation and returns it.
    ///
    /// Once a match is found, all attempts to the right of the match
    /// will be stopped, while attempts to the left must continue in case
    /// an earlier match is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "first" may be nebulous.  If you
    /// just want the first match that discovered anywhere in the iterator,
    /// `find_any` is a better choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3, 3];
    ///
    /// assert_eq!(a.par_iter().find_first(|&&x| x == 3), Some(&3));
    ///
    /// assert_eq!(a.par_iter().find_first(|&&x| x == 100), None);
    /// ```
    fn find_first<O>(self, operation: O) -> Find<Self, O>
    where
        O: Fn(&Self::Item) -> bool + Clone + Send + 'a,
    {
        Find::new(self, operation, FindMatch::First)
    }

    /// Searches for the sequentially **last** item in the parallel iterator
    /// that matches the given operation and returns it.
    ///
    /// Once a match is found, all attempts to the left of the match
    /// will be stopped, while attempts to the right must continue in case
    /// a later match is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "last" may be nebulous.  When the
    /// order doesn't actually matter to you, `find_any` is a better choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [1, 2, 3, 3];
    ///
    /// assert_eq!(a.par_iter().find_last(|&&x| x == 3), Some(&3));
    ///
    /// assert_eq!(a.par_iter().find_last(|&&x| x == 100), None);
    /// ```
    fn find_last<O>(self, operation: O) -> Find<Self, O>
    where
        O: Fn(&Self::Item) -> bool + Clone + Send + 'a,
    {
        Find::new(self, operation, FindMatch::Last)
    }

    /// Applies the given operation to the items in the parallel iterator
    /// and returns **any** non-None result of the map operation.
    ///
    /// Once a non-None value is produced from the map operation, we will
    /// attempt to stop processing the rest of the items in the iterator
    /// as soon as possible.
    ///
    /// Note that this method only returns **some** item in the parallel
    /// iterator that is not None from the map operation. The item returned
    /// may not be the **first** non-None value produced in the parallel
    /// sequence, since the entire sequence is mapped over in parallel.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let c = ["lol", "NaN", "5", "5"];
    ///
    /// let found_number = c.par_iter().find_map_any(|s| s.parse().ok());
    ///
    /// assert_eq!(found_number, Some(5));
    /// ```
    fn find_map_any<O, T>(self, operation: O) -> FindMap<Self, O>
    where
        O: Fn(Self::Item) -> Option<T> + Clone + Send + 'a,
        T: Send,
    {
        FindMap::new(self, operation, FindMatch::Any)
    }

    /// Applies the given operation to the items in the parallel iterator and
    /// returns the sequentially **first** non-None result of the map operation.
    ///
    /// Once a non-None value is produced from the map operation, all attempts
    /// to the right of the match will be stopped, while attempts to the left
    /// must continue in case an earlier match is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "first" may be nebulous. If you
    /// just want the first non-None value discovered anywhere in the iterator,
    /// `find_map_any` is a better choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let c = ["lol", "NaN", "2", "5"];
    ///
    /// let first_number = c.par_iter().find_map_first(|s| s.parse().ok());
    ///
    /// assert_eq!(first_number, Some(2));
    /// ```
    fn find_map_first<O, T>(self, operation: O) -> FindMap<Self, O>
    where
        O: Fn(Self::Item) -> Option<T> + Clone + Send + 'a,
        T: Send,
    {
        FindMap::new(self, operation, FindMatch::First)
    }

    /// Applies the given operation to the items in the parallel iterator and
    /// returns the sequentially **last** non-None result of the map operation.
    ///
    /// Once a non-None value is produced from the map operation, all attempts
    /// to the left of the match will be stopped, while attempts to the right
    /// must continue in case a later match is found.
    ///
    /// Note that not all parallel iterators have a useful order, much like
    /// sequential `HashMap` iteration, so "first" may be nebulous. If you
    /// just want the first non-None value discovered anywhere in the iterator,
    /// `find_map_any` is a better choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let c = ["lol", "NaN", "2", "5"];
    ///
    /// let last_number = c.par_iter().find_map_last(|s| s.parse().ok());
    ///
    /// assert_eq!(last_number, Some(5));
    /// ```
    fn find_map_last<O, T>(self, operation: O) -> FindMap<Self, O>
    where
        O: Fn(Self::Item) -> Option<T> + Clone + Send + 'a,
        T: Send,
    {
        FindMap::new(self, operation, FindMatch::Last)
    }

    /// Searches for **some** item in the parallel iterator that
    /// matches the given operation, and if so returns true.  Once
    /// a match is found, we'll attempt to stop process the rest
    /// of the items.  Proving that there's no match, returning false,
    /// does require visiting every item.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [0, 12, 3, 4, 0, 23, 0];
    ///
    /// let is_valid = a.par_iter().any(|&x| x > 10);
    ///
    /// assert!(is_valid);
    /// ```
    fn any<O>(self, operation: O) -> Any<Self, O>
    where
        O: Fn(Self::Item) -> bool + Clone + Send + 'a,
    {
        Any::new(self, operation)
    }

    /// Tests that every item in the parallel iterator matches the given
    /// operation, and if so returns true.  If a counter-example is found,
    /// we'll attempt to stop processing more items, then return false.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [0, 12, 3, 4, 0, 23, 0];
    ///
    /// let is_valid = a.par_iter().all(|&x| x > 10);
    ///
    /// assert!(!is_valid);
    /// ```
    fn all<O>(self, operation: O) -> All<Self, O>
    where
        O: Fn(Self::Item) -> bool + Clone + Send + 'a,
    {
        All::new(self, operation)
    }

    /// Creates an iterator over the `Some` items of this iterator, halting
    /// as soon as any `None` is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    ///
    /// let counter = AtomicUsize::new(0);
    /// let value = (0_i32..2048)
    ///     .into_par_iter()
    ///     .map(|x| {
    ///              counter.fetch_add(1, Ordering::SeqCst);
    ///              if x < 1024 { Some(x) } else { None }
    ///          })
    ///     .while_some()
    ///     .max();
    ///
    /// assert!(value = Some(1023));
    /// assert!(counter.load(Ordering::SeqCst) < 2048); // should not have visited every single one
    /// ```
    fn while_some<T>(self) -> WhileSome<Self>
    where
        Self: ParallelIterator<'a, Item = Option<T>>,
        T: Send + 'a,
    {
        WhileSome::new(self)
    }

    /// Wraps an iterator with a fuse in case of panics, to halt all threads
    /// as soon as possible.
    ///
    /// Panics within parallel iterators are always propagated to the caller,
    /// but they don't always halt the rest of the iterator right away, due to
    /// the internal semantics of [`join`]. This adaptor makes a greater effort
    /// to stop processing other items sooner, with the cost of additional
    /// synchronization overhead, which may also inhibit some optimizations.
    ///
    /// [`join`]: ../fn.join.html#panics
    ///
    /// # Examples
    ///
    /// If this code didn't use `panic_fuse()`, it would continue processing
    /// many more items in other threads (with long sleep delays) before the
    /// panic is finally propagated.
    ///
    /// ```should_panic
    /// use rayon::prelude::*;
    /// use std::{thread, time};
    ///
    /// (0..1_000_000)
    ///     .into_par_iter()
    ///     .panic_fuse()
    ///     .for_each(|i| {
    ///         // simulate some work
    ///         thread::sleep(time::Duration::from_secs(1));
    ///         assert!(i > 0); // oops!
    ///     });
    /// ```
    fn panic_fuse(self) -> PanicFuse<Self> {
        PanicFuse::new(self)
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
        T: FromParallelIterator<'a, Self::Item>,
    {
        Collect::new(self)
    }

    /// Unzips the items of a parallel iterator into a pair of arbitrary
    /// `ParallelExtend` containers.
    ///
    /// You may prefer to use `unzip_into_vecs()`, which allocates more
    /// efficiently with precise knowledge of how many elements the
    /// iterator contains, and even allows you to reuse existing
    /// vectors' backing stores rather than allocating fresh vectors.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let a = [(0, 1), (1, 2), (2, 3), (3, 4)];
    ///
    /// let (left, right): (Vec<_>, Vec<_>) = a.par_iter().cloned().unzip();
    ///
    /// assert_eq!(left, [0, 1, 2, 3]);
    /// assert_eq!(right, [1, 2, 3, 4]);
    /// ```
    ///
    /// Nested pairs can be unzipped too.
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let (values, squares, cubes): (Vec<_>, Vec<_>, Vec<_>) = (0..4).into_par_iter()
    ///     .map(|i| (i, i * i, i * i * i))
    ///     .unzip();
    ///
    /// assert_eq!(values, [0, 1, 2, 3]);
    /// assert_eq!(squares, [0, 1, 4, 9]);
    /// assert_eq!(cubes, [0, 1, 8, 27]);
    /// ```
    fn unzip(self) -> Unzip<Self> {
        Unzip::new(self)
    }

    /// Partitions the items of a parallel iterator into a pair of arbitrary
    /// `ParallelExtend` containers.  Items for which the `operation` returns
    /// true go into the first container, and the rest go into the second.
    ///
    /// Note: unlike the standard `Iterator::partition`, this allows distinct
    /// collection types for the left and right items.  This is more flexible,
    /// but may require new type annotations when converting sequential code
    /// that used type inferrence assuming the two were the same.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let (left, right): (Vec<_>, Vec<_>) = (0..8).into_par_iter().partition(|x| x % 2 == 0);
    ///
    /// assert_eq!(left, [0, 2, 4, 6]);
    /// assert_eq!(right, [1, 3, 5, 7]);
    /// ```
    fn partition<O>(self, operation: O) -> Partition<Self, O>
    where
        O: Fn(&Self::Item) -> bool + Sync + Send,
    {
        Partition::new(self, operation)
    }

    /// Partitions and maps the items of a parallel iterator into a pair of
    /// arbitrary `ParallelExtend` containers.  `Either::Left` items go into
    /// the first container, and `Either::Right` items go into the second.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use rayon::iter::Either;
    ///
    /// let (left, right): (Vec<_>, Vec<_>) = (0..8).into_par_iter()
    ///     .partition_map(|x| {
    ///         if x % 2 == 0 {
    ///             Either::Left(x * 4)
    ///         } else {
    ///             Either::Right(x * 3)
    ///         }
    ///     });
    ///
    /// assert_eq!(left, [0, 8, 16, 24]);
    /// assert_eq!(right, [3, 9, 15, 21]);
    /// ```
    ///
    /// Nested `Either` enums can be split as well.
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use rayon::iter::Either::*;
    ///
    /// let ((fizzbuzz, fizz), (buzz, other)): ((Vec<_>, Vec<_>), (Vec<_>, Vec<_>)) = (1..20)
    ///     .into_par_iter()
    ///     .partition_map(|x| match (x % 3, x % 5) {
    ///         (0, 0) => Left(Left(x)),
    ///         (0, _) => Left(Right(x)),
    ///         (_, 0) => Right(Left(x)),
    ///         (_, _) => Right(Right(x)),
    ///     });
    ///
    /// assert_eq!(fizzbuzz, [15]);
    /// assert_eq!(fizz, [3, 6, 9, 12, 18]);
    /// assert_eq!(buzz, [5, 10]);
    /// assert_eq!(other, [1, 2, 4, 7, 8, 11, 13, 14, 16, 17, 19]);
    /// ```
    fn partition_map<O>(self, operation: O) -> PartitionMap<Self, O> {
        PartitionMap::new(self, operation)
    }

    /// Intersperses clones of an element between items of this iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let x = vec![1, 2, 3];
    /// let r: Vec<_> = x.into_par_iter().intersperse(-1).collect();
    ///
    /// assert_eq!(r, vec![1, -1, 2, -1, 3]);
    /// ```
    fn intersperse(self, item: Self::Item) -> Intersperse<Self, Self::Item>
    where
        Self::Item: Clone,
    {
        Intersperse::new(self, item)
    }

    /// Sets the number of splits that are processed in parallel.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let min = (0..1_000_000)
    ///     .into_par_iter()
    ///     .with_splits(8)
    ///     .for_each(|| println!("Thread ID: {:?}", std::thread::current().id))
    ///     .exec();
    ///
    /// assert!(min >= 1234);
    /// ```
    fn with_splits(self, splits: usize) -> Splits<Self> {
        Splits::new(self, splits)
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
        D: Send + 'a,
        R: Reducer<D> + Send + 'a;

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
