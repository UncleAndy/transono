//! Wire-format encode/decode between domain commands/events and transport bytes.
//!
//! [`Protocol`] is independent of how bytes are carried; pairing with a
//! [`crate::core::transport::Transport`] keeps vendor schemas swappable.

use crate::core::error::Result;
use crate::core::transport::TransportData;

/// Encode commands and decode events for a specific remote API schema.
pub trait Protocol: Send + Sync + 'static {
    /// Outbound command type for this protocol.
    type Command;
    /// Inbound event type for this protocol.
    type Event;

    /// Relative or absolute endpoint path associated with this protocol.
    const ENDPOINT: &'static str;

    /// Serialize a domain command into transport payload.
    ///
    /// # Errors
    ///
    /// Returns protocol errors if encoding fails.
    fn encode(
        &self,
        command: &Self::Command,
    ) -> Result<TransportData>;

    /// Deserialize a transport payload into a domain event.
    ///
    /// # Errors
    ///
    /// Returns protocol errors if the payload is invalid or unexpected.
    fn decode(
        &self,
        data: TransportData,
    ) -> Result<Self::Event>;
}
