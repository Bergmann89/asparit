use std::ops::Range;

use crate::{Consumer, Folder, IntoParallelIterator, ParallelIterator, Producer, ProducerCallback, Executor, Reducer, ExecutorCallback};

pub struct Iter {
    range: Range<usize>,
}

struct IterProducer {
    range: Range<usize>,
}

impl<'a> IntoParallelIterator<'a> for Range<usize> {
    type Iter = Iter;
    type Item = usize;

    fn into_par_iter(self) -> Self::Iter {
        Iter { range: self }
    }
}

impl<'a> ParallelIterator<'a> for Iter {
    type Item = usize;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send,
    {
        self.with_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint_opt(&self) -> Option<usize> {
        Some(self.range.end - self.range.start)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(IterProducer { range: self.range })
    }
}

impl Producer for IterProducer {
    type Item = usize;
    type IntoIter = Range<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.range
    }

    fn split(mut self) -> (Self, Option<Self>) {
        let index = self.range.len() / 2;

        if index > 0 {
            let mid = self.range.start.wrapping_add(index);
            let right = mid..self.range.end;

            self.range.end = mid;

            (self, Some(IterProducer { range: right }))
        } else {
            (self, None)
        }
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        folder.consume_iter(self.range)
    }
}
