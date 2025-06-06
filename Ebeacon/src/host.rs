// src/host.rs
// -------------------------------------------------------------
// Represents a specific computer (node/peer) in the cyber system.
// This module provides a struct that encapsulates the host's identity,
// typically using the system's hostname. This identity is used by other
// layers (e.g., Network, Service, Application) to identify which node is
// executing a transaction.
// -------------------------------------------------------------
use whoami::fallible::hostname;
use std::fmt;

pub struct Host {
    /// A unique identifier for the host, typically derived from the system hostname.
    pub hostname: String,
}

impl Host {
    /// Creates a new Host instance using the current system hostname.
    pub fn new() -> Self {
        let host_id = hostname().unwrap_or_else(|_| "unknown".to_string());
        Host { hostname: host_id }
    }

    /// Returns the host's identifier.
    pub fn id(&self) -> &str {
        &self.hostname
    }
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Host: {}", self.hostname)
    }
}

impl fmt::Debug for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Host {{ hostname: {} }}", self.hostname)
    }
}
