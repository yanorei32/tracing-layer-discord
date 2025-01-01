#![doc = include_str!("../README.md")]
pub use config::DiscordConfig;
pub use layer::DiscordLayer;
pub use layer::DiscordLayerBuilder;
pub use worker::BackgroundWorker;

use crate::worker::WorkerMessage;

mod config;
mod layer;
mod message;
mod worker;

pub(crate) type ChannelSender = tokio::sync::mpsc::UnboundedSender<WorkerMessage>;
pub(crate) type ChannelReceiver = tokio::sync::mpsc::UnboundedReceiver<WorkerMessage>;
