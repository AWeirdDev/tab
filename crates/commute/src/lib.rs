#![no_std]

extern crate alloc;

use alloc::string::String;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum MessageContent {
    Text(String),
    // Image(String), <- haven't landed yet, thanks
    // File
    // Interaction
}

/// A commutable interface.
///
/// Note that all functions should be synchronous.
///
/// When the interaction fails, it's the implementer's
/// job to deal with it, thus there is no `Result` on return.
///
/// # Example
///
/// ```
/// # struct Message;
/// # impl Message {
/// #   fn react_with(&self, _emoji: &str) {}
/// #   fn reply(&self, _text: &str) -> usize { 0 }
/// # }
/// # fn get_message(_id: usize) -> Message { Message }
/// # fn send_message(_text: &str) -> usize { 0 }
/// #
/// use commute::{Commute, MessageContent};
///
/// // Let's say some text-based platform
/// struct SomePlatform;
///
/// impl Commute for SomePlatform {
///     type MessageId = usize;
///
///     fn react(&self, message_id: Self::MessageId, emoji: &str) {
///         let message = get_message(message_id);
///         message.react_with(emoji);
///     }
///
///     fn reply(
///         &self,
///         message_id: Self::MessageId,
///         content: MessageContent
///     ) -> Self::MessageId {
///         let message = get_message(message_id);
///         match content {
///             MessageContent::Text(text) => message.reply(&text),
///
///             // note: non-exhaustive
///             // because this imaginary platform is text-based, other
///             // types of message content is not supported, so we'll
///             // return a sentinel value
///             _ => usize::MAX,
///         }
///     }
///
///     fn send(
///         &self,
///         content: MessageContent
///     ) -> Self::MessageId {
///         match content {
///             MessageContent::Text(text) => send_message(&text),
///             _ => usize::MAX,
///         }
///     }
/// }
/// ```
pub trait Commute {
    /// The associated message ID type.
    type MessageId: Clone;

    /// React to a message with an emoji.
    fn react(&self, message_id: Self::MessageId, emoji: &str);

    /// Reply to a specific message.
    fn reply(&self, message_id: Self::MessageId, content: MessageContent) -> Self::MessageId;

    /// Send a message.
    fn send(&self, content: MessageContent) -> Self::MessageId;
}
