use crate::otel::bounded_queue::BoundedQueue;
use opentelemetry_proto::tonic::trace::v1::Span;

pub struct BatchProcessor {
    buffer: Vec<Span>,
}

impl BatchProcessor {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn push_batch(&mut self, batch: Vec<Span>) {
        self.buffer.extend(batch);
    }

    pub fn drain_filtered_into<E>(&mut self, out: &mut BoundedQueue<E>)
    where
        E: From<Span>,
    {
        self.buffer.retain(|span| span.name != "low_priority_event");

        for span in self.buffer.drain(..) {
            out.push(span.into());
        }
    }

    pub fn is_full(&self, capacity: usize) -> bool {
        self.buffer.len() >= capacity
    }
}
