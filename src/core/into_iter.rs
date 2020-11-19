use super::ParallelIterator;

/// `IntoParallelIterator` implements the conversion to a [`ParallelIterator`].
///
/// By implementing `IntoParallelIterator` for a type, you define how it will
/// transformed into an iterator. This is a parallel version of the standard
/// library's [`std::iter::IntoIterator`] trait.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`std::iter::IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
pub trait IntoParallelIterator<'a> {
    /// The parallel iterator type that will be created.
    type Iter: ParallelIterator<'a, Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    type Item: Send + 'a;

    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use asparit::*;
    ///
    /// println!("counting in parallel:");
    /// (0..100)
    ///     .into_par_iter()
    ///     .for_each(|i| println!("{}", i))
    ///     .exec_with(SimpleExecutor);
    /// ```
    ///
    /// This conversion is often implicit for arguments to methods like [`zip`].
    ///
    /// ```
    /// use asparit::*;
    ///
    /// let v: Vec<_> = (0..5)
    ///     .into_par_iter()
    ///     .zip(5..10)
    ///     .collect()
    ///     .exec_with(SimpleExecutor);
    /// assert_eq!(v, [(0, 5), (1, 6), (2, 7), (3, 8), (4, 9)]);
    /// ```
    ///
    /// [`zip`]: trait.IndexedParallelIterator.html#method.zip
    fn into_par_iter(self) -> Self::Iter;
}

/// `IntoParallelRefIterator` implements the conversion to a
/// [`ParallelIterator`], providing shared references to the data.
///
/// This is a parallel version of the `iter()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefIterator<'a> {
    /// The type of the parallel iterator that will be returned.
    type Iter: ParallelIterator<'a, Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This will typically be an `&'a T` reference type.
    type Item: Send + 'a;

    /// Converts `self` into a parallel iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use asparit::*;
    ///
    /// let v: Vec<_> = (0..100).collect();
    /// assert_eq!(
    ///     v.par_iter().sum::<i32>().exec_with(SimpleExecutor),
    ///     100 * 99 / 2
    /// );
    ///
    /// // `v.par_iter()` is shorthand for `(&v).into_par_iter()`,
    /// // producing the exact same references.
    /// assert!(v
    ///     .par_iter()
    ///     .zip(&v)
    ///     .all(|(a, b)| std::ptr::eq(a, b))
    ///     .exec_with(SimpleExecutor));
    /// ```
    fn par_iter(&'a self) -> Self::Iter;
}

/// `IntoParallelRefMutIterator` implements the conversion to a
/// [`ParallelIterator`], providing mutable references to the data.
///
/// This is a parallel version of the `iter_mut()` method
/// defined by various collections.
///
/// This trait is automatically implemented
/// `for I where &mut I: IntoParallelIterator`. In most cases, users
/// will want to implement [`IntoParallelIterator`] rather than implement
/// this trait directly.
///
/// [`ParallelIterator`]: trait.ParallelIterator.html
/// [`IntoParallelIterator`]: trait.IntoParallelIterator.html
pub trait IntoParallelRefMutIterator<'a> {
    /// The type of iterator that will be created.
    type Iter: ParallelIterator<'a, Item = Self::Item>;

    /// The type of item that will be produced; this is typically an
    /// `&'a mut T` reference.
    type Item: Send + 'a;

    /// Creates the parallel iterator from `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asparit::*;
    ///
    /// let mut v = vec![0usize; 5];
    /// v.par_iter_mut()
    ///     .enumerate()
    ///     .for_each(|(i, x)| *x = i)
    ///     .exec_with(SimpleExecutor);
    /// assert_eq!(v, [0, 1, 2, 3, 4]);
    /// ```
    fn par_iter_mut(&'a mut self) -> Self::Iter;
}

impl<'a, T> IntoParallelIterator<'a> for T
where
    T: ParallelIterator<'a>,
{
    type Iter = T;
    type Item = T::Item;

    fn into_par_iter(self) -> T {
        self
    }
}

impl<'a, I> IntoParallelRefIterator<'a> for I
where
    I: 'a + ?Sized,
    &'a I: IntoParallelIterator<'a>,
{
    type Iter = <&'a I as IntoParallelIterator<'a>>::Iter;
    type Item = <&'a I as IntoParallelIterator<'a>>::Item;

    fn par_iter(&'a self) -> Self::Iter {
        self.into_par_iter()
    }
}

impl<'a, I> IntoParallelRefMutIterator<'a> for I
where
    I: 'a + ?Sized,
    &'a mut I: IntoParallelIterator<'a>,
{
    type Iter = <&'a mut I as IntoParallelIterator<'a>>::Iter;
    type Item = <&'a mut I as IntoParallelIterator<'a>>::Item;

    fn par_iter_mut(&'a mut self) -> Self::Iter {
        self.into_par_iter()
    }
}
