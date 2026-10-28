//! Local automation API for Zapdude.
//!
//! A loopback HTTP server usable by humans or agents: send messages, open
//! chats, list chats, and stream WhatsApp Web events over SSE or a webhook.
//! Disabled by default; enabled through the persisted `config.json`.

pub mod events;
pub mod page_ops;
pub mod protocol;
pub mod server;
