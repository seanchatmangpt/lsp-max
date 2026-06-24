pub mod config;
pub mod fanout;
pub mod registry;

pub use config::CompositorConfig;
pub use fanout::{DispatchStrategy, dispatch_strategy, servers_for_uri};
pub use registry::{ChildServer, ChildTier, ExtensionRouter};
