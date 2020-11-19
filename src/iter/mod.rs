pub mod chain;
pub mod chunks;
pub mod cloned;
pub mod cmp;
pub mod collect;
pub mod copied;
pub mod count;
pub mod enumerate;
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
pub mod position;
pub mod product;
pub mod reduce;
pub mod rev;
pub mod setup;
pub mod skip;
pub mod step_by;
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
        let x: Vec<usize> = vec![0usize, 1, 2, 3, 4, 5]
            .into_par_iter()
            .rev()
            .collect()
            .exec()
            .await;

        dbg!(x);
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
