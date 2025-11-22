//! API layer: REST (Axum) and gRPC (Tonic)
pub mod rest;
pub mod grpc;

pub struct ApiServer {}
impl ApiServer {
    pub fn new() -> Self { Self {} }
}
impl Default for ApiServer {
    fn default() -> Self { Self::new() }
}
