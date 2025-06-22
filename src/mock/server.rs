use std::{future::Future, net::SocketAddr, path::PathBuf};

use anyhow::Context;
use tokio::net::TcpListener;

use crate::error::Result;

use super::{
    routes,
    state::MockState,
    storage::{MockStorage, default_data_dir},
};

#[derive(Clone, Debug)]
pub struct MockServerOptions {
    pub host: String,
    pub port: u16,
    pub data_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub struct BoundMockServer {
    local_addr: SocketAddr,
    state: MockState,
    listener: TcpListener,
}

impl BoundMockServer {
    pub async fn bind(options: MockServerOptions) -> Result<Self> {
        let bind_addr = format!("{}:{}", options.host, options.port);
        let listener = TcpListener::bind(&bind_addr)
            .await
            .with_context(|| format!("failed to bind mock server to {bind_addr}"))?;
        let local_addr = listener
            .local_addr()
            .context("failed to read mock server local address")?;
        let data_dir = match options.data_dir {
            Some(data_dir) => data_dir,
            None => default_data_dir()?,
        };
        let state = MockState::reset_new(MockStorage::new(data_dir))?;

        Ok(Self {
            local_addr,
            state,
            listener,
        })
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.local_addr)
    }

    pub fn data_dir(&self) -> &std::path::Path {
        self.state.data_dir()
    }

    pub async fn serve_until_shutdown<F>(self, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let state = self.state.clone();
        let result = axum::serve(self.listener, routes::router(self.state))
            .with_graceful_shutdown(shutdown)
            .await
            .context("mock server failed");
        let reset_result = state.reset();

        result?;
        reset_result
    }
}
