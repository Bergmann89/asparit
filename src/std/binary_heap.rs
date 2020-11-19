use std::collections::BinaryHeap;
use std::iter::FromIterator;

use crate::{IntoParallelIterator, ParallelDrainFull};

impl<'a, I> IntoParallelIterator<'a> for BinaryHeap<I>
where
    I: Send + 'a,
{
    type Iter = <Vec<I> as IntoParallelIterator<'a>>::Iter;
    type Item = I;

    fn into_par_iter(self) -> Self::Iter {
        let vec = Vec::from_iter(self);

        vec.into_par_iter()
    }
}

impl<'a, I> IntoParallelIterator<'a> for &'a BinaryHeap<I>
where
    I: Send + Sync + 'a,
{
    type Iter = <Vec<&'a I> as IntoParallelIterator<'a>>::Iter;
    type Item = &'a I;

    fn into_par_iter(self) -> Self::Iter {
        let vec = Vec::<&'a I>::from_iter(self);

        vec.into_par_iter()
    }
}

impl<'a, I> ParallelDrainFull<'a> for &'a mut BinaryHeap<I>
where
    I: Send + 'a,
{
    type Iter = <Vec<I> as IntoParallelIterator<'a>>::Iter;
    type Item = I;

    fn par_drain(self) -> Self::Iter {
        let vec = self.drain().collect::<Vec<_>>();

        vec.into_par_iter()
    }
}
