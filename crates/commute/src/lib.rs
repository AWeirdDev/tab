#![no_std]

/// A commutable interface.
///
/// Note that all functions should be synchronous.
pub trait Commute {
    /// React to a message with an emoji.
    fn react(&self, message_id: usize, emoji: &str);

    /// Reply to a specific message.
    fn reply(&self, message_id: usize, text: &str);

    /// Send a message.
    fn send(&self, text: &str);
}
