/// The reducer is the final step of a `Consumer` -- after a consumer
/// has been split into two parts, and each of those parts has been
/// fully processed, we are left with two results. The reducer is then
/// used to combine those two results into one. See [the `plumbing`
/// README][r] for further details.
///
/// [r]: https://github.com/rayon-rs/rayon/blob/master/src/iter/plumbing/README.md
pub trait Reducer<Result> {
    /// Reduce two final results into one; this is executed after a
    /// split.
    fn reduce(self, left: Result, right: Result) -> Result;
}
