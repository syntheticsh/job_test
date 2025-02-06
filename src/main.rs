mod ip_counter;

use axum::{routing::get, Router};
use clap::Parser;
use ip_counter::IpCounterLayer;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;

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
    eprintln!("Listening on {}", addr);

    let (writer, reader) = unbounded_channel();

    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .layer(
            ServiceBuilder::new()
                .layer(IpCounterLayer::new(writer))
                .layer(TimeoutLayer::new(Duration::from_secs(args.timeout))),
        );

    tokio::spawn(ip_printer(reader));

    axum::serve(
        socket,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Couldn't start server");

    eprintln!("Server stopped");
}

async fn ip_printer(mut receiver: UnboundedReceiver<String>) {
    let mut state = HashMap::new();
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        while let Ok(ip) = receiver.try_recv() {
            state.entry(ip).and_modify(|e| *e += 1).or_insert(1);
        }
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
    eprintln!("Shutting down...");
}
