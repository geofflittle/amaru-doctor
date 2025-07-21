use crate::view::flamegraph::{
    AmaruSpan, FlamegraphData, SpanNode, compute_trace_bounds_for_single_root,
};
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use std::time::{Duration, SystemTime};

lazy_static! {
    static ref ROOT_SPAN_TREE_1: SpanNode = {
        let base = SystemTime::now();

        SpanNode {
            span: AmaruSpan {
                trace_id: "trace1".into(),
                span_id: "root".into(),
                parent_id_opt: None,
                name: "MainApplicationFlow".into(),
                start: base,
                end: base + Duration::from_micros(10000), // Total 10ms duration
            },
            children: BTreeSet::from([
                // --- Level 1 Children ---

                // Child 1: Initial setup, relatively fast
                SpanNode {
                    span: AmaruSpan {
                        trace_id: "trace1".into(),
                        span_id: "child1_init".into(),
                        parent_id_opt: Some("root".into()),
                        name: "Initialization".into(),
                        start: base + Duration::from_micros(50),
                        end: base + Duration::from_micros(50) + Duration::from_micros(300), // 300us
                    },
                    children: BTreeSet::from([
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild1_load_config".into(),
                                parent_id_opt: Some("child1_init".into()),
                                name: "LoadConfig".into(),
                                start: base + Duration::from_micros(60),
                                end: base + Duration::from_micros(60) + Duration::from_micros(80), // 80us
                            },
                            children: BTreeSet::new(),
                        },
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild1_db_connect".into(),
                                parent_id_opt: Some("child1_init".into()),
                                name: "DBConnect".into(),
                                start: base + Duration::from_micros(150),
                                end: base + Duration::from_micros(150) + Duration::from_micros(150), // 150us
                            },
                            children: BTreeSet::new(),
                        },
                    ]),
                },

                // Child 2: Core processing, longer duration with nested calls
                SpanNode {
                    span: AmaruSpan {
                        trace_id: "trace1".into(),
                        span_id: "child2_process_data".into(),
                        parent_id_opt: Some("root".into()),
                        name: "ProcessData".into(),
                        start: base + Duration::from_micros(400),
                        end: base + Duration::from_micros(400) + Duration::from_micros(4000), // 4ms
                    },
                    children: BTreeSet::from([
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild2_fetch_api".into(),
                                parent_id_opt: Some("child2_process_data".into()),
                                name: "FetchAPI".into(),
                                start: base + Duration::from_micros(500),
                                end: base + Duration::from_micros(500) + Duration::from_micros(1500), // 1.5ms
                            },
                            children: BTreeSet::from([
                                SpanNode {
                                    span: AmaruSpan {
                                        trace_id: "trace1".into(),
                                        span_id: "great_grandchild_api_call".into(),
                                        parent_id_opt: Some("grandchild2_fetch_api".into()),
                                        name: "ExternalAPI_Call".into(),
                                        start: base + Duration::from_micros(600),
                                        end: base + Duration::from_micros(600) + Duration::from_micros(1200), // 1.2ms
                                    },
                                    children: BTreeSet::new(),
                                },
                            ]),
                        },
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild2_transform".into(),
                                parent_id_opt: Some("child2_process_data".into()),
                                name: "TransformData".into(),
                                start: base + Duration::from_micros(2000),
                                end: base + Duration::from_micros(2000) + Duration::from_micros(800), // 800us
                            },
                            children: BTreeSet::new(),
                        },
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild2_db_write".into(),
                                parent_id_opt: Some("child2_process_data".into()),
                                name: "DBWrite".into(),
                                start: base + Duration::from_micros(3000),
                                end: base + Duration::from_micros(3000) + Duration::from_micros(1200), // 1.2ms
                            },
                            children: BTreeSet::new(),
                        },
                    ]),
                },

                // Child 3: Concurrent background task, overlaps with ProcessData
                SpanNode {
                    span: AmaruSpan {
                        trace_id: "trace1".into(),
                        span_id: "child3_background_task".into(),
                        parent_id_opt: Some("root".into()),
                        name: "BackgroundTask".into(),
                        start: base + Duration::from_micros(1000),
                        end: base + Duration::from_micros(1000) + Duration::from_micros(2500), // 2.5ms
                    },
                    children: BTreeSet::from([
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild3_cache_refresh".into(),
                                parent_id_opt: Some("child3_background_task".into()),
                                name: "CacheRefresh".into(),
                                start: base + Duration::from_micros(1100),
                                end: base + Duration::from_micros(1100) + Duration::from_micros(500), // 500us
                            },
                            children: BTreeSet::new(),
                        },
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild3_log_process".into(),
                                parent_id_opt: Some("child3_background_task".into()),
                                name: "ProcessLogs".into(),
                                start: base + Duration::from_micros(1800),
                                end: base + Duration::from_micros(1800) + Duration::from_micros(600), // 600us
                            },
                            children: BTreeSet::new(),
                        },
                    ]),
                },

                // Child 4: Finalization, short
                SpanNode {
                    span: AmaruSpan {
                        trace_id: "trace1".into(),
                        span_id: "child4_finalize".into(),
                        parent_id_opt: Some("root".into()),
                        name: "Finalization".into(),
                        start: base + Duration::from_micros(5000),
                        end: base + Duration::from_micros(5000) + Duration::from_micros(100), // 100us
                    },
                    children: BTreeSet::new(),
                },

                // Child 5: Another independent task, very long, starts late
                SpanNode {
                    span: AmaruSpan {
                        trace_id: "trace1".into(),
                        span_id: "child5_long_task".into(),
                        parent_id_opt: Some("root".into()),
                        name: "LongRunningTask".into(),
                        start: base + Duration::from_micros(6000),
                        end: base + Duration::from_micros(6000) + Duration::from_micros(3500), // 3.5ms
                    },
                    children: BTreeSet::from([
                        SpanNode {
                            span: AmaruSpan {
                                trace_id: "trace1".into(),
                                span_id: "grandchild5_heavy_compute".into(),
                                parent_id_opt: Some("child5_long_task".into()),
                                name: "HeavyCompute".into(),
                                start: base + Duration::from_micros(6100),
                                end: base + Duration::from_micros(6100) + Duration::from_micros(3000), // 3ms
                            },
                            children: BTreeSet::new(),
                        },
                    ]),
                },
            ]),
        }
    };

    // Keep ROOT_SPAN_TREE_2 as is, or simplify if it's not strictly needed for testing
    static ref ROOT_SPAN_TREE_2: SpanNode = {
        let base = SystemTime::now();

        SpanNode {
            span: AmaruSpan {
                trace_id: "trace2".into(),
                span_id: "another_root".into(),
                parent_id_opt: None,
                name: "AnotherRootFlow".into(),
                start: base + Duration::from_micros(500),
                end: base + Duration::from_micros(500) + Duration::from_micros(1000),
            },
            children: BTreeSet::from([
                SpanNode {
                    span: AmaruSpan {
                        trace_id: "trace2".into(),
                        span_id: "sub_child_of_another_root".into(),
                        parent_id_opt: Some("another_root".into()),
                        name: "SubChildA".into(),
                        start: base + Duration::from_micros(600),
                        end: base + Duration::from_micros(600) + Duration::from_micros(300),
                    },
                    children: BTreeSet::new(),
                }
            ]),
        }
    };


    pub static ref EX_FLAMEGRAPH_ROOT1: FlamegraphData<'static> = {
        let (trace_start, trace_end) = compute_trace_bounds_for_single_root(&ROOT_SPAN_TREE_1);
        FlamegraphData {
            root_node: &ROOT_SPAN_TREE_1,
            trace_start,
            trace_end,
        }
    };

    pub static ref EX_FLAMEGRAPH_ROOT2: FlamegraphData<'static> = {
        let (trace_start, trace_end) = compute_trace_bounds_for_single_root(&ROOT_SPAN_TREE_2);
        FlamegraphData {
            root_node: &ROOT_SPAN_TREE_2,
            trace_start,
            trace_end,
        }
    };
}
