//! Client errors are errors that can occur when using the [`Client`](crate::client::Client) api.
use crate::connection::queue::SendQueueError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClientError {
    /// The client is already connected to the peer on this address.
    AddrBindErr,
    /// The client is already connected a peer.
    AlreadyOnline,
    /// The client is offline and can not send packets.
    NotListening,
    /// The client is unable to connect to the peer.
    Unavailable,
    /// The client is unable to connect to the peer because the peer is using a different protocol version.
    IncompatibleProtocolVersion,
    /// The client has been closed.
    Killed,
    /// The client has been closed, and can not be used again.
    Reset,
    /// The client is unable to connect to the peer because the peer is offline.
    ServerOffline,
    /// The client failed to process a packet you sent.
    SendQueueError(SendQueueError),
}