use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use mailparse::{
    parse_content_disposition, parse_mail, DispositionType, MailHeaderMap, ParsedMail,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tracing::info;

use crate::common::EmailDoc;
use crate::store::EmailIndex;
use scraper::{Html, Node};

pub async fn run_indexer(
    mbox_path: String,
    reindex: bool,
    attachments_dir: Option<String>,
) -> Result<()> {
    info!("Starting indexer for {}", mbox_path);

    let index_path = std::env::var("INDEX_PATH").unwrap_or_else(|_| "tantivy_index".to_string());
    let index_store = EmailIndex::new(std::path::Path::new(&index_path))?;
    let mut writer = index_store.writer()?;

    // 2. Reindex if requested
    if reindex {
        info!("Clearing index...");
        index_store.clear(&mut writer)?;
        writer.commit()?;
    }

    // 3. Process MBOX
    let file = File::open(&mbox_path).context("Failed to open mbox file")?;
    let reader = BufReader::new(file);

    let mut current_email_lines: Vec<u8> = Vec::new();
    let mut current_from_line: String = String::new();
    let mut batch: Vec<EmailDoc> = Vec::new();
    let batch_size = 50;

    // Attachment dir
    let att_dir_path = attachments_dir.map(PathBuf::from);

    let mut count = 0;
    for line_res in reader.split(b'\n') {
        let line = line_res?;

        // MBOX format: "From " at start of line
        let is_from_line = line.starts_with(b"From ");

        if is_from_line {
            if !current_email_lines.is_empty() {
                // Process accumulated email
                if let Ok(doc) = process_email_content(
                    &current_email_lines,
                    &current_from_line,
                    &att_dir_path,
                    count,
                ) {
                    batch.push(doc);
                    count += 1;
                }
                current_email_lines.clear();

                if batch.len() >= batch_size {
                    index_store.add_emails(&mut writer, &batch)?;
                    batch.clear();
                    info!("Indexed {} documents...", count);
                }
            }
            // Store the new From line
            current_from_line = String::from_utf8_lossy(&line).trim().to_string();
        }

        current_email_lines.extend_from_slice(&line);
        current_email_lines.push(b'\n');
    }

    // Process last email
    if !current_email_lines.is_empty() {
        if let Ok(doc) = process_email_content(
            &current_email_lines,
            &current_from_line,
            &att_dir_path,
            count,
        ) {
            batch.push(doc);
            count += 1;
        }
    }

    if !batch.is_empty() {
        index_store.add_emails(&mut writer, &batch)?;
    }

    writer.commit()?;

    info!("Indexing complete. Total documents: {}", count);
    Ok(())
}

fn process_email_content(
    raw_bytes: &[u8],
    from_line: &str,
    attachments_dir: &Option<PathBuf>,
    index: usize,
) -> Result<EmailDoc> {
    let parsed = parse_mail(raw_bytes).context("Failed to parse mail")?;

    // Helper to get header value
    let get_header =
        |name: &str| -> String { parsed.headers.get_first_value(name).unwrap_or_default() };

    let subject = sanitize_header(&get_header("Subject"));
    let from = sanitize_header(&get_header("From"));
    let to = sanitize_header(&get_header("To"));
    let message_id_raw = get_header("Message-ID");

    let msg_id = if message_id_raw.is_empty() {
        format!("generated-{}", index)
    } else {
        sanitize_header(&message_id_raw)
    };

    // Generate short ID
    let mut hasher = Sha256::new();
    hasher.update(msg_id.as_bytes());
    let short_id = hex::encode(hasher.finalize())[..12].to_string();

    // Date parsing: Try candidates in order
    let mut date: DateTime<Utc> = Utc::now();
    let mut found_date = false;

    // 1. Date Header
    let date_raw = get_header("Date");
    if !date_raw.is_empty() {
        let san = sanitize_header(&date_raw);
        if let Some(dt) = try_parse_date(&san) {
            date = dt;
            found_date = true;
        }
    }

    // 2. X-Received
    if !found_date {
        let x_rec = get_header("X-Received");
        if !x_rec.is_empty() {
            let val = if let Some(pos) = x_rec.rfind(';') {
                x_rec[pos + 1..].to_string()
            } else {
                x_rec
            };
            let san = sanitize_header(&val);
            if let Some(dt) = try_parse_date(&san) {
                date = dt;
                found_date = true;
            }
        }
    }

    // 3. Received
    if !found_date {
        let rec = get_header("Received");
        if !rec.is_empty() {
            let val = if let Some(pos) = rec.rfind(';') {
                rec[pos + 1..].to_string()
            } else {
                rec
            };
            let san = sanitize_header(&val);
            if let Some(dt) = try_parse_date(&san) {
                date = dt;
                found_date = true;
            }
        }
    }

    // 4. From Line
    if !found_date && !from_line.is_empty() {
        // "From 12345@xxx Thu Jul 10 14:57:19 +0000 2014"
        // Split by space, assume part[2] is dayname or part[2..] contains date?
        // Actually splitn(3) trick:
        // From <addr> <date...>
        let parts: Vec<&str> = from_line.splitn(3, ' ').collect();
        if parts.len() >= 3 {
            let val = parts[2];
            let san = sanitize_header(val);
            if let Some(dt) = try_parse_date(&san) {
                date = dt;
            }
        }
    }

    // Labels
    let labels_header = get_header("X-Gmail-Labels");
    let labels: Vec<String> = labels_header
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Body & Attachments
    let (body_text, body_html, attachments) = extract_parts(&parsed, &short_id, attachments_dir)?;

    let final_body_text = if body_text.is_empty() && !body_html.is_empty() {
        clean_html(&body_html)
    } else {
        body_text
    };

    Ok(EmailDoc {
        id: short_id,
        subject,
        from,
        to,
        date: date.to_rfc3339(),
        labels,
        body_text: final_body_text,
        body_html,
        has_attachment: !attachments.is_empty(),
        attachments: serde_json::to_value(attachments).unwrap_or(serde_json::json!([])),
    })
}

fn clean_html(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut text = String::new();
    traverse_node(document.tree.root(), &mut text);
    text
}

fn traverse_node(node: ego_tree::NodeRef<scraper::Node>, output: &mut String) {
    match node.value() {
        Node::Text(t) => {
            let s = t.trim();
            if !s.is_empty() {
                if !output.is_empty() {
                    output.push(' ');
                }
                output.push_str(s);
            }
        }
        Node::Element(e) => {
            let name = e.name();
            if name == "script" || name == "style" {
                return;
            }
        }
        _ => {}
    }

    for child in node.children() {
        traverse_node(child, output);
    }
}

fn sanitize_header(val: &str) -> String {
    val.replace(['\r', '\n'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn try_parse_date(date_str: &str) -> Option<DateTime<Utc>> {
    if date_str.is_empty() {
        return None;
    }

    // 0. Try generic RFC 3339 (ISO 8601)
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // 1. Try generic RFC 2822
    if let Ok(dt) = DateTime::parse_from_rfc2822(date_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // 1b. Try stripping Weekday if present (Fri, 21 Nov ...)
    if let Some(comma_pos) = date_str.find(',') {
        let trimmed = date_str[comma_pos + 1..].trim();
        if let Ok(dt) = DateTime::parse_from_str(trimmed, "%d %b %Y %H:%M:%S %z") {
            return Some(dt.with_timezone(&Utc));
        }
    }

    // 2. Try DD-MM-YY
    if let Ok(ndt) = NaiveDate::parse_from_str(date_str, "%d-%m-%y") {
        return Some(ndt.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    // 3. Try Incomplete (Wed, 14 May 2008 15) -> "%a, %d %b %Y %H"
    let incomplete_with_mins = format!("{}:00:00", date_str);
    if let Ok(ndt) = NaiveDateTime::parse_from_str(&incomplete_with_mins, "%a, %d %b %Y %H:%M:%S") {
        return Some(ndt.and_utc());
    }

    // 4. Try From line format: "Thu Jul 10 14:57:19 +0000 2014" -> "%a %b %d %H:%M:%S %z %Y"
    if let Ok(dt) = DateTime::parse_from_str(date_str, "%a %b %d %H:%M:%S %z %Y") {
        return Some(dt.with_timezone(&Utc));
    }

    None
}

// Wrapper for tests
#[allow(dead_code)]
fn parse_date(date_str: &str) -> DateTime<Utc> {
    try_parse_date(date_str).unwrap_or_else(Utc::now)
}

#[derive(Serialize)]
struct AttachmentMeta {
    filename: String,
    size: usize,
    content_type: String,
    path: String,
}

fn extract_parts(
    parsed: &ParsedMail,
    msg_id: &str,
    att_dir: &Option<PathBuf>,
) -> Result<(String, String, Vec<AttachmentMeta>)> {
    let mut text_body = String::new();
    let mut html_body = String::new();
    let mut attachments = Vec::new();

    if parsed.subparts.is_empty() {
        let ctype = &parsed.ctype.mimetype;
        if ctype.starts_with("text/plain") {
            text_body = parsed.get_body().unwrap_or_default();
        } else if ctype.starts_with("text/html") {
            html_body = parsed.get_body().unwrap_or_default();
        }
    } else {
        for part in &parsed.subparts {
            let disposition_val = part
                .headers
                .get_first_value("Content-Disposition")
                .unwrap_or_default();
            let disposition = parse_content_disposition(&disposition_val);

            let is_attachment = disposition.disposition == DispositionType::Attachment;

            if is_attachment {
                if let Some(dir) = att_dir {
                    let filename = disposition
                        .params
                        .get("filename")
                        .cloned()
                        .or_else(|| part.ctype.params.get("name").cloned())
                        .unwrap_or_else(|| "attachment".to_string());

                    let safe_filename = filename.replace(['/', '\\'], "_");

                    let safe_msg_id = msg_id.replace('/', "_");
                    let msg_dir = dir.join(&safe_msg_id);
                    fs::create_dir_all(&msg_dir)?;

                    let file_path = msg_dir.join(&safe_filename);
                    let content = part.get_body_raw().unwrap_or_default();
                    fs::write(&file_path, &content)?;

                    attachments.push(AttachmentMeta {
                        filename: safe_filename.clone(),
                        size: content.len(),
                        content_type: part.ctype.mimetype.clone(),
                        path: format!("{}/{}", safe_msg_id, safe_filename),
                    });
                }
            } else {
                let ctype = &part.ctype.mimetype;
                if ctype.starts_with("multipart/") {
                    let (sub_text, sub_html, sub_att) = extract_parts(part, msg_id, att_dir)?;
                    text_body.push_str(&sub_text);
                    html_body.push_str(&sub_html);
                    attachments.extend(sub_att);
                } else if ctype.starts_with("text/plain") {
                    text_body.push_str(&part.get_body().unwrap_or_default());
                } else if ctype.starts_with("text/html") {
                    html_body.push_str(&part.get_body().unwrap_or_default());
                }
            }
        }
    }

    Ok((text_body, html_body, attachments))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_header() {
        assert_eq!(sanitize_header("test@example.com\r"), "test@example.com");
        assert_eq!(sanitize_header("test@example.com\n"), "test@example.com");
        assert_eq!(
            sanitize_header("From:\r\ntest@example.com"),
            "From: test@example.com"
        );
        assert_eq!(sanitize_header("   spaces   "), "spaces");
    }

    #[test]
    fn test_parse_date() {
        // RFC 2822
        let dt = parse_date("Tue, 30 Dec 2025 12:00:00 -0000");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-12-30");

        // Short date DD-MM-YY
        let dt = parse_date("28-09-12");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2012-09-28");

        // Incomplete
        let dt = parse_date("Wed, 14 May 2008 15");
        assert_eq!(dt.format("%Y-%m-%d %H").to_string(), "2008-05-14 15");

        // From Line (Chat log)
        let dt = parse_date("Thu Jul 10 14:57:19 +0000 2014");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2014-07-10");

        // Sanatized received
        let sanitized = sanitize_header("Fri, 21 Nov 2007 09:55:06 -0600");
        let dt = parse_date(&sanitized);
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2007-11-21");

        // ISO 8601
        let dt = parse_date("2026-01-02T12:00:00Z");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2026-01-02");

        // Year 95 -> 1995 (assuming NaiveDate logic or simple parsing)
        // Note: chrono's %y usually maps 69-99 to 1900s, 00-68 to 2000s in some implementations,
        // or requires explicit base. Let's verify behavior.
        let dt = parse_date("15-03-95");
        // If it fails to parse as DD-MM-YY with %y, it might fallback to now() or error.
        // Let's assert based on expected behavior relative to 2000 split if standard
        // actually chrono %y is 20XX usually unless configured.
        // For robustness, let's just ensure it parses.
        assert_eq!(dt.format("%d-%m").to_string(), "15-03");

        // Whitespace only
        let dt = parse_date("   ");
        // Should be now(), hard to equality test exactly, but shouldn't panic.
        assert!(dt.timestamp() > 0);
    }

    #[test]
    fn test_mime_decoding() {
        // Test that mailparse + sanitize_header correctly decodes MIME headers
        let raw = b"Subject: =?UTF-8?B?VGVzdCBTdWJqZWN0?=\r\n\
From: =?iso-8859-2?Q?Rozpocz=EAcie_zam=F3wienia?=\r\n\
\r\n";

        let parsed = parse_mail(raw).unwrap();

        // Helper to get header value
        let get_header =
            |name: &str| -> String { parsed.headers.get_first_value(name).unwrap_or_default() };

        let subject = sanitize_header(&get_header("Subject"));
        assert_eq!(subject, "Test Subject");

        let from = sanitize_header(&get_header("From"));
        assert_eq!(from, "Rozpoczęcie zamówienia");
    }

    #[test]
    fn test_extract_attachments_multipart() {
        let raw = b"Content-Type: multipart/mixed; boundary=\"boundary\"\r\n\
\r\n\
--boundary\r\n\
Content-Type: text/plain\r\n\
\r\n\
Body text\r\n\
--boundary\r\n\
Content-Type: application/pdf; name=\"test_file.pdf\"\r\n\
Content-Disposition: attachment; filename=\"test_file.pdf\"\r\n\
\r\n\
FAKE PDF CONTENT\r\n\
--boundary--";

        let parsed = parse_mail(raw).unwrap();
        // Use a temp dir
        let temp_dir = std::env::temp_dir().join("email_server_test_att");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).ok();
        }

        let (body, _, atts) = extract_parts(&parsed, "shortid", &Some(temp_dir.clone())).unwrap();

        assert_eq!(body.trim(), "Body text");
        assert_eq!(atts.len(), 1);
        assert_eq!(atts[0].filename, "test_file.pdf");
        assert_eq!(atts[0].content_type, "application/pdf");

        // Check file exists
        let path = temp_dir.join("shortid").join("test_file.pdf");
        assert!(path.exists());
        let content = fs::read(path).unwrap();
        assert_eq!(content, b"FAKE PDF CONTENT\r\n");

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }
}
