//! AI Provider implementations
//!
//! This module contains concrete implementations of the AIProvider trait
//! for various AI services (OpenAI, Anthropic, Gemini, etc.)

pub mod openai;

pub use openai::OpenAIProvider;
