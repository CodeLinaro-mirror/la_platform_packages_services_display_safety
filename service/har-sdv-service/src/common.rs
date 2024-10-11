use futures::executor::block_on;
use grpcio::Server;

// Defines where the GRPC server listens at.
pub const DRIVERUI_RPC_SERVER_PORT: i32 = 7000;
pub const DRIVERUI_RPC_SERVER_HOST: &str = "0.0.0.0";

pub const CAMERA_RPC_SERVER_PORT: i32 = 8000;

// Defines where the GRPC proxy connects to.
pub const DRIVERUI_RPC_CLIENT_ADDRESS: &str = "127.0.0.1:7001";
pub const CAMERA_RPC_CLIENT_ADDRESS: &str = "127.0.0.1:8001";

/// A token for a running server. Dropping this will stop the server.
pub struct GrpcProxyServerToken(pub Server);

impl GrpcProxyServerToken {
    /// Shutdown the server
    pub fn shutdown(&mut self) {
        let _ = block_on(self.0.shutdown());
    }
}

impl Drop for GrpcProxyServerToken {
    fn drop(&mut self) {
        self.shutdown();
    }
}
