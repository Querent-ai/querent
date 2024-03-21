use std::net::SocketAddr;
use common::Pool;
mod service_client;
use service_client::InsightsServiceClient;


pub type InsightsPool = Pool<SocketAddr, InsightsServiceClient>;
