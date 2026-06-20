use std::{
    marker::PhantomData,
    sync::atomic::{
        AtomicU8,
        Ordering,
    },
};

use anyhow::{
    Result,
    anyhow,
    bail,
};

// Structs
#[derive(Debug)]
pub struct AtomicEnumCell<T: Eq + Into<u8> + PartialEq + TryFrom<u8>> {
    _marker: PhantomData<T>,
    inner: AtomicU8,
}

impl<T: Eq + Into<u8> + PartialEq + TryFrom<u8>> AtomicEnumCell<T> {
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            _marker: PhantomData,
            inner: AtomicU8::new(value.into()),
        }
    }

    // Public methods
    #[inline]
    pub fn ensure<F: FnOnce(T) -> String>(&self, expected: T, message: F) -> Result<()> {
        let value = self.get();
        if value != expected {
            bail!(message(value));
        }

        Ok(())
    }

    #[inline]
    pub fn get(&self) -> T {
        #[allow(
            clippy::expect_used,
            reason = "AtomicEnumCell only writes values converted from T, so an invalid discriminant indicates a broken internal invariant."
        )]
        T::try_from(self.inner.load(Ordering::SeqCst))
            .ok()
            .expect("AtomicEnumCell contained an invalid enum discriminant")
    }

    #[inline]
    pub fn is(&self, value: T) -> bool {
        self.inner.load(Ordering::SeqCst) == value.into()
    }

    #[inline]
    pub fn store(&self, value: T) {
        self.inner.store(value.into(), Ordering::SeqCst);
    }

    #[inline]
    pub fn try_transition(&self, from: T, to: T) -> Result<()> {
        self.inner
            .compare_exchange(from.into(), to.into(), Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| anyhow!("Failed to transition value"))
    }
}

#[cfg(test)]
mod tests {
    use super::AtomicEnumCell;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    }

    #[test]
    fn ensure_returns_ok_for_expected_value() {
        let cell = AtomicEnumCell::new(TestState::Ready);

        let result = cell.ensure(TestState::Ready, |_| "unexpected state".to_owned());

        assert!(result.is_ok());
    }

    #[test]
    fn ensure_uses_current_value_in_error_message() {
        let cell = AtomicEnumCell::new(TestState::Done);

        let result = cell.ensure(TestState::Ready, |actual| format!("actual: {actual:?}"));

        match result {
            Ok(()) => panic!("mismatched state should fail"),
            Err(error) => assert_eq!(error.to_string(), "actual: Done"),
        }
    }

    #[test]
    fn try_transition_updates_only_when_current_value_matches() {
        let cell = AtomicEnumCell::new(TestState::Init);

        let result = cell.try_transition(TestState::Init, TestState::Ready);

        assert!(result.is_ok());
        assert_eq!(cell.get(), TestState::Ready);

        let result = cell.try_transition(TestState::Init, TestState::Done);

        match result {
            Ok(()) => panic!("transition from stale state should fail"),
            Err(error) => assert_eq!(error.to_string(), "Failed to transition value"),
        }

        assert_eq!(cell.get(), TestState::Ready);
    }
}
