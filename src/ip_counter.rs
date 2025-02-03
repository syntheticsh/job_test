use axum::extract::{ConnectInfo, Request};
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tower::Service;

use crate::SharedState;

#[derive(Debug, Clone)]
pub struct IpCounterLayer {
    state: SharedState,
}

impl IpCounterLayer {
    pub fn new(state: SharedState) -> Self {
        IpCounterLayer { state }
    }
}

impl<S> tower::Layer<S> for IpCounterLayer {
    type Service = IpCounter<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpCounter::new(inner, self.state.clone())
    }
}

#[derive(Debug, Clone)]
pub struct IpCounter<S> {
    inner: S,
    state: SharedState,
}

impl<S> IpCounter<S> {
    fn new(inner: S, state: SharedState) -> Self {
        IpCounter { inner, state }
    }
}

impl<S, ReqBody> Service<Request<ReqBody>> for IpCounter<S>
where
    S: Service<Request<ReqBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        if let Some(addr) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
            let ip = addr.ip().to_string();

            let mut state = self.state.write().unwrap();
            state.entry(ip).and_modify(|e| *e += 1).or_insert(1);
        }

        self.inner.call(request)
    }
}
