// hyper-panel-core: shared business logic (no HTTP/axum dependency)
pub const VERSION: &str = "1.0.0";
// Used by both the Linux web panel and the future Windows desktop client.
pub mod alerts;
pub mod client;
pub mod history;
pub mod identity;
pub mod logging;
pub mod nodes;
pub mod poller;
pub mod protocol;
pub mod relay_client;
pub mod relay_tls;
pub mod state;
pub mod util;
pub mod webhook;

pub use client::*;
pub use history::*;
pub use logging::*;
pub use nodes::*;
pub use poller::*;
pub use protocol::*;
pub use state::*;
pub use util::*;
