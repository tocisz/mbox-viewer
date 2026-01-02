use axum::{
    extract::{Path, Query as AxumQuery, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tantivy::collector::{Count, FacetCollector, TopDocs};
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, RangeQuery, TermQuery};
use tantivy::schema::{
    Facet, FacetOptions, IndexRecordOption, Schema, FAST, INDEXED, STORED, TEXT,
};
use tantivy::{
    doc, DateTime, Document, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info};

use crate::common::EmailDoc;

#[derive(Clone)]
pub struct AppState {
    pub index: Index,
    pub reader: IndexReader,
    pub writer: Arc<RwLock<IndexWriter>>,
    pub schema: Schema,
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
    let index_path = "tantivy_index";
    std::fs::create_dir_all(index_path)?;

    // Attachments dir passed from main
    std::fs::create_dir_all(&attachments_dir)?;

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STORED | TEXT | FAST);
    schema_builder.add_text_field("subject", TEXT | STORED);
    schema_builder.add_text_field("from", TEXT | STORED);
    schema_builder.add_text_field("to", TEXT | STORED);
    schema_builder.add_date_field("date", STORED | INDEXED | FAST);
    schema_builder.add_facet_field("labels", FacetOptions::default().set_stored());
    schema_builder.add_text_field("body_text", TEXT | STORED);
    schema_builder.add_text_field("body_html", STORED);
    schema_builder.add_bool_field("has_attachment", STORED | INDEXED);
    schema_builder.add_json_field("attachments", STORED);

    let schema = schema_builder.build();
    let index = Index::open_or_create(
        tantivy::directory::MmapDirectory::open(index_path)?,
        schema.clone(),
    )?;

    let writer = index.writer(50_000_000)?; // 50MB heap
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;

    let state = AppState {
        index,
        reader,
        writer: Arc::new(RwLock::new(writer)),
        schema,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/index", post(index_documents))
        .route("/search", get(search_emails_get))
        .route("/labels", get(get_labels))
        .route("/email/:id", get(get_email_detail))
        .route("/doc/:id", get(get_document_raw))
        .route("/create/:name", post(create_index_handler))
        .route("/delete/:name", delete(delete_index))
        .nest_service("/attachment", ServeDir::new(attachments_dir))
        .layer(CorsLayer::permissive())
        .layer(axum::extract::DefaultBodyLimit::max(50_000_000))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Email Server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

async fn create_index_handler(Path(name): Path<String>) -> impl IntoResponse {
    info!("Creating index: {}", name);
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "created", "index": name})),
    )
}

async fn delete_index(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    info!("Deleting/Clearing index: {}", name);
    let mut writer = state.writer.write().unwrap();
    writer.delete_all_documents().ok();
    match writer.commit() {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "deleted", "index": name})),
        ),
        Err(e) => {
            error!("Failed to clear index: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
}

async fn index_documents(
    State(state): State<AppState>,
    Json(docs): Json<Vec<EmailDoc>>,
) -> impl IntoResponse {
    info!("Received {} documents for indexing", docs.len());
    let mut writer = state.writer.write().unwrap();
    let schema = &state.schema;

    let id_field = schema.get_field("id").unwrap();
    let subject_field = schema.get_field("subject").unwrap();
    let from_field = schema.get_field("from").unwrap();
    let to_field = schema.get_field("to").unwrap();
    let date_field = schema.get_field("date").unwrap();
    let labels_field = schema.get_field("labels").unwrap();
    let body_text_field = schema.get_field("body_text").unwrap();
    let body_html_field = schema.get_field("body_html").unwrap();
    let has_attachment_field = schema.get_field("has_attachment").unwrap();
    let attachments_field = schema.get_field("attachments").unwrap();

    for doc_data in docs {
        let date_parsed = chrono::DateTime::parse_from_rfc3339(&doc_data.date)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                NaiveDateTime::parse_from_str(&doc_data.date, "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| ndt.and_utc())
            })
            .unwrap_or_else(|_| Utc::now());

        let date = DateTime::from_timestamp_secs(date_parsed.timestamp());

        let mut tantivy_doc = doc!(
            id_field => doc_data.id,
            subject_field => doc_data.subject,
            from_field => doc_data.from,
            to_field => doc_data.to,
            date_field => date,
            body_text_field => doc_data.body_text,
            body_html_field => doc_data.body_html,
            has_attachment_field => doc_data.has_attachment,
            attachments_field => doc_data.attachments,
        );

        for label in doc_data.labels {
            let facet = Facet::from(&format!("/{}", label));
            tantivy_doc.add_facet(labels_field, facet);
        }

        writer.add_document(tantivy_doc).ok();
    }

    match writer.commit() {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "indexed"})),
        ),
        Err(e) => {
            error!("Commit failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        }
    }
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
    let schema = &state.schema;

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
                &state.index,
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

        let field_name = state.schema.get_field_name(date_field).to_string();
        let range_query = RangeQuery::new_date(field_name, start..end);
        filter_queries.push((Occur::Must, Box::new(range_query)));
    }

    let final_query = BooleanQuery::new(filter_queries);

    let collector = TopDocs::with_limit(size)
        .and_offset(from)
        .order_by_fast_field::<DateTime>("date", tantivy::Order::Desc);

    let (total_count, top_docs) = searcher.search(&final_query, &(Count, collector)).unwrap();

    let mut hits = Vec::new();
    for (_val, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.schema);
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
    let labels_field = state.schema.get_field("labels").unwrap();
    let mut facet_collector = FacetCollector::for_field(state.schema.get_field_name(labels_field));
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
    let id_field = state.schema.get_field("id").unwrap();
    let term = Term::from_field_text(id_field, &doc_id);
    let query = TermQuery::new(term, IndexRecordOption::Basic);

    let top_docs = searcher.search(&query, &TopDocs::with_limit(1)).unwrap();

    if let Some((_score, doc_address)) = top_docs.first() {
        let retrieved_doc: TantivyDocument = searcher.doc(*doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.schema);
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
