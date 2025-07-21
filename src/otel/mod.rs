use crate::otel::trace_handler::{SpanNode, TraceRequestHandler};
use arc_swap::ArcSwap;
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse, trace_service_server::TraceService,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendError;
use tonic::{Request, Response, Status};

pub mod span;
pub mod trace_handler;

#[derive(Clone, Default, Debug)]
pub struct FlamegraphData {
    pub root_to_tree: HashMap<String, Arc<SpanNode>>,
    pub trace_to_roots: HashMap<String, HashSet<String>>,
}

pub struct TraceCollector {
    batch_tx: mpsc::Sender<ExportTraceServiceRequest>,
    flamegraph_snapshot: Arc<ArcSwap<FlamegraphData>>,
}

impl TraceCollector {
    pub fn new(queue_cap: usize, expire_duration: Duration) -> Arc<Self> {
        let (tx, rx) = mpsc::channel(queue_cap);
        let render_data = Arc::new(ArcSwap::from_pointee(FlamegraphData::default()));
        let render_data_clone = render_data.clone();
        tokio::spawn(run_worker(rx, render_data_clone, expire_duration));

        Arc::new(Self {
            batch_tx: tx,
            flamegraph_snapshot: render_data,
        })
    }

    pub async fn handle(
        &self,
        request: ExportTraceServiceRequest,
    ) -> Result<(), SendError<ExportTraceServiceRequest>> {
        self.batch_tx.send(request).await
    }

    pub fn flamegraph_snapshot(&self) -> Arc<ArcSwap<FlamegraphData>> {
        self.flamegraph_snapshot.clone()
    }
}

async fn run_worker(
    mut batch_rx: mpsc::Receiver<ExportTraceServiceRequest>,
    shared_flamegraph_snapshot: Arc<ArcSwap<FlamegraphData>>,
    expire_duration: Duration,
) {
    let mut handler = TraceRequestHandler::new(expire_duration);

    // Block for a single request
    while let Some(req) = batch_rx.recv().await {
        handler.handle(req);
        // If there are more available, drain the queue of requests
        while let Ok(another_req) = batch_rx.try_recv() {
            handler.handle(another_req);
        }

        handler.evict_expired();

        let new_snapshot = FlamegraphData {
            root_to_tree: handler.root_to_tree.clone(),
            trace_to_roots: handler.trace_to_roots.clone(),
        };

        shared_flamegraph_snapshot.store(Arc::new(new_snapshot));
    }
}

pub struct AmaruTraceService {
    collector: Arc<TraceCollector>,
}

impl AmaruTraceService {
    pub fn new(collector: Arc<TraceCollector>) -> Self {
        Self { collector }
    }
}

#[tonic::async_trait]
impl TraceService for AmaruTraceService {
    async fn export(
        &self,
        req: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        let trace_req = req.into_inner();
        self.collector
            .handle(trace_req)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))
            .map(|_| Response::new(ExportTraceServiceResponse::default()))
    }
}
