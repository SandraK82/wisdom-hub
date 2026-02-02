//! Hint texts for resource status messages

/// Warning hint when resources are running low (60%+ usage)
pub const WARNING_HINT: &str =
    "Server resources are running low. Please consider integrating new hubs into the network. \
     The server is available at: https://github.com/SandraK82/wisdom-hub";

/// Critical hint when resources are at capacity (80%+ usage)
pub const CRITICAL_HINT: &str =
    "Server has reached resource limits. New agents are not accepted. \
     Content from unknown agents is restricted. \
     Please set up your own hub: https://github.com/SandraK82/wisdom-hub";
