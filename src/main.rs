use crate::{
    app::App,
    otel::{AmaruTraceService, TraceCollector},
    tui::Tui,
};
use amaru_stores::rocksdb::{ReadOnlyRocksDB, consensus::RocksDBStore};
use clap::Parser;
use cli::Cli;
use color_eyre::Result;
use opentelemetry_proto::tonic::collector::trace::v1::trace_service_server::TraceServiceServer;
use std::{path::PathBuf, str::FromStr, time::Duration};
use tokio::task;
use tonic::transport::Server;

mod app;
mod app_state;
mod cli;
mod config;
mod controller;
mod errors;
mod logging;
mod model;
mod otel;
mod states;
mod store;
mod tui;
mod ui;
mod update;
mod view;

#[tokio::main]
async fn main() -> Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let collector = TraceCollector::new(100, Duration::from_secs(10 * 60));
    let flamegraph_data = collector.flamegraph_snapshot();

    task::spawn(async move {
        let addr = "0.0.0.0:4317".parse().unwrap();
        Server::builder()
            .add_service(TraceServiceServer::new(AmaruTraceService::new(collector)))
            .serve(addr)
            .await
            .unwrap();
    });

    let args = Cli::parse();

    let ledger_path = PathBuf::from_str(&args.ledger_db)?;
    let chain_path = PathBuf::from_str(&args.chain_db)?;

    let ledger_db = ReadOnlyRocksDB::new(&ledger_path)?;
    let chain_db = RocksDBStore::open_for_readonly(&chain_path)?;

    let mut tui = Tui::new()?;
    let mut app: App = App::new(ledger_db, chain_db, flamegraph_data, tui.get_frame().area())?;
    app.run(&mut tui).await?;
    Ok(())
}
