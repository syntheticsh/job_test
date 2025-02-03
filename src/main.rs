use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use clap::Parser;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;

type SharedState = Arc<RwLock<HashMap<String, usize>>>;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    listening_address: String,
    #[arg(short, long, default_value_t = 5)]
    timeout: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let addr = match args.listening_address.parse::<std::net::SocketAddr>() {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("Invalid address: {err}");
            return;
        }
    };
    let socket = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Couldn't bind socket");
    println!("Listening on {}", addr);

    let state: SharedState = Arc::new(RwLock::new(HashMap::new()));

    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn_with_state(state.clone(), track_ip))
                .layer(TimeoutLayer::new(Duration::from_secs(args.timeout))),
        );

    tokio::spawn(ip_printer(state));

    axum::serve(
        socket,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Couldn't start server");

    println!("Server stopped");
}

async fn track_ip(
    State(state): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let ip = addr.ip().to_string();

    let mut state = state.write().await;
    state.entry(ip).and_modify(|e| *e += 1).or_insert(1);

    next.run(req).await
}

async fn ip_printer(state: SharedState) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let state = state.read().await;
        let mut addr_vec = state.iter().collect::<Vec<_>>();
        addr_vec.sort_by(|a, b| b.1.cmp(a.1));
        let mut output = String::from("IPs:\n");
        for (addr, count) in addr_vec.iter() {
            output.push_str(&format!("\t{}: {}\n", addr, count));
        }

        println!("{}", output);
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl-c event");
    println!("Shutting down...");
}
