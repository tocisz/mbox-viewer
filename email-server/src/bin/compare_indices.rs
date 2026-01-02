use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;
use tantivy::{Document, Index, IndexReader, ReloadPolicy, TantivyDocument};

#[derive(Debug, PartialEq)]
struct DocData {
    subject: String,
    from: String,
    date: String,
    body_text_len: usize,
    body_text_sample: String,
}

fn extract_from_json(val: &serde_json::Value, field: &str) -> String {
    if let Some(v) = val.get(field) {
        if let Some(arr) = v.as_array() {
            if let Some(first) = arr.first() {
                return first.as_str().unwrap_or("").to_string();
            }
        }
        return v.as_str().unwrap_or("").to_string();
    }
    String::new()
}

fn load_index(path: &str) -> HashMap<String, DocData> {
    println!("Loading index from: {}", path);
    let index_path = Path::new(path);
    if !index_path.exists() {
        eprintln!("Index path not found: {}", path);
        return HashMap::new();
    }

    let index = Index::open_in_dir(index_path).expect("Failed to open index");
    let schema = index.schema();
    let reader: IndexReader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()
        .expect("Failed to create reader");

    let searcher = reader.searcher();

    let mut data = HashMap::new();
    let segment_readers = searcher.segment_readers();

    for segment in segment_readers {
        let store = segment.get_store_reader(100).unwrap();
        for doc_res in store.iter(segment.alive_bitset()) {
            let doc: TantivyDocument = doc_res.expect("Failed to read doc");

            let doc_json_str = doc.to_json(&schema);
            let doc_json: serde_json::Value = serde_json::from_str(&doc_json_str).unwrap();

            let id = extract_from_json(&doc_json, "id");
            let subject = extract_from_json(&doc_json, "subject");
            let from = extract_from_json(&doc_json, "from");
            let date = extract_from_json(&doc_json, "date");
            let body_text = extract_from_json(&doc_json, "body_text");

            let doc_data = DocData {
                subject,
                from,
                date,
                body_text_len: body_text.len(),
                body_text_sample: body_text.chars().take(50).collect(),
            };

            if !id.is_empty() {
                data.insert(id, doc_data);
            }
        }
    }

    println!("Loaded {} documents.", data.len());
    data
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path1 = if args.len() > 1 {
        &args[1]
    } else {
        "tantivy_index.1"
    };
    let path2 = if args.len() > 2 {
        &args[2]
    } else {
        "tantivy_index"
    };

    println!("Comparing Index 1 (Legacy/Python): {}", path1);
    println!("Comparing Index 2 (New/Rust):      {}", path2);
    println!("---------------------------------------------------");

    let idx1 = load_index(path1);
    let idx2 = load_index(path2);

    let keys1: HashSet<_> = idx1.keys().collect();
    let keys2: HashSet<_> = idx2.keys().collect();

    let only_in_1: Vec<_> = keys1.difference(&keys2).collect();
    let only_in_2: Vec<_> = keys2.difference(&keys1).collect();
    let common: Vec<_> = keys1.intersection(&keys2).collect();

    println!("\n--- ID Comparison ---");
    println!("Total in Index 1: {}", keys1.len());
    println!("Total in Index 2: {}", keys2.len());
    println!("Common IDs:       {}", common.len());

    if !only_in_1.is_empty() {
        println!("\nOnly in Index 1 ({}):", only_in_1.len());
        for k in only_in_1.iter().take(5) {
            println!("  - {}", k);
        }
        if only_in_1.len() > 5 {
            println!("  ...");
        }
    }

    if !only_in_2.is_empty() {
        println!("\nOnly in Index 2 ({}):", only_in_2.len());
        for k in only_in_2.iter().take(5) {
            println!("  - {}", k);
        }
        if only_in_2.len() > 5 {
            println!("  ...");
        }
    }

    println!("\n--- Field Discrepancies (Common IDs) ---");
    let mut diff_count = 0;

    for id in common {
        let d1 = idx1.get(*id).unwrap();
        let d2 = idx2.get(*id).unwrap();

        if d1.subject != d2.subject {
            println!("Mismatch [Subject] for ID {}", id);
            println!("  IDX1: {}", d1.subject);
            println!("  IDX2: {}", d2.subject);
            diff_count += 1;
        }

        if d1.from != d2.from {
            println!("Mismatch [From] for ID {}", id);
            println!("  IDX1: {}", d1.from);
            println!("  IDX2: {}", d2.from);
            diff_count += 1;
        }

        if d1.date != d2.date {
            println!("Mismatch [Date] for ID {}", id);
            println!("  IDX1: {}", d1.date);
            println!("  IDX2: {}", d2.date);
            diff_count += 1;
        }

        if d1.body_text_len != d2.body_text_len {
            println!("Mismatch [Body Len] for ID {}", id);
            println!(
                "  IDX1: {} (Sample: {})",
                d1.body_text_len, d1.body_text_sample
            );
            println!(
                "  IDX2: {} (Sample: {})",
                d2.body_text_len, d2.body_text_sample
            );
            diff_count += 1;
        }
    }

    if diff_count == 0 {
        println!("\nNo field discrepancies found in common documents!");
    } else {
        println!("\nFound {} discrepancies in common documents.", diff_count);
    }
}
