use axum::extract::{ConnectInfo, Request};
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tokio::sync::mpsc::UnboundedSender;
use tower::Service;

type Writer = UnboundedSender<String>;

#[derive(Debug, Clone)]
pub struct IpCounterLayer {
    writer: Writer,
}

impl IpCounterLayer {
    pub fn new(state: Writer) -> Self {
        IpCounterLayer { writer: state }
    }
}

impl<S> tower::Layer<S> for IpCounterLayer {
    type Service = IpCounter<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpCounter::new(inner, self.writer.clone())
    }
}

#[derive(Debug, Clone)]
pub struct IpCounter<S> {
    inner: S,
    writer: Writer,
}

impl<S> IpCounter<S> {
    fn new(inner: S, state: Writer) -> Self {
        IpCounter { inner, writer: state }
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

            self.writer.send(ip).expect("Couldn't send IP to counter");
        }

        self.inner.call(request)
    }
}
