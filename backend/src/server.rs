use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use std::net::SocketAddr;

use tantivy::collector::{Count, FacetCollector, TopDocs};
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::{Document, Facet, IndexRecordOption};
use tantivy::{DateTime, IndexReader, ReloadPolicy, TantivyDocument, Term};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::info;

use crate::store::EmailIndex;

#[derive(Clone)]
pub struct AppState {
    pub reader: IndexReader,
    pub index_store: EmailIndex,
}

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
    label: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    page: Option<usize>,
    size: Option<usize>,
}

pub async fn run_server(port: u16, attachments_dir: String) -> anyhow::Result<()> {
    let index_path = std::env::var("INDEX_PATH").unwrap_or_else(|_| "tantivy_index".to_string());
    std::fs::create_dir_all(&index_path)?;

    // Attachments dir passed from main

    std::fs::create_dir_all(&attachments_dir)?;

    let index_store = EmailIndex::new(std::path::Path::new(&index_path))?;
    let reader = index_store
        .index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;

    let state = AppState {
        reader,
        index_store,
    };

    let frontend_dir =
        std::env::var("FRONTEND_DIR").unwrap_or_else(|_| "../frontend/dist".to_string());
    let frontend_index = format!("{}/index.html", frontend_dir);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/search", get(search_emails_get))
        .route("/labels", get(get_labels))
        .route("/email/:id", get(get_email_detail))
        .route("/doc/:id", get(get_document_raw))
        .nest_service("/attachment", ServeDir::new(attachments_dir))
        .fallback_service(
            ServeDir::new(&frontend_dir)
                .fallback(tower_http::services::ServeFile::new(frontend_index)),
        )
        .layer(CorsLayer::permissive())
        .layer(axum::extract::DefaultBodyLimit::max(50_000_000))
        .with_state(state);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr_str = format!("{}:{}", host, port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid address format");
    info!("Email Server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

// Helper to extract string from Json Value (could be array or string)
fn extract_string(val: &serde_json::Value) -> String {
    if val.is_array() {
        val[0].as_str().unwrap_or("").to_string()
    } else {
        val.as_str().unwrap_or("").to_string()
    }
}

async fn search_emails_get(
    State(state): State<AppState>,
    AxumQuery(params): AxumQuery<SearchParams>,
) -> impl IntoResponse {
    let searcher = state.reader.searcher();
    let schema = &state.index_store.schema;

    let page = params.page.unwrap_or(1);
    let size = params.size.unwrap_or(20);
    let from = (page - 1) * size;

    let subject_field = schema.get_field("subject").unwrap();
    let from_field = schema.get_field("from").unwrap();
    let to_field = schema.get_field("to").unwrap();
    let body_text_field = schema.get_field("body_text").unwrap();
    let date_field = schema.get_field("date").unwrap();
    let labels_field = schema.get_field("labels").unwrap();

    let mut filter_queries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

    // Query (q)
    if let Some(q) = params.q {
        if !q.is_empty() {
            let query_parser = QueryParser::for_index(
                &state.index_store.index,
                vec![subject_field, from_field, body_text_field, to_field],
            );
            match query_parser.parse_query(&q) {
                Ok(query) => filter_queries.push((Occur::Must, query)),
                Err(_) => {} // Ignore invalid queries
            }
        }
    } else {
        if params.label.is_none() && params.start_date.is_none() && params.end_date.is_none() {
            filter_queries.push((Occur::Must, Box::new(AllQuery)));
        }
    }

    // Label
    if let Some(raw_label) = params.label {
        let label_lower = raw_label.to_lowercase();
        if label_lower != "all" {
            let actual_label = if label_lower == "inbox" {
                "Inbox"
            } else if label_lower == "sent" {
                "Sent"
            } else {
                &raw_label
            };

            let facet = Facet::from(&format!("/{}", actual_label));
            let label_query = TermQuery::new(
                Term::from_facet(labels_field, &facet),
                IndexRecordOption::Basic,
            );
            filter_queries.push((Occur::Must, Box::new(label_query)));
        }
    }

    // Date Range
    if params.start_date.is_some() || params.end_date.is_some() {
        let start = params
            .start_date
            .as_ref()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|| {
                        NaiveDate::parse_from_str(s, "%Y-%m-%d")
                            .ok()
                            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                    })
            })
            .map(|dt| DateTime::from_timestamp_secs(dt.timestamp()))
            .unwrap_or(DateTime::from_timestamp_secs(0));

        let end = params
            .end_date
            .as_ref()
            .and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|| {
                        NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| {
                            d.succ_opt()
                                .unwrap()
                                .and_hms_opt(0, 0, 0)
                                .unwrap()
                                .and_utc()
                        })
                    })
            })
            .map(|dt| DateTime::from_timestamp_secs(dt.timestamp()))
            .unwrap_or(DateTime::from_timestamp_secs(2147483647));

        let field_name = state
            .index_store
            .schema
            .get_field_name(date_field)
            .to_string();
        let range_query = RangeQuery::new_date(field_name, start..end);
        filter_queries.push((Occur::Must, Box::new(range_query)));
    }

    // If no filters are applied (e.g., label="ALL" acts as no label), default to AllQuery
    if filter_queries.is_empty() {
        filter_queries.push((Occur::Must, Box::new(AllQuery)));
    }

    let final_query = BooleanQuery::new(filter_queries);

    let collector = TopDocs::with_limit(size)
        .and_offset(from)
        .order_by_fast_field::<DateTime>("date", tantivy::Order::Desc);

    let (total_count, top_docs) = searcher.search(&final_query, &(Count, collector)).unwrap();

    let mut hits = Vec::new();
    for (_val, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.index_store.schema);
        let doc_obj: serde_json::Value = serde_json::from_str(&doc_json).unwrap();

        let labels: Vec<String> = doc_obj["labels"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim_start_matches('/').to_string()))
            .collect();

        let snippet = extract_string(&doc_obj["body_text"])
            .chars()
            .take(200)
            .collect::<String>();

        hits.push(serde_json::json!({
            "id": extract_string(&doc_obj["id"]),
            "subject": extract_string(&doc_obj["subject"]),
            "sender": extract_string(&doc_obj["from"]),
            "date": extract_string(&doc_obj["date"]),
            "snippet": snippet,
            "labels": labels,
            "has_attachment": if doc_obj["has_attachment"].is_array() { doc_obj["has_attachment"][0].as_bool().unwrap_or(false) } else { doc_obj["has_attachment"].as_bool().unwrap_or(false) }
        }));
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "total": total_count,
            "page": page,
            "size": size,
            "items": hits
        })),
    )
}

