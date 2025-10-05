use anyhow::{anyhow, Result};
use std::fmt::Debug;

// OptionAnyhowExt
pub trait OptionAnyhowExt<T> {
    fn ok_anyhow<M>(self, msg: M) -> Result<T>
    where
        M: AsRef<str>;
}

impl<T> OptionAnyhowExt<T> for Option<T> {
    fn ok_anyhow<M>(self, msg: M) -> Result<T>
    where
        M: AsRef<str>,
    {
        self.ok_or_else(|| anyhow!(msg.as_ref().to_string()))
    }
}

// ResultAnyhowExt
pub trait ResultAnyhowExt<T> {
    fn map_anyhow(self) -> Result<T>;
}

impl<T, E> ResultAnyhowExt<T> for Result<T, E>
where
    E: Debug,
{
    fn map_anyhow(self) -> Result<T> {
        self.map_err(|err| anyhow!("{:#?}", err))
    }
}
