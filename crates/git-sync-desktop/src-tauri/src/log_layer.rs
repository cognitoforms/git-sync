use chrono::Utc;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{Event, Level, Subscriber};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

#[derive(Clone, Serialize, specta::Type)]
pub struct FrontendLogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub repo: Option<String>,
}

/// Stored in a span's extensions when the span has a `repo` field.
struct RepoField(String);

struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

struct SpanRepoVisitor {
    repo: Option<String>,
}

impl Visit for SpanRepoVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "repo" {
            self.repo = Some(format!("{value:?}"));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "repo" {
            self.repo = Some(value.to_string());
        }
    }
}

pub struct TauriLogLayer {
    tx: UnboundedSender<FrontendLogEntry>,
}

impl TauriLogLayer {
    pub fn new(tx: UnboundedSender<FrontendLogEntry>) -> Self {
        Self { tx }
    }
}

impl<S> Layer<S> for TauriLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let Some(span) = ctx.span(id) else { return };
        let mut visitor = SpanRepoVisitor { repo: None };
        attrs.record(&mut visitor);
        if let Some(repo) = visitor.repo {
            span.extensions_mut().insert(RepoField(repo));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let level = event.metadata().level();
        // Only forward INFO and above (skip DEBUG / TRACE)
        if *level > Level::INFO {
            return;
        }

        let mut visitor = MessageVisitor { message: String::new() };
        event.record(&mut visitor);
        if visitor.message.is_empty() {
            return;
        }

        // Walk the span ancestry to find the nearest span that carries a repo field.
        let repo = ctx.lookup_current().and_then(|span| {
            span.scope()
                .find_map(|s| s.extensions().get::<RepoField>().map(|r| r.0.clone()))
        });

        let entry = FrontendLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string().to_lowercase(),
            message: visitor.message,
            repo,
        };

        let _ = self.tx.send(entry);
    }
}
