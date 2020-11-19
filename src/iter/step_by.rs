use std::cmp::min;

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Producer, Reducer, Setup, WithIndexedProducer,
    WithSetup,
};

/* StepBy */

pub struct StepBy<X> {
    base: X,
    step: usize,
}

impl<X> StepBy<X> {
    pub fn new(base: X, step: usize) -> Self {
        Self { base, step }
    }
}

impl<'a, X, I> ParallelIterator<'a> for StepBy<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Item = I;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, I> IndexedParallelIterator<'a> for StepBy<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

impl<'a, X> WithIndexedProducer<'a> for StepBy<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = X::Item;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(StepByCallback {
            base,
            step: self.step,
        })
    }
}

/* StepByCallback */

struct StepByCallback<CB> {
    base: CB,
    step: usize,
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for StepByCallback<CB>
where
    CB: IndexedProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        self.base.callback(StepByProducer {
            base,
            step: self.step,
        })
    }
}

/* StepByProducer */

struct StepByProducer<P> {
    base: P,
    step: usize,
}

impl<P> WithSetup for StepByProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        let Setup {
            splits,
            min_len,
            max_len,
        } = self.base.setup();

        Setup {
            splits,
            min_len: min_len.map(|x| if x > 0 { (x - 1) / self.step + 1 } else { x }),
            max_len: max_len.map(|x| x / self.step),
        }
    }
}

impl<P> Producer for StepByProducer<P>
where
    P: IndexedProducer,
{
    type Item = P::Item;
    type IntoIter = std::iter::StepBy<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().step_by(self.step)
    }

    fn split(self) -> (Self, Option<Self>) {
        let len = self.len();
        if len < 2 {
            return (self, None);
        }

        let (left, right) = self.split_at(len / 2);

        (left, Some(right))
    }
}

impl<P> IndexedProducer for StepByProducer<P>
where
    P: IndexedProducer,
{
    type Item = P::Item;
    type IntoIter = std::iter::StepBy<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().step_by(self.step)
    }

    fn len(&self) -> usize {
        self.base.len() / self.step
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let index = min(index * self.step, self.base.len());

        let (left, right) = self.base.split_at(index);

        let left = Self {
            base: left,
            step: self.step,
        };
        let right = Self {
            base: right,
            step: self.step,
        };

        (left, right)
    }
}