async fn get_labels(State(state): State<AppState>) -> impl IntoResponse {
    let searcher = state.reader.searcher();
    let labels_field = state.index_store.schema.get_field("labels").unwrap();
    let mut facet_collector =
        FacetCollector::for_field(state.index_store.schema.get_field_name(labels_field));
    facet_collector.add_facet("/");
    let facet_counts = searcher.search(&AllQuery, &facet_collector).unwrap();
    let mut labels = Vec::new();
    for (facet, _count) in facet_counts.get("/") {
        labels.push(facet.to_string().trim_start_matches('/').to_string());
    }
    (StatusCode::OK, Json(labels))
}

async fn get_email_detail(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> impl IntoResponse {
    let searcher = state.reader.searcher();
    let id_field = state.index_store.schema.get_field("id").unwrap();
    let term = Term::from_field_text(id_field, &doc_id);
    let query = TermQuery::new(term, IndexRecordOption::Basic);

    let top_docs = searcher.search(&query, &TopDocs::with_limit(1)).unwrap();

    if let Some((_score, doc_address)) = top_docs.first() {
        let retrieved_doc: TantivyDocument = searcher.doc(*doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.index_store.schema);
        let doc_obj: serde_json::Value = serde_json::from_str(&doc_json).unwrap();

        let labels: Vec<String> = doc_obj["labels"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim_start_matches('/').to_string()))
            .collect();

        let body_text = extract_string(&doc_obj["body_text"]);
        let body_html = extract_string(&doc_obj["body_html"]);
        let final_html = if !body_html.is_empty() {
            body_html
        } else {
            format!("<pre>{}</pre>", body_text)
        };

        let attachments = if doc_obj["attachments"].is_array() {
            doc_obj["attachments"][0].clone()
        } else {
            doc_obj["attachments"].clone()
        };

        let response = serde_json::json!({
            "id": doc_id,
            "subject": extract_string(&doc_obj["subject"]),
            "sender": extract_string(&doc_obj["from"]),
            "to": extract_string(&doc_obj["to"]),
            "date": extract_string(&doc_obj["date"]),
            "labels": labels,
            "body_html": final_html,
            "attachments": attachments,
        });

        (StatusCode::OK, Json(response)).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not found"})),
        )
            .into_response()
    }
}

async fn get_document_raw(
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> impl IntoResponse {
    get_email_detail(State(state), Path(doc_id)).await
}
