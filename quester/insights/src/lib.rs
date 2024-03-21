use common::Pool;
use error::DiscoveryError;
use std::net::SocketAddr;
mod client;
mod error;
mod service;

pub type InsightsPool = Pool<SocketAddr, InsightsServiceClient>;

pub type Result<T> = std::result::Result<T, DiscoveryError>;
