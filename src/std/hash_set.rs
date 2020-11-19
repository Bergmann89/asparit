use std::collections::HashSet;
use std::hash::BuildHasher;
use std::iter::FromIterator;

use crate::{IntoParallelIterator, ParallelDrainFull};

impl<'a, I, S> IntoParallelIterator<'a> for HashSet<I, S>
where
    I: Send + 'a,
    S: BuildHasher,
{
    type Iter = <Vec<I> as IntoParallelIterator<'a>>::Iter;
    type Item = I;

    fn into_par_iter(self) -> Self::Iter {
        let vec = Vec::from_iter(self);

        vec.into_par_iter()
    }
}

impl<'a, I, S> IntoParallelIterator<'a> for &'a HashSet<I, S>
where
    I: Send + Sync + 'a,
    S: BuildHasher,
{
    type Iter = <Vec<&'a I> as IntoParallelIterator<'a>>::Iter;
    type Item = &'a I;

    fn into_par_iter(self) -> Self::Iter {
        let vec = Vec::<&'a I>::from_iter(self);

        vec.into_par_iter()
    }
}

impl<'a, I, S> ParallelDrainFull<'a> for &'a mut HashSet<I, S>
where
    I: Send + 'a,
    S: BuildHasher,
{
    type Iter = <Vec<I> as IntoParallelIterator<'a>>::Iter;
    type Item = I;

    fn par_drain(self) -> Self::Iter {
        let vec = self.drain().collect::<Vec<_>>();

        vec.into_par_iter()
    }
}
