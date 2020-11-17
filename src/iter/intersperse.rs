use std::iter::{once, DoubleEndedIterator, ExactSizeIterator, Fuse, Iterator};

use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithSetup,
};

/* Intersperse */

pub struct Intersperse<X, I> {
    base: X,
    item: I,
}

impl<X, I> Intersperse<X, I> {
    pub fn new(base: X, item: I) -> Self {
        Self { base, item }
    }
}

impl<'a, X, I> ParallelIterator<'a> for Intersperse<X, I>
where
    X: ParallelIterator<'a, Item = I>,
    I: Clone + Send + 'a,
{
    type Item = I;

    fn drive<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            IntersperseConsumer {
                base,
                item: self.item,
                clone_first: false,
            },
        )
    }

    fn with_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        let item = self.item;

        self.base.with_producer(IntersperseCallback { base, item })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        match self.base.len_hint_opt()? {
            0 => Some(0),
            len => len.checked_add(len - 1),
        }
    }
}

impl<'a, X, I> IndexedParallelIterator<'a> for Intersperse<X, I>
where
    X: IndexedParallelIterator<'a, Item = I>,
    I: Clone + Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive_indexed(
            executor,
            IntersperseConsumer {
                base,
                item: self.item,
                clone_first: false,
            },
        )
    }

    fn with_producer_indexed<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        let item = self.item;

        self.base
            .with_producer_indexed(IntersperseCallback { base, item })
    }

    fn len_hint(&self) -> usize {
        match self.base.len_hint() {
            0 => 0,
            len => len - 1,
        }
    }
}

/* IntersperseCallback */

struct IntersperseCallback<CB, I> {
    base: CB,
    item: I,
}

impl<'a, CB, I> ProducerCallback<'a, I> for IntersperseCallback<CB, I>
where
    CB: ProducerCallback<'a, I>,
    I: Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a,
    {
        let item = self.item;
        let len = None;

        self.base.callback(IntersperseProducer {
            base,
            item,
            len,
            clone_first: false,
        })
    }
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for IntersperseCallback<CB, I>
where
    CB: IndexedProducerCallback<'a, I>,
    I: Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let item = self.item;
        let len = Some(base.len());

        self.base.callback(IntersperseProducer {
            base,
            item,
            len,
            clone_first: false,
        })
    }
}

/* IntersperseProducer */

struct IntersperseProducer<P, I> {
    base: P,
    item: I,
    len: Option<usize>,
    clone_first: bool,
}

impl<P, I> WithSetup for IntersperseProducer<P, I>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<P, I> Producer for IntersperseProducer<P, I>
where
    P: Producer<Item = I>,
    I: Clone + Send,
{
    type Item = I;
    type IntoIter = IntersperseIter<P::IntoIter, I>;

    fn into_iter(self) -> Self::IntoIter {
        IntersperseIter {
            base: self.base.into_iter().fuse(),
            item: self.item,
            item_front: None,
            item_back: None,
            clone_first: self.clone_first,
            clone_last: false,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let item = self.item;
        let (left, right) = self.base.split();

        let left = Self {
            base: left,
            item: item.clone(),
            len: None,
            clone_first: self.clone_first,
        };
        let right = right.map(move |base| Self {
            base,
            item,
            len: None,
            clone_first: true,
        });

        (left, right)
    }

    fn fold_with<F>(self, base: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let item = self.item;
        let clone_first = self.clone_first;
        let folder = IntersperseFolder {
            base,
            item,
            clone_first,
        };

        self.base.fold_with(folder).base
    }
}

impl<P, I> IndexedProducer for IntersperseProducer<P, I>
where
    P: IndexedProducer<Item = I>,
    I: Clone + Send,
{
    type Item = I;
    type IntoIter = IntersperseIter<P::IntoIter, I>;

    fn into_iter(self) -> Self::IntoIter {
        let len = self.len.unwrap();

        IntersperseIter {
            base: self.base.into_iter().fuse(),
            item: self.item,
            item_front: None,
            item_back: None,
            clone_first: len > 0 && self.clone_first,
            clone_last: len > 1 && ((len & 1 == 0) ^ self.clone_first),
        }
    }

    #[allow(clippy::let_and_return)]
    fn len(&self) -> usize {
        let len = self.len.unwrap();

        let clone_first = len > 0 && self.clone_first;
        let clone_last = len > 1 && ((len & 1 == 0) ^ self.clone_first);

        let len = 2 * self.len.unwrap();
        let len = len + clone_first as usize;
        let len = len - (len > 0 && !clone_last) as usize;

        len
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let len = self.len.unwrap();

        debug_assert!(index <= len);

        // The left needs half of the items from the base producer, and the
        // other half will be our interspersed item.  If we're not leading with
        // a cloned item, then we need to round up the base number of items,
        // otherwise round down.
        let base_index = (index + !self.clone_first as usize) / 2;
        let (left_base, right_base) = self.base.split_at(base_index);

        let left = IntersperseProducer {
            base: left_base,
            item: self.item.clone(),
            len: Some(index),
            clone_first: self.clone_first,
        };

        let right = IntersperseProducer {
            base: right_base,
            item: self.item,
            len: Some(len - index),
            clone_first: (index & 1 == 1) ^ self.clone_first,
        };

        (left, right)
    }

    fn fold_with<F>(self, base: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = IntersperseFolder {
            base,
            item: self.item,
            clone_first: self.clone_first,
        };

        self.base.fold_with(folder).base
    }
}

/* IntersperseIter */

struct IntersperseIter<X, I> {
    base: Fuse<X>,
    item: I,
    item_front: Option<I>,
    item_back: Option<I>,
    clone_first: bool,
    clone_last: bool,
}

impl<X, I> IntersperseIter<X, I> {
    fn calc_len(&self, len: usize) -> usize {
        let mut len = 2 * len;

        if self.clone_first {
            len += 1
        }

        if len > 0 && !self.clone_last {
            len -= 1;
        }

        len
    }
}

impl<X, I> IntersperseIter<X, I>
where
    X: Iterator<Item = I>,
{
    fn get_next_front(&mut self) -> Option<I> {
        if let Some(item) = self.item_front.take() {
            return Some(item);
        }

        if let Some(item) = self.base.next() {
            return Some(item);
        }

        if let Some(item) = self.item_back.take() {
            return Some(item);
        }

        None
    }
}

impl<X, I> IntersperseIter<X, I>
where
    X: DoubleEndedIterator<Item = I>,
{
    fn get_next_back(&mut self) -> Option<I> {
        if let Some(item) = self.item_back.take() {
            return Some(item);
        }

        if let Some(item) = self.base.next_back() {
            return Some(item);
        }

        if let Some(item) = self.item_front.take() {
            return Some(item);
        }

        None
    }
}

impl<X, I> Iterator for IntersperseIter<X, I>
where
    X: Iterator<Item = I>,
    I: Clone,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        if self.clone_first {
            self.clone_first = false;

            Some(self.item.clone())
        } else if let Some(next) = self.get_next_front() {
            if let Some(item) = self.get_next_front() {
                self.clone_first = true;
                self.item_front = Some(item);
            }

            Some(next)
        } else if self.clone_last {
            self.clone_last = false;

            Some(self.item.clone())
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (beg, end) = self.base.size_hint();

        let beg = self.calc_len(beg);
        let end = end.map(|end| self.calc_len(end));

        (beg, end)
    }
}

