use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tantivy::collector::{Count, FacetCollector, TopDocs};
use tantivy::query::{AllQuery, BooleanQuery, Occur, QueryParser, TermQuery};
use tantivy::schema::{
    Facet, FacetOptions, IndexRecordOption, Schema, TextOptions, FAST, INDEXED, STORED, TEXT,
};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term, Document};
use tower_http::cors::CorsLayer;
use tracing::{info, error};

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

#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
    label: Option<String>,
    from: Option<usize>,
    size: Option<usize>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let index_path = "tantivy_index";
    std::fs::create_dir_all(index_path)?;

    let mut schema_builder = Schema::builder();
    let id = schema_builder.add_text_field("id", STORED | TEXT | FAST);
    let subject = schema_builder.add_text_field("subject", TEXT | STORED);
    let from = schema_builder.add_text_field("from", TEXT | STORED);
    let to = schema_builder.add_text_field("to", TEXT | STORED);
    let date = schema_builder.add_text_field("date", STORED | TEXT | FAST);
    let labels = schema_builder.add_facet_field("labels", FacetOptions::default());
    let body_text = schema_builder.add_text_field("body_text", TEXT);
    let body_html = schema_builder.add_text_field("body_html", STORED);
    let has_attachment = schema_builder.add_bool_field("has_attachment", STORED | INDEXED);
    let attachments = schema_builder.add_json_field("attachments", STORED);

    let schema = schema_builder.build();
    let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(index_path)?, schema.clone())?;

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
        let mut tantivy_doc = doc!(
            id_field => doc_data.id,
            subject_field => doc_data.subject,
            from_field => doc_data.from,
            to_field => doc_data.to,
            date_field => doc_data.date,
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
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "indexed"}))),
        Err(e) => {
            error!("Commit failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()})))
        }
    }
}

#[derive(Serialize)]
struct SearchResponse {
    total: usize,
    hits: Vec<serde_json::Value>,
}

async fn search(
    State(state): State<AppState>,
    Json(query_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Simplified ES query body parsing
    // { "query": { "bool": { "must": [...], "filter": [...] } }, "from": 0, "size": 20, "sort": [...] }
    
    let searcher = state.reader.searcher();
    let schema = &state.schema;
    
    let query_expr = query_body["query"]["bool"]["must"][0]["multi_match"]["query"]
        .as_str()
        .unwrap_or("");
        
    let label_filter = query_body["query"]["bool"]["filter"]
        .as_array()
        .and_then(|filters| {
            filters.iter().find(|f| f["term"]["labels"].is_string())
                .and_then(|f| f["term"]["labels"].as_str())
        });

    let from = query_body["from"].as_u64().unwrap_or(0) as usize;
    let size = query_body["size"].as_u64().unwrap_or(20) as usize;

    let mut query_parser = QueryParser::for_index(&state.index, vec![
        schema.get_field("subject").unwrap(),
        schema.get_field("body_text").unwrap(),
        schema.get_field("from").unwrap(),
        schema.get_field("to").unwrap(),
    ]);

    let query = if query_expr.is_empty() {
        Box::new(AllQuery) as Box<dyn tantivy::query::Query>
    } else {
        match query_parser.parse_query(query_expr) {
            Ok(q) => q,
            Err(_) => Box::new(AllQuery),
        }
    };

    // Apply label filter if present
    let final_query: Box<dyn tantivy::query::Query> = if let Some(label) = label_filter {
        let label_field = schema.get_field("labels").unwrap();
        let facet = Facet::from(&format!("/{}", label));
        let label_query = TermQuery::new(
            Term::from_facet(label_field, &facet),
            IndexRecordOption::Basic,
        );
        Box::new(BooleanQuery::new(vec![
            (Occur::Must, query),
            (Occur::Must, Box::new(label_query)),
        ]))
    } else {
        query
    };

    let (total_count, top_docs) = searcher.search(&final_query, &(Count, TopDocs::with_limit(size).and_offset(from))).unwrap();

    let mut hits = Vec::new();
    for (_score, doc_address) in top_docs {
        let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.schema);
        let mut doc_obj: serde_json::Value = serde_json::from_str(&doc_json).unwrap();
        
        // Tantivy 0.22 JSON format is different, it might return arrays for fields
        hits.append(&mut vec![serde_json::json!({
            "_id": doc_obj["id"][0],
            "_source": {
                "subject": doc_obj["subject"][0],
                "from": doc_obj["from"][0],
                "date": doc_obj["date"][0],
                "labels": doc_obj["labels"], 
                "body_text": doc_obj["body_text"][0],
                "has_attachment": doc_obj["has_attachment"][0],
            }
        })]);
    }

    (StatusCode::OK, Json(SearchResponse { total: total_count, hits }))
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
        let retrieved_doc: tantivy::TantivyDocument = searcher.doc(*doc_address).unwrap();
        let doc_json = retrieved_doc.to_json(&state.schema);
        let doc_obj: serde_json::Value = serde_json::from_str(&doc_json).unwrap();
        
        // Cleanup response to match ES format
        let response = serde_json::json!({
            "_id": doc_id,
            "_source": {
                "subject": doc_obj["subject"][0],
                "from": doc_obj["from"][0],
                "to": doc_obj["to"][0],
                "date": doc_obj["date"][0],
                "labels": doc_obj["labels"],
                "body_html": doc_obj["body_html"][0],
                "attachments": doc_obj["attachments"][0],
            }
        });
        
        (StatusCode::OK, Json(response)).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response()
    }
}
