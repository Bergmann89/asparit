pub mod cloned;
pub mod collect;
pub mod copied;
pub mod count;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod fold;
pub mod for_each;
pub mod inspect;
pub mod map;
pub mod map_init;
pub mod map_with;
pub mod noop;
pub mod product;
pub mod reduce;
pub mod sum;
pub mod try_fold;
pub mod try_for_each;
pub mod try_reduce;
pub mod update;

#[cfg(test)]
mod tests {
    use crate::*;

    #[tokio::test]
    async fn test_for_each() {
        use ::std::sync::atomic::{AtomicUsize, Ordering};
        use ::std::sync::Arc;

        let i = Arc::new(AtomicUsize::new(0));
        let j = Arc::new(AtomicUsize::new(0));

        let x = vec![
            vec![1usize, 2usize],
            vec![3usize, 4usize],
            vec![5usize, 6usize],
        ];

        let x = x
            .par_iter()
            .cloned()
            .update(|x| x.push(0))
            .flatten_iter()
            .map_init(
                move || i.fetch_add(1, Ordering::Relaxed),
                |init, item| (item, *init),
            )
            .try_fold_with(String::new(), |s, item| -> Result<String, ()> {
                Ok(format!("{} + {:?}", s, item))
            })
            .try_for_each_init(
                move || j.fetch_add(1, Ordering::Relaxed),
                |init, item| -> Result<(), ()> {
                    println!("{:?} - {:?}", init, item);
                    Ok(())
                },
            )
            .exec()
            .await;

        dbg!(&x);
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
