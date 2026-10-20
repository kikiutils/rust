use dashmap::{
    DashMap,
    DashSet,
};
use rustc_hash::FxBuildHasher;
pub use rustc_hash::{
    FxHashMap,
    FxHashSet,
};

pub type FxDashMap<K, V> = DashMap<K, V, FxBuildHasher>;
pub type FxDashSet<K> = DashSet<K, FxBuildHasher>;
