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
pub mod interleave;
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
pub mod splits;
pub mod sum;
pub mod take;
pub mod try_fold;
pub mod try_for_each;
pub mod try_reduce;
pub mod unzip;
pub mod update;
pub mod while_some;
pub mod zip;

#[cfg(test)]
mod tests {
    use crate::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_for_each() {
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

        a.par_iter()
            .cloned()
            .chain(b)
            .with_splits(1)
            .interleave_shortest(
                vec![vec![50, 51], vec![52, 53], vec![54, 55]]
                    .into_par_iter()
                    .take(2),
            )
            .for_each(|x| {
                dbg!(x);
            })
            .exec()
            .await;
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
