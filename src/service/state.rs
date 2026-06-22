use num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, IntoPrimitive, PartialEq, TryFromPrimitive)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
}
