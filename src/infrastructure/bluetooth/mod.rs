//! Bluetooth Module
//!
//! Provides BLE communication with the Gear VR Controller.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    BluetoothService                      │
//! │  (Main coordinator - public API for the application)     │
//! └─────────────────────┬───────────────────────────────────┘
//!                       │
//!         ┌─────────────┼─────────────┐
//!         │             │             │
//!         ▼             ▼             ▼
//! ┌───────────┐  ┌────────────┐  ┌──────────┐
//! │  Scanner  │  │ Connection │  │ Protocol │
//! │           │  │            │  │          │
//! │ - BLE     │  │ - Pairing  │  │ - UUIDs  │
//! │   discovery│ │ - GATT     │  │ - Commands│
//! │           │  │   access   │  │ - Parsing │
//! └───────────┘  └────────────┘  └──────────┘
//! ```
//!
//! ## Modules
//!
//! - [`protocol`] - Controller protocol definitions, commands, and data parsing
//! - [`scanner`] - BLE device discovery
//! - [`connection`] - Device connection, pairing, and GATT service handling
//! - [`service`] - Main service coordinator

pub mod connection;
pub mod protocol;
pub mod scanner;
pub mod service;

// Re-export main service for convenience
pub use service::BluetoothService;
