use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tantivy::schema::{FacetOptions, Schema, FAST, INDEXED, STORED, TEXT};
use tantivy::{doc, Index, IndexWriter};

use crate::common::EmailDoc;

#[derive(Clone)]
pub struct EmailIndex {
    pub index: Index,
    pub schema: Schema,
}

impl EmailIndex {
    pub fn new(path: &Path) -> Result<Self> {
        fs::create_dir_all(path)?;

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
            tantivy::directory::MmapDirectory::open(path)?,
            schema.clone(),
        )?;

        Ok(Self { index, schema })
    }

    pub fn writer(&self) -> Result<IndexWriter> {
        self.index
            .writer(50_000_000)
            .context("Failed to create index writer")
    }

    pub fn add_emails(&self, writer: &mut IndexWriter, emails: &[EmailDoc]) -> Result<()> {
        let id_field = self.schema.get_field("id").unwrap();
        let subject_field = self.schema.get_field("subject").unwrap();
        let from_field = self.schema.get_field("from").unwrap();
        let to_field = self.schema.get_field("to").unwrap();
        let date_field = self.schema.get_field("date").unwrap();
        let labels_field = self.schema.get_field("labels").unwrap();
        let body_text_field = self.schema.get_field("body_text").unwrap();
        let body_html_field = self.schema.get_field("body_html").unwrap();
        let has_attachment_field = self.schema.get_field("has_attachment").unwrap();
        let attachments_field = self.schema.get_field("attachments").unwrap();

        for doc_data in emails {
            // Date parsing logic reused or simplified?
            // For now, assume date string is ISO or close enough, or parse robustly like server did.
            // Ideally common.rs should have a robust parser, but let's replicate server logic here for safety
            // or better yet, move that logic to common later. keeping it inline for now.
            let date_parsed = chrono::DateTime::parse_from_rfc3339(&doc_data.date)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&doc_data.date, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                })
                .unwrap_or_else(|_| chrono::Utc::now());

            let date = tantivy::DateTime::from_timestamp_secs(date_parsed.timestamp());

            let mut tantivy_doc = doc!(
                id_field => doc_data.id.clone(),
                subject_field => doc_data.subject.clone(),
                from_field => doc_data.from.clone(),
                to_field => doc_data.to.clone(),
                date_field => date,
                body_text_field => doc_data.body_text.clone(),
                body_html_field => doc_data.body_html.clone(),
                has_attachment_field => doc_data.has_attachment,
                attachments_field => doc_data.attachments.clone(),
            );

            for label in &doc_data.labels {
                let facet = tantivy::schema::Facet::from(&format!("/{}", label));
                tantivy_doc.add_facet(labels_field, facet);
            }

            writer.add_document(tantivy_doc)?;
        }
        Ok(())
    }

    pub fn clear(&self, writer: &mut IndexWriter) -> Result<()> {
        writer.delete_all_documents()?;
        // commit is caller's responsibility usually, but if we want to enforce it immediately:
        // writer.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_write_index() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let store = EmailIndex::new(temp_dir.path())?;

        let mut writer = store.writer()?;

        let email = EmailDoc {
            id: "123".to_string(),
            subject: "Test Subject".to_string(),
            from: "me@example.com".to_string(),
            to: "you@example.com".to_string(),
            date: "2025-01-01T12:00:00Z".to_string(),
            labels: vec!["Inbox".to_string()],
            body_text: "Hello".to_string(),
            body_html: "<p>Hello</p>".to_string(),
            has_attachment: false,
            attachments: serde_json::json!([]),
        };

        store.add_emails(&mut writer, &[email])?;
        writer.commit()?;

        // Verify read
        let reader = store.index.reader()?;
        let searcher = reader.searcher();
        let query_parser = tantivy::query::QueryParser::for_index(
            &store.index,
            vec![store.schema.get_field("subject").unwrap()],
        );

        let query = query_parser.parse_query("Test")?;
        let (count, _docs) = searcher.search(
            &query,
            &(
                tantivy::collector::Count,
                tantivy::collector::TopDocs::with_limit(10),
            ),
        )?;

        assert_eq!(count, 1);

        Ok(())
    }
}
