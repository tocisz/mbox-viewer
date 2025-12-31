use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
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
use tracing::{error, info};

#[derive(Clone)]
struct AppState {
    index: Index,
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    schema: Schema,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmailDoc {
    id: String,
    subject: String,
    from: String,
    to: String,
    date: String, // ISO 8601
    labels: Vec<String>,
    body_text: String,
    body_html: String,
    has_attachment: bool,
    attachments: serde_json::Value,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let index_path = "tantivy_index";
    std::fs::create_dir_all(index_path)?;

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STORED | TEXT | FAST);
    schema_builder.add_text_field("subject", TEXT | STORED);
    schema_builder.add_text_field("from", TEXT | STORED);
    schema_builder.add_text_field("to", TEXT | STORED);
    schema_builder.add_date_field("date", STORED | INDEXED | FAST);
    schema_builder.add_facet_field("labels", FacetOptions::default().set_stored());
    schema_builder.add_text_field("body_text", TEXT);
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
        .route("/search", post(search))
        .route("/labels", get(get_labels))
        .route("/doc/:id", get(get_document))
        .route("/create/:name", post(create_index))
        .layer(CorsLayer::permissive())
        .layer(axum::extract::DefaultBodyLimit::max(50_000_000))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8001));
    info!("Search service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

async fn create_index(Path(name): Path<String>) -> impl IntoResponse {
    info!("Creating index: {}", name);
    // For now, we just return OK as the index is pre-configured
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "created", "index": name})),
    )
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
            .or_else(|_| Utc.datetime_from_str(&doc_data.date, "%Y-%m-%d %H:%M:%S"))
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

#[derive(Serialize)]
struct SearchResponse {
    total: usize,
    hits: Vec<serde_json::Value>,
}

fn extract_string(val: &serde_json::Value) -> String {
    if val.is_array() {
        val[0].as_str().unwrap_or("").to_string()
    } else {
        val.as_str().unwrap_or("").to_string()
    }
}

