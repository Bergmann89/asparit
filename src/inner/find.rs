use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::{Consumer, Driver, Executor, Folder, ParallelIterator, Reducer};

/* Find */

pub struct Find<X, O> {
    iterator: X,
    operation: O,
    find_match: FindMatch,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FindMatch {
    Any,
    First,
    Last,
}

impl<X, O> Find<X, O> {
    pub fn new(iterator: X, operation: O, find_match: FindMatch) -> Self {
        Self {
            iterator,
            operation,
            find_match,
        }
    }
}

impl<'a, X, O> Driver<'a, Option<X::Item>> for Find<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&X::Item) -> bool + Clone + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Option<X::Item>>,
    {
        let consumer = FindConsumer {
            operation: self.operation,
            found: Arc::new(AtomicUsize::new(0)),
            lower_bound: 1,
            upper_bound: usize::max_value(),
            find_match: self.find_match,
        };

        self.iterator.drive(executor, consumer)
    }
}

/* FindConsumer */

struct FindConsumer<O> {
    operation: O,
    found: Arc<AtomicUsize>,
    lower_bound: usize,
    upper_bound: usize,
    find_match: FindMatch,
}

impl<O, T> Consumer<T> for FindConsumer<O>
where
    O: Fn(&T) -> bool + Clone + Send,
    T: Send,
{
    type Folder = FindFolder<O, T>;
    type Reducer = FindReducer;
    type Result = Option<T>;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let FindConsumer {
            operation,
            found,
            lower_bound,
            upper_bound,
            find_match,
        } = self;
        let mid = lower_bound + (upper_bound - lower_bound) / 2;

        (
            Self {
                operation: operation.clone(),
                found: found.clone(),
                lower_bound,
                upper_bound: mid,
                find_match,
            },
            Self {
                operation,
                found,
                lower_bound: mid,
                upper_bound,
                find_match,
            },
            FindReducer {
                find_match: self.find_match,
            },
        )
    }

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        self.split()
    }

    fn into_folder(self) -> Self::Folder {
        FindFolder {
            operation: self.operation,
            found: self.found,
            item: None,
            find_match: self.find_match,
            lower_bound: self.lower_bound,
            upper_bound: self.upper_bound,
        }
    }

    fn is_full(&self) -> bool {
        self.found.load(Ordering::Relaxed) > 0
    }
}

/* FindFolder */

struct FindFolder<O, T> {
    operation: O,
    found: Arc<AtomicUsize>,
    item: Option<T>,
    lower_bound: usize,
    upper_bound: usize,
    find_match: FindMatch,
}

impl<O, T> Folder<T> for FindFolder<O, T>
where
    O: Fn(&T) -> bool + Clone,
{
    type Result = Option<T>;

    fn consume(mut self, item: T) -> Self {
        match self.find_match {
            FindMatch::First if self.item.is_some() => return self,
            FindMatch::Any if self.item.is_some() => return self,
            _ => (),
        }

        if (self.operation)(&item) {
            let mut found = self.found.load(Ordering::Relaxed);
            loop {
                let boundary = match self.find_match {
                    FindMatch::Any if found > 0 => return self,
                    FindMatch::First if found < self.lower_bound => return self,
                    FindMatch::Last if found > self.upper_bound => return self,
                    FindMatch::Any => self.lower_bound,
                    FindMatch::First => self.lower_bound,
                    FindMatch::Last => self.upper_bound,
                };

                let ret = self.found.compare_exchange_weak(
                    found,
                    boundary,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                );

                match ret {
                    Ok(_) => {
                        self.item = Some(item);

                        break;
                    }
                    Err(v) => found = v,
                }
            }
        }

        self
    }

    fn complete(self) -> Self::Result {
        self.item
    }

    fn is_full(&self) -> bool {
        self.found.load(Ordering::Relaxed) > 0
    }
}

/* FindReducer */

struct FindReducer {
    find_match: FindMatch,
}

impl<T> Reducer<Option<T>> for FindReducer {
    fn reduce(self, left: Option<T>, right: Option<T>) -> Option<T> {
        match self.find_match {
            FindMatch::First => left.or(right),
            FindMatch::Any => left.or(right),
            FindMatch::Last => right.or(left),
        }
    }
}
