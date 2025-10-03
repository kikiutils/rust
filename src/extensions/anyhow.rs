use anyhow::{anyhow, Result};
use std::fmt::Debug;

// OptionAnyhowExt
pub trait OptionAnyhowExt<T> {
    fn ok_anyhow(self, msg: impl AsRef<str>) -> Result<T>;
}

impl<T> OptionAnyhowExt<T> for Option<T> {
    fn ok_anyhow(self, msg: impl AsRef<str>) -> Result<T> {
        self.ok_or_else(|| anyhow!("{}", msg.as_ref()))
    }
}

// ResultAnyhowExt
pub trait ResultAnyhowExt<T> {
    fn map_anyhow(self) -> Result<T>;
}

impl<T, E: Debug> ResultAnyhowExt<T> for Result<T, E> {
    fn map_anyhow(self) -> Result<T> {
        self.map_err(|err| anyhow!("{:#?}", err))
    }
}
