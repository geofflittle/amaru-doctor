use crate::otel::FlamegraphData;
use crate::otel::trace_handler::SpanNode;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[derive(Copy, Clone)]
struct TraceBounds {
    start: SystemTime,
    end: SystemTime,
}

pub fn render_flamegraph(
    frame: &mut Frame,
    area: Rect,
    data: &FlamegraphData,
    is_focused: bool,
) -> Result<()> {
    let mut block = Block::default().title("Flame Graph").borders(Borders::ALL);
    if is_focused {
        block = block.border_style(Style::default().fg(Color::Blue));
    }

    let graph_width = area.width.saturating_sub(2) as f64;
    let mut lines = Vec::new();

    for (trace_id, root_ids) in &data.trace_to_roots {
        lines.push(Line::from(Span::styled(
            format!("--- TRACE: {} ---", trace_id),
            Style::default().fg(Color::DarkGray),
        )));

        for root_id in root_ids {
            if let Some(root_node) = data.root_to_tree.get(root_id) {
                if let Some(bounds) = compute_trace_bounds(root_node) {
                    let total_duration =
                        bounds.end.duration_since(bounds.start).unwrap_or_default();
                    if total_duration.is_zero() {
                        continue;
                    }

                    build_lines_for_node(root_node, &bounds, graph_width, &mut lines);
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);

    Ok(())
}

fn build_lines_for_node(
    node: &Arc<SpanNode>,
    bounds: &TraceBounds,
    graph_width: f64,
    lines: &mut Vec<Line>,
) {
    let total_duration_micros = bounds
        .end
        .duration_since(bounds.start)
        .unwrap_or_default()
        .as_micros();

    // Calculate the duration and offset of the current span relative to the whole trace.
    let span_start_offset = node
        .span
        .start
        .duration_since(bounds.start)
        .unwrap_or_default();
    let span_duration = node
        .span
        .end
        .duration_since(node.span.start)
        .unwrap_or_default();

    let scale = graph_width / total_duration_micros as f64;
    let offset_cells = (span_start_offset.as_micros() as f64 * scale).floor() as usize;
    let bar_width = (span_duration.as_micros() as f64 * scale).ceil().max(1.0) as usize;

    let start_time: DateTime<Utc> = DateTime::from(node.span.start);
    let full_label = format!(
        " {} [{}] ({})",
        node.span.name,
        start_time.format("%H:%M:%S%.3f"),
        format_duration(span_duration)
    );

    let bar_text = if full_label.len() > bar_width {
        let mut cut_off = bar_width.saturating_sub(1);
        while !full_label.is_char_boundary(cut_off) {
            cut_off -= 1;
        }
        format!("{}…", &full_label[..cut_off])
    } else {
        format!("{:<width$}", full_label, width = bar_width)
    };

    let bg_color = color_from_duration(
        span_duration,
        Duration::from_micros(total_duration_micros as u64),
    );
    let fg_color = if let Color::Rgb(r, g, b) = bg_color {
        if (r as u16 + g as u16 + b as u16) > 382 {
            Color::Black
        } else {
            Color::White
        }
    } else {
        Color::White
    };

    let bar = Span::styled(bar_text, Style::default().fg(fg_color).bg(bg_color));

    // The line is now composed of a calculated spacer and the bar
    lines.push(Line::from(vec![Span::raw(" ".repeat(offset_cells)), bar]));

    // Recurse for children
    let children_guard = node.children.read().unwrap();
    let mut sorted_children: Vec<_> = children_guard.values().collect();
    sorted_children.sort_by_key(|a| a.span.start);

    for child in sorted_children {
        build_lines_for_node(child, bounds, graph_width, lines);
    }
}

fn compute_trace_bounds(node: &Arc<SpanNode>) -> Option<TraceBounds> {
    let mut bounds = TraceBounds {
        start: node.span.start,
        end: node.span.end,
    };

    let children_guard = node.children.read().unwrap();
    for child in children_guard.values() {
        if let Some(child_bounds) = compute_trace_bounds(child) {
            if child_bounds.start < bounds.start {
                bounds.start = child_bounds.start;
            }
            if child_bounds.end > bounds.end {
                bounds.end = child_bounds.end;
            }
        }
    }
    Some(bounds)
}

/// Helper to format durations into units.
fn format_duration(duration: Duration) -> String {
    let micros = duration.as_micros();
    const MICROS_IN_SEC: u128 = 1_000_000;

    if micros >= MICROS_IN_SEC {
        // If one second or more, display with 3 decimal places.
        format!("{:.3} s", duration.as_secs_f64())
    } else {
        // Otherwise, display the raw microsecond count.
        format!("{} µs", micros)
    }
}

/// Helper to select a color from a "hot" to "cold" gradient
fn color_from_duration(duration: Duration, max_duration: Duration) -> Color {
    if max_duration.is_zero() {
        return Color::Rgb(0, 50, 100);
    }

    // Use logarithmic ratio for a better color distribution
    let log_max = (max_duration.as_micros() as f64).max(1.0).ln();
    let log_span = (duration.as_micros() as f64).max(1.0).ln();
    let ratio = if log_max > 0.0 {
        log_span / log_max
    } else {
        0.0
    };

    // Interpolate between a cool and hot color
    let cold = (80, 120, 180); // Blueish
    let hot = (210, 100, 80); // Reddish

    let r = (cold.0 as f64 * (1.0 - ratio) + hot.0 as f64 * ratio) as u8;
    let g = (cold.1 as f64 * (1.0 - ratio) + hot.1 as f64 * ratio) as u8;
    let b = (cold.2 as f64 * (1.0 - ratio) + hot.2 as f64 * ratio) as u8;

    Color::Rgb(r, g, b)
}
