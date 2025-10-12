use std::fmt::Debug;

use anyhow::{
    anyhow,
    Result,
};

// OptionAnyhowExt
pub trait OptionAnyhowExt<T> {
    fn ok_anyhow<M: AsRef<str>>(self, msg: M) -> Result<T>;
}

impl<T> OptionAnyhowExt<T> for Option<T> {
    fn ok_anyhow<M: AsRef<str>>(self, msg: M) -> Result<T> {
        self.ok_or_else(|| anyhow!(msg.as_ref().to_string()))
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
