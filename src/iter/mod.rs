pub mod chain;
pub mod cloned;
pub mod collect;
pub mod copied;
pub mod count;
pub mod filter;
pub mod filter_map;
pub mod find;
pub mod flatten;
pub mod fold;
pub mod for_each;
pub mod inspect;
pub mod intersperse;
pub mod map;
pub mod map_init;
pub mod map_with;
pub mod max;
pub mod min;
pub mod noop;
pub mod panic_fuse;
pub mod partition;
pub mod product;
pub mod reduce;
pub mod sum;
pub mod try_fold;
pub mod try_for_each;
pub mod try_reduce;
pub mod unzip;
pub mod update;
pub mod while_some;

#[cfg(test)]
mod tests {
    use crate::*;

    #[tokio::test]
    async fn test_for_each() {
        use ::std::sync::atomic::{AtomicUsize, Ordering};
        use ::std::sync::Arc;

        let i = Arc::new(AtomicUsize::new(0));
        let j = Arc::new(AtomicUsize::new(0));

        let a = vec![
            vec![1usize, 2usize],
            vec![3usize, 4usize],
            vec![5usize, 6usize],
        ];
        let b = vec![
            vec![7usize, 8usize],
            vec![9usize, 10usize],
            vec![11usize, 12usize],
        ];

        let (x, y, z): (Vec<_>, Vec<_>, Vec<_>) = a
            .par_iter()
            .cloned()
            .chain(b)
            .update(|x| x.push(0))
            .flatten_iter()
            .intersperse(100)
            .panic_fuse()
            .map(Some)
            .while_some()
            .map_init(
                move || i.fetch_add(1, Ordering::Relaxed),
                |init, item| (*init, item),
            )
            .map_init(
                move || j.fetch_add(2, Ordering::Relaxed),
                |init, (init2, item)| (*init, init2, item),
            )
            .partition_map(|(i, j, k)| match j % 3 {
                0 => (Some(i), None, None),
                1 => (None, Some(j), None),
                2 => (None, None, Some(k)),
                _ => unreachable!(),
            })
            .exec()
            .await;

        dbg!(&x);
        dbg!(&y);
        dbg!(&z);
    }

    #[tokio::test]
    async fn test_reduce() {
        let x = (0..10usize)
            .into_par_iter()
            .map::<_, Result<usize, ()>>(Ok)
            .try_reduce(|| 0, |a, b| Ok(a + b))
            .exec()
            .await;

        dbg!(&x);

        assert_eq!(Ok(45), x);
    }
}
