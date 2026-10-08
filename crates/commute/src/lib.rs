#![no_std]

extern crate alloc;

use alloc::string::String;

/// A marker trait which represents a message ID.
/// It's explicitly separated to express intent,
/// despite the fact that it only needs `Clone`.
pub trait MessageId: Clone {}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Image(String),
    // File
    // Interaction
}

/// A commutable interface.
///
/// Note that all functions should be synchronous.
///
/// When the interaction fails, it's the implementer's
/// job to deal with it, thus there is no `Result` on return.
pub trait Commute {
    /// The associated message ID type.
    type MessageId: MessageId;

    /// React to a message with an emoji.
    fn react(&self, message_id: Self::MessageId, emoji: &str);

    /// Reply to a specific message.
    fn reply(&self, message_id: Self::MessageId, content: MessageContent) -> impl MessageId;

    /// Send a message.
    fn send(&self, content: MessageContent) -> Self::MessageId;
}