async fn search(
    State(state): State<AppState>,
    Json(query_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let searcher = state.reader.searcher();
    let schema = &state.schema;

    let query_expr = query_body["query"]["bool"]["must"][0]["multi_match"]["query"]
        .as_str()
        .unwrap_or("");

    let from = query_body["from"].as_u64().unwrap_or(0) as usize;
    let size = query_body["size"].as_u64().unwrap_or(20) as usize;

    let label_filter = query_body["query"]["bool"]["filter"]
        .as_array()
        .and_then(|filters| {
            filters
                .iter()
                .find(|f| f["term"]["labels"].is_string())
                .and_then(|f| f["term"]["labels"].as_str())
        });

    let date_range_filter = query_body["query"]["bool"]["filter"]
        .as_array()
        .and_then(|filters| {
            filters
                .iter()
                .find(|f| f["range"]["date"].is_object())
                .map(|f| &f["range"]["date"])
        });

    let query_parser = QueryParser::for_index(
        &state.index,
        vec![
            schema.get_field("subject").unwrap(),
            schema.get_field("body_text").unwrap(),
            schema.get_field("from").unwrap(),
            schema.get_field("to").unwrap(),
        ],
    );

    let query = if query_expr.is_empty() {
        Box::new(AllQuery) as Box<dyn tantivy::query::Query>
    } else {
        match query_parser.parse_query(query_expr) {
            Ok(q) => q,
            Err(_) => Box::new(AllQuery),
        }
    };

    let mut filter_queries: Vec<(Occur, Box<dyn tantivy::query::Query>)> =
        vec![(Occur::Must, query)];

    if let Some(label) = label_filter {
        let label_field = schema.get_field("labels").unwrap();
        let facet = Facet::from(&format!("/{}", label));
        let label_query = TermQuery::new(
            Term::from_facet(label_field, &facet),
            IndexRecordOption::Basic,
        );
        filter_queries.push((Occur::Must, Box::new(label_query)));
    }

    if let Some(range) = date_range_filter {
        let date_field = schema.get_field("date").unwrap();
        let field_name = schema.get_field_name(date_field).to_string();

        let gte = range["gte"].as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|| Utc.datetime_from_str(s, "%Y-%m-%d").ok())
                .or_else(|| Utc.datetime_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
        });
        let lte = range["lte"].as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|| Utc.datetime_from_str(s, "%Y-%m-%d").ok())
                .or_else(|| Utc.datetime_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
        });

        if gte.is_some() || lte.is_some() {
            let start = gte
                .map(|dt| DateTime::from_timestamp_secs(dt.timestamp()))
                .unwrap_or(DateTime::from_timestamp_secs(0));
            let end = lte
                .map(|dt| DateTime::from_timestamp_secs(dt.timestamp()))
                .unwrap_or(DateTime::from_timestamp_secs(2147483647));

            let range_query = RangeQuery::new_date(field_name, start..end);
            filter_queries.push((Occur::Must, Box::new(range_query)));
        }
    }

    let final_query = BooleanQuery::new(filter_queries);

    let sort_field_name = query_body["sort"][0]
        .as_object()
        .and_then(|s| s.keys().next())
        .map(|s| s.as_str());
    let sort_order = query_body["sort"][0]
        .as_object()
        .and_then(|s| s.values().next())
        .and_then(|v| v["order"].as_str());

    let collector = TopDocs::with_limit(size).and_offset(from);

    let mut doc_addresses = Vec::new();
    let total_count;

    if let Some("date") = sort_field_name {
        let field_name = "date".to_string();
        if sort_order == Some("asc") {
            let (total, top_docs) = searcher
                .search(
                    &final_query,
                    &(
                        Count,
                        collector.order_by_fast_field::<DateTime>(field_name, tantivy::Order::Asc),
                    ),
                )
                .unwrap();
            total_count = total;
            for (_val, doc_address) in top_docs {
                doc_addresses.push(doc_address);
            }
        } else {
            let (total, top_docs) = searcher
                .search(
                    &final_query,
                    &(
                        Count,
                        collector.order_by_fast_field::<DateTime>(field_name, tantivy::Order::Desc),
                    ),
                )
                .unwrap();
            total_count = total;
            for (_val, doc_address) in top_docs {
                doc_addresses.push(doc_address);
            }
        }
    } else {
        let (total, top_docs) = searcher.search(&final_query, &(Count, collector)).unwrap();
        total_count = total;
        for (_score, doc_address) in top_docs {
            doc_addresses.push(doc_address);
        }
    }

    let mut hits = Vec::new();
    for doc_address in doc_addresses {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.schema);
        let doc_obj: serde_json::Value = serde_json::from_str(&doc_json).unwrap();

        let labels: Vec<String> = doc_obj["labels"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.trim_start_matches('/').to_string()))
            .collect();

        hits.push(serde_json::json!({
            "_id": extract_string(&doc_obj["id"]),
            "_source": {
                "subject": extract_string(&doc_obj["subject"]),
                "from": extract_string(&doc_obj["from"]),
                "date": extract_string(&doc_obj["date"]),
                "labels": labels, 
                "body_text": "",
                "has_attachment": if doc_obj["has_attachment"].is_array() { doc_obj["has_attachment"][0].as_bool().unwrap_or(false) } else { doc_obj["has_attachment"].as_bool().unwrap_or(false) },
            }
        }));
    }

    (
        StatusCode::OK,
        Json(SearchResponse {
            total: total_count,
            hits,
        }),
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

async fn get_document(
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

        let response = serde_json::json!({
            "_id": doc_id,
            "_source": {
                "subject": extract_string(&doc_obj["subject"]),
                "from": extract_string(&doc_obj["from"]),
                "to": extract_string(&doc_obj["to"]),
                "date": extract_string(&doc_obj["date"]),
                "labels": labels,
                "body_html": extract_string(&doc_obj["body_html"]),
                "attachments": if doc_obj["attachments"].is_array() { doc_obj["attachments"][0].clone() } else { doc_obj["attachments"].clone() },
            }
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
