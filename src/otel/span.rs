use opentelemetry_proto::tonic::trace::v1::Span;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AmaruSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_id_opt: Option<String>,
    pub name: String,
    pub start: SystemTime,
    pub end: SystemTime,
}

impl From<Span> for AmaruSpan {
    fn from(span: Span) -> AmaruSpan {
        AmaruSpan {
            trace_id: hex::encode(span.trace_id),
            span_id: hex::encode(span.span_id),
            parent_id_opt: (!span.parent_span_id.is_empty())
                .then(|| hex::encode(span.parent_span_id)),
            name: span.name,
            start: UNIX_EPOCH + Duration::from_nanos(span.start_time_unix_nano),
            end: UNIX_EPOCH + Duration::from_nanos(span.end_time_unix_nano),
        }
    }
}
