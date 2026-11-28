use std::{
    error::Error,
    fmt,
    marker::PhantomData,
    sync::atomic::{
        AtomicU8,
        Ordering,
    },
};

use anyhow::{
    Result,
    ensure,
};

// Structs

/// `Into<u8>` and `TryFrom<u8>` must round-trip every value; otherwise every method that returns the
/// stored value as `T`, including the error paths of [`ensure`](Self::ensure) and
/// [`try_transition`](Self::try_transition), panics.
pub struct AtomicEnumCell<T> {
    _marker: PhantomData<T>,
    inner: AtomicU8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TransitionError<T> {
    pub actual: T,
    pub expected: T,
    pub target: T,
}

impl<T: fmt::Debug> fmt::Display for TransitionError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid transition {:?} -> {:?}: current value is {:?}",
            self.expected, self.target, self.actual
        )
    }
}

impl<T: fmt::Debug> Error for TransitionError<T> {}

impl<T: fmt::Debug + TryFrom<u8>> fmt::Debug for AtomicEnumCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw = self.inner.load(Ordering::SeqCst);
        let mut tuple = f.debug_tuple("AtomicEnumCell");
        match T::try_from(raw) {
            Ok(value) => tuple.field(&value),
            Err(_) => tuple.field(&raw),
        };

        tuple.finish()
    }
}

impl<T: Copy + Into<u8> + TryFrom<u8>> AtomicEnumCell<T> {
    pub fn new(value: T) -> Self {
        Self {
            _marker: PhantomData,
            inner: AtomicU8::new(value.into()),
        }
    }

    // Public methods
    pub fn ensure<M, F>(&self, expected: T, message: F) -> Result<()>
    where
        M: fmt::Debug + fmt::Display + Send + Sync + 'static,
        F: FnOnce(T) -> M,
    {
        let raw = self.inner.load(Ordering::SeqCst);
        ensure!(raw == expected.into(), message(Self::decode(raw)));
        Ok(())
    }

    pub fn get(&self) -> T {
        Self::decode(self.inner.load(Ordering::SeqCst))
    }

    pub fn is(&self, value: T) -> bool {
        self.inner.load(Ordering::SeqCst) == value.into()
    }

    pub fn store(&self, value: T) {
        self.inner.store(value.into(), Ordering::SeqCst);
    }

    pub fn try_transition(&self, from: T, to: T) -> Result<(), TransitionError<T>> {
        self.inner
            .compare_exchange(from.into(), to.into(), Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|actual| TransitionError {
                actual: Self::decode(actual),
                expected: from,
                target: to,
            })
    }

    /// Atomically replaces the value with `f(current)` unless `f` returns `None`, returning
    /// `Ok(previous)` or `Err(current)`. `f` may run several times under contention, so it must be
    /// free of side effects.
    pub fn try_update<F: FnMut(T) -> Option<T>>(&self, mut f: F) -> Result<T, T> {
        self.inner
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |raw| {
                f(Self::decode(raw)).map(Into::into)
            })
            .map(Self::decode)
            .map_err(Self::decode)
    }

    // Private methods
    fn decode(raw: u8) -> T {
        #[allow(
            clippy::expect_used,
            reason = "AtomicEnumCell only writes values converted from T, so an invalid discriminant indicates a broken internal invariant."
        )]
        T::try_from(raw)
            .ok()
            .expect("AtomicEnumCell contained an invalid enum discriminant")
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{
        AtomicEnumCell,
        TransitionError,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestState {
        Init,
        Ready,
        Done,
    }

    impl From<TestState> for u8 {
        fn from(value: TestState) -> Self {
            match value {
                TestState::Init => 0,
                TestState::Ready => 1,
                TestState::Done => 2,
            }
        }
    }

    impl TryFrom<u8> for TestState {
        type Error = ();

        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                0 => Ok(Self::Init),
                1 => Ok(Self::Ready),
                2 => Ok(Self::Done),
                _ => Err(()),
            }
        }
    }

    #[test]
    fn new_get_is_and_store_round_trip_enum_values() {
        let cell = AtomicEnumCell::new(TestState::Init);

        assert_eq!(cell.get(), TestState::Init);
        assert!(cell.is(TestState::Init));
        assert!(!cell.is(TestState::Ready));

        cell.store(TestState::Ready);

        assert_eq!(cell.get(), TestState::Ready);
        assert!(cell.is(TestState::Ready));
        assert_eq!(format!("{cell:?}"), "AtomicEnumCell(Ready)");
    }

    #[test]
    fn ensure_evaluates_message_only_on_mismatch() {
        let cell = AtomicEnumCell::new(TestState::Done);

        assert!(
            cell.ensure(TestState::Done, |_| panic!("message must not be evaluated on success"))
                .is_ok()
        );

        let error = cell
            .ensure(TestState::Ready, |actual| format!("actual: {actual:?}"))
            .expect_err("mismatched state should fail");

        assert_eq!(error.to_string(), "actual: Done");
    }

    #[test]
    fn try_transition_updates_only_when_current_value_matches() {
        let cell = AtomicEnumCell::new(TestState::Init);

        assert!(cell.try_transition(TestState::Init, TestState::Ready).is_ok());
        assert_eq!(cell.get(), TestState::Ready);

        let error = cell
            .try_transition(TestState::Init, TestState::Done)
            .expect_err("transition from stale state should fail");

        assert_eq!(
            error,
            TransitionError {
                actual: TestState::Ready,
                expected: TestState::Init,
                target: TestState::Done,
            }
        );

        assert_eq!(
            error.to_string(),
            "invalid transition Init -> Done: current value is Ready"
        );

        assert_eq!(cell.get(), TestState::Ready);
    }

    #[test]
    fn try_update_applies_closure_result_or_returns_current_value() {
        let cell = AtomicEnumCell::new(TestState::Init);

        assert_eq!(cell.try_update(|_| Some(TestState::Ready)), Ok(TestState::Init));
        assert_eq!(cell.get(), TestState::Ready);

        assert_eq!(cell.try_update(|_| None), Err(TestState::Ready));
        assert_eq!(cell.get(), TestState::Ready);
    }
}
