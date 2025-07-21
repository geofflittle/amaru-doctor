use crate::otel::span::AmaruSpan;
use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tracing::error;

#[derive(Debug)]
pub struct SpanNode {
    pub span: AmaruSpan,
    // The Arc is for efficient cloning for the snapshot
    // The RwLock is for adding children to a parent, which will be an immutable ref (Arc)
    pub children: RwLock<HashMap<String, Arc<SpanNode>>>,
}

impl Clone for SpanNode {
    fn clone(&self) -> Self {
        let children_guard = self.children.read().unwrap();
        Self {
            span: self.span.clone(),
            children: RwLock::new(children_guard.clone()),
        }
    }
}

pub struct TraceRequestHandler {
    // Root id to its tree, this is the primary structure that we'll ultimately render
    pub root_to_tree: HashMap<String, Arc<SpanNode>>,
    // Span id to its ancestral root id, used for finding the tree to insert a span into
    span_to_root: HashMap<String, String>,
    // Parent id to orphans, used to hold spans for which we haven't yet seen the parent
    parent_to_orphans: HashMap<String, Vec<AmaruSpan>>,

    // Time-ordered index of root spans, used in trace eviction
    roots_by_start: BTreeMap<SystemTime, HashSet<String>>,
    // Trace id to child root ids, used to find all roots in trace eviction
    pub trace_to_roots: HashMap<String, HashSet<String>>,

    expire_duration: Duration,
}

impl TraceRequestHandler {
    pub fn new(expire_duration: Duration) -> Self {
        Self {
            root_to_tree: HashMap::new(),
            span_to_root: HashMap::new(),
            parent_to_orphans: HashMap::new(),
            roots_by_start: BTreeMap::new(),
            trace_to_roots: HashMap::new(),
            expire_duration,
        }
    }

    pub fn handle(&mut self, request: ExportTraceServiceRequest) {
        for r_spans in request.resource_spans {
            for s_spans in r_spans.scope_spans {
                for otel_span in s_spans.spans {
                    let span = AmaruSpan::from(otel_span);
                    self.handle_span(span);
                }
            }
        }
    }

    /// Recursively finds a mutable reference to a node within a tree, given its span id
    fn find_node(node: &Arc<SpanNode>, span_id: &str) -> Option<Arc<SpanNode>> {
        if node.span.span_id == span_id {
            return Some(node.clone());
        }

        let children_guard = node.children.read().unwrap();
        for child in children_guard.values() {
            if let Some(found) = Self::find_node(child, span_id) {
                return Some(found);
            }
        }

        None
    }

    pub fn handle_span(&mut self, span: AmaruSpan) {
        let span_id = span.span_id.clone();
        let trace_id = span.trace_id.clone();

        let mut was_inserted = false;
        if let Some(parent_id) = &span.parent_id_opt {
            // This is a child span, find its root
            if let Some(root_id) = self.span_to_root.get(parent_id).cloned() {
                // The parent's tree is known, find the parent node and attach this child
                if let Some(root_node) = self.root_to_tree.get_mut(&root_id) {
                    if let Some(parent_node) = Self::find_node(root_node, parent_id) {
                        // Attach the new child node.
                        let child_node = Arc::new(SpanNode {
                            span,
                            children: RwLock::new(HashMap::new()),
                        });
                        parent_node
                            .children
                            .write()
                            .unwrap()
                            .insert(span_id.clone(), child_node);
                        self.span_to_root.insert(span_id.clone(), root_id);
                        was_inserted = true;
                    } else {
                        error!(
                            "Found span's ({}) parent ({}) in span to root index, but couldn't find parent node in tree",
                            &span_id, &parent_id
                        )
                    }
                } else {
                    error!(
                        "Found span's ({}) parent's ({}) root {} in span to root index, but couldn't find root in root to tree index",
                        &span_id, &parent_id, &root_id
                    )
                }
            } else {
                // The parent hasn't been seen, add this span to orphans
                self.parent_to_orphans
                    .entry(parent_id.clone())
                    .or_default()
                    .push(span);
            }
        } else {
            // This is a root span, create a new tree
            self.roots_by_start
                .entry(span.start)
                .or_default()
                .insert(span_id.clone());
            self.span_to_root.insert(span_id.clone(), span_id.clone());
            self.trace_to_roots
                .entry(trace_id)
                .or_default()
                .insert(span_id.clone());
            self.root_to_tree.insert(
                span_id.clone(),
                Arc::new(SpanNode {
                    span,
                    children: RwLock::new(HashMap::new()),
                }),
            );
            was_inserted = true;
        }

        // If the span was successfully placed, it might be a parent to existing orphans that need to be resolved
        if was_inserted {
            self.resolve_orphans_for(&span_id);
        }
    }

    fn resolve_orphans_for(&mut self, parent_id: &str) {
        if let Some(orphans) = self.parent_to_orphans.remove(parent_id) {
            for orphan in orphans {
                // Now that we've seen the parent, handle the orphan as if it "newly arrived"
                self.handle_span(orphan);
            }
        }
    }

    pub fn evict_expired(&mut self) {
        let expire_before = match SystemTime::now().checked_sub(self.expire_duration) {
            Some(time) => time,
            None => return,
        };

        // Collect all expired root ids and their trace ids
        let expired_roots: Vec<String> = self
            .roots_by_start
            .range(..expire_before)
            .flat_map(|(_, v)| v.iter().cloned())
            .collect();
        let mut traces_to_evict = HashSet::new();
        for root_id in expired_roots {
            if let Some(root_node) = self.root_to_tree.get(&root_id) {
                traces_to_evict.insert(root_node.span.trace_id.clone());
            }
        }

        // For each trace id, evict all associated data
        for trace_id in traces_to_evict {
            if let Some(root_ids_in_trace) = self.trace_to_roots.remove(&trace_id) {
                for root_id in &root_ids_in_trace {
                    // Remove the tree and recursively clean up the span_to_root index
                    if let Some(removed_root_node) = self.root_to_tree.remove(root_id) {
                        // Handle the set of ids at a given timestamp
                        if let Some(ids_at_time) =
                            self.roots_by_start.get_mut(&removed_root_node.span.start)
                        {
                            ids_at_time.remove(root_id);
                            if ids_at_time.is_empty() {
                                self.roots_by_start.remove(&removed_root_node.span.start);
                            }
                        }

                        // Clean up the span_to_root index for the entire evicted tree
                        let mut stack = vec![removed_root_node];
                        while let Some(node) = stack.pop() {
                            self.span_to_root.remove(&node.span.span_id);
                            for child_node in node.children.read().unwrap().values() {
                                stack.push(child_node.clone());
                            }
                        }
                    }
                }
            }
        }

        // Evict expired orphans
        self.parent_to_orphans.retain(|_, orphans| {
            // Keep any orphans that aren't expired
            orphans.retain(|orphan| orphan.start >= expire_before);
            // Keep this map entry so long as there are remaining orphans
            !orphans.is_empty()
        });
    }
}
