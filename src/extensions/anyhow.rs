use anyhow::{anyhow, Result};
use std::fmt::Debug;

pub trait OptionAnyhowExt<T> {
    fn ok_anyhow(self, msg: impl AsRef<str>) -> Result<T>;
}

pub trait ResultAnyhowExt<T> {
    fn map_anyhow(self) -> Result<T>;
}

impl<T> OptionAnyhowExt<T> for Option<T> {
    fn ok_anyhow(self, msg: impl AsRef<str>) -> Result<T> {
        return self.ok_or_else(|| anyhow!("{}", msg.as_ref()));
    }
}

impl<T, E: Debug> ResultAnyhowExt<T> for Result<T, E> {
    fn map_anyhow(self) -> Result<T> {
        return self.map_err(|err| anyhow!("{:#?}", err));
    }
}
