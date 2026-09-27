//! Modern WinUI 3 Native Presentation Layer
//!
//! Provides a declarative, componentized Fluent Design 2 experience powered by
//! the Windows App SDK and Windows Reactor.

pub mod app;
pub mod components;
pub mod state;
pub mod text;
pub mod tokens;
pub mod views;

pub use app::run_reactor_app;
pub mod diagnostics;