impl<X, I> DoubleEndedIterator for IntersperseIter<X, I>
where
    X: DoubleEndedIterator<Item = I>,
    I: Clone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.clone_last {
            self.clone_last = false;

            Some(self.item.clone())
        } else if let Some(next) = self.get_next_back() {
            if let Some(item) = self.get_next_back() {
                self.clone_last = true;
                self.item_back = Some(item);
            }

            Some(next)
        } else if self.clone_first {
            self.clone_first = false;

            Some(self.item.clone())
        } else {
            None
        }
    }
}

impl<X, I> ExactSizeIterator for IntersperseIter<X, I>
where
    X: ExactSizeIterator<Item = I>,
    I: Clone,
{
    fn len(&self) -> usize {
        self.calc_len(self.base.len())
    }
}

/* IntersperseConsumer */

struct IntersperseConsumer<C, I> {
    base: C,
    item: I,
    clone_first: bool,
}

impl<C, I> WithSetup for IntersperseConsumer<C, I>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<C, I> Consumer<I> for IntersperseConsumer<C, I>
where
    C: Consumer<I>,
    I: Clone + Send,
{
    type Folder = IntersperseFolder<C::Folder, I>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = Self {
            base: left,
            item: self.item.clone(),
            clone_first: self.clone_first,
        };
        let right = Self {
            base: right,
            item: self.item,
            clone_first: true,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let base_index = index + index.saturating_sub(!self.clone_first as usize);
        let (left, right, reducer) = self.base.split_at(base_index);

        let left = Self {
            base: left,
            item: self.item.clone(),
            clone_first: self.clone_first,
        };
        let right = Self {
            base: right,
            item: self.item,
            clone_first: true,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        IntersperseFolder {
            base: self.base.into_folder(),
            item: self.item,
            clone_first: self.clone_first,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* IntersperseFolder */

struct IntersperseFolder<F, I> {
    base: F,
    item: I,
    clone_first: bool,
}

impl<F, I> Folder<I> for IntersperseFolder<F, I>
where
    F: Folder<I>,
    I: Clone,
{
    type Result = F::Result;

    fn consume(mut self, item: I) -> Self {
        if self.clone_first {
            self.base = self.base.consume(self.item.clone());
        } else {
            self.clone_first = true;
        }

        self.base = self.base.consume(item);

        self
    }

    fn consume_iter<X>(self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        let mut clone_first = self.clone_first;
        let item = self.item;
        let iter = iter.into_iter().flat_map(|x| {
            let first = if clone_first {
                Some(item.clone())
            } else {
                clone_first = true;

                None
            };

            first.into_iter().chain(once(x))
        });
        let base = self.base.consume_iter(iter);

        IntersperseFolder {
            base,
            item,
            clone_first,
        }
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
