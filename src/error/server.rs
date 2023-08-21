//! Server errors
//! Server errors are errors that can occur when using the [`Listener`](crate::server::Listener) api.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ServerError {
    /// The server is unable to bind to the given address.
    AddrBindErr,
    /// The server is already online and can not be started again.
    AlreadyOnline,
    /// The server is offline and can not send packets.
    NotListening,
    /// The server has been closed.
    Killed,
    /// The server has been closed, and can not be used again.
    Reset,
}
