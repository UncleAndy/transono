//! Concrete AI provider implementations.
//!
//! Application code should depend on [`crate::core::provider::Provider`]
//! and pick a backend from this module (today: OpenAI).

pub mod openai;
