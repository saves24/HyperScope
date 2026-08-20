// hyper-panel-core: shared business logic (no HTTP/axum dependency)
pub const VERSION: &str = "0.1.0";
// Used by both the Linux web panel and the future Windows desktop client.
pub mod client;
pub mod history;
pub mod logging;
pub mod nodes;
pub mod poller;
pub mod state;
pub mod util;

pub use client::*;
pub use history::*;
pub use logging::*;
pub use nodes::*;
pub use poller::*;
pub use state::*;
pub use util::*;
