use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};

use crate::{Driver, Executor, IndexedParallelIterator, ParallelIterator, WithIndexedProducer};

/* Cmp */

pub struct Cmp<XA, XB> {
    iterator_a: XA,
    iterator_b: XB,
}

impl<XA, XB> Cmp<XA, XB> {
    pub fn new(iterator_a: XA, iterator_b: XB) -> Self {
        Self {
            iterator_a,
            iterator_b,
        }
    }
}

impl<'a, XA, XB, I> Driver<'a, Ordering, Option<Ordering>> for Cmp<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    XB: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Ord + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Ordering, Option<Ordering>>,
    {
        let Self {
            iterator_a,
            iterator_b,
        } = self;

        let len_a = iterator_a.len_hint();
        let len_b = iterator_b.len_hint();
        let ord_len = len_a.cmp(&len_b);

        let executor = executor.into_inner();
        let inner = iterator_a
            .zip(iterator_b)
            .map(|(a, b)| Ord::cmp(&a, &b))
            .find_first(|ord| ord != &Ordering::Equal)
            .exec_with(executor);

        E::map(inner, move |inner| inner.unwrap_or(ord_len))
    }
}

/* PartialCmp */

pub struct PartialCmp<XA, XB> {
    iterator_a: XA,
    iterator_b: XB,
}

impl<XA, XB> PartialCmp<XA, XB> {
    pub fn new(iterator_a: XA, iterator_b: XB) -> Self {
        Self {
            iterator_a,
            iterator_b,
        }
    }
}

impl<'a, XA, XB, I> Driver<'a, Option<Ordering>, Option<Option<Ordering>>> for PartialCmp<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    XB: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: PartialOrd + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Option<Ordering>, Option<Option<Ordering>>>,
    {
        let Self {
            iterator_a,
            iterator_b,
        } = self;

        let len_a = iterator_a.len_hint();
        let len_b = iterator_b.len_hint();
        let ord_len = len_a.cmp(&len_b);

        let executor = executor.into_inner();
        let inner = iterator_a
            .zip(iterator_b)
            .map(|(a, b)| PartialOrd::partial_cmp(&a, &b))
            .find_first(|ord| ord != &Some(Ordering::Equal))
            .exec_with(executor);

        E::map(inner, move |inner| inner.unwrap_or(Some(ord_len)))
    }
}

/* Equal */

pub struct Equal<XA, XB> {
    iterator_a: XA,
    iterator_b: XB,
    expected: bool,
}

impl<XA, XB> Equal<XA, XB> {
    pub fn new(iterator_a: XA, iterator_b: XB, expected: bool) -> Self {
        Self {
            iterator_a,
            iterator_b,
            expected,
        }
    }
}

impl<'a, XA, XB, I> Driver<'a, bool, Option<bool>> for Equal<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    XB: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: PartialEq + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, bool, Option<bool>>,
    {
        let Self {
            iterator_a,
            iterator_b,
            expected,
        } = self;

        let len_a = iterator_a.len_hint();
        let len_b = iterator_b.len_hint();

        if (len_a == len_b) ^ expected {
            return executor.ready(false);
        }

        iterator_a
            .zip(iterator_b)
            .all(move |(x, y)| PartialEq::eq(&x, &y) == expected)
            .exec_with(executor)
    }
}

/* Compare */

pub struct Compare<XA, XB> {
    iterator_a: XA,
    iterator_b: XB,
    ord: Ordering,
    ord_opt: Option<Ordering>,
}

impl<XA, XB> Compare<XA, XB> {
    pub fn new(iterator_a: XA, iterator_b: XB, ord: Ordering, ord_opt: Option<Ordering>) -> Self {
        Self {
            iterator_a,
            iterator_b,
            ord,
            ord_opt,
        }
    }
}

impl<'a, XA, XB, I> Driver<'a, bool, Option<Ordering>, Option<Option<Ordering>>> for Compare<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    XB: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: PartialOrd + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, bool, Option<Ordering>, Option<Option<Ordering>>>,
    {
        let Self {
            iterator_a,
            iterator_b,
            ord,
            ord_opt,
        } = self;

        let executor = executor.into_inner();
        let inner = PartialCmp::new(iterator_a, iterator_b).exec_with(executor);

        E::map(inner, move |inner| inner == Some(ord) || inner == ord_opt)
    }
}
