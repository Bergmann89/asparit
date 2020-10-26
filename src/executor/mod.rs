mod sequential;

pub use sequential::Sequential as SequentialExecutor;

pub type DefaultExecutor = SequentialExecutor;
