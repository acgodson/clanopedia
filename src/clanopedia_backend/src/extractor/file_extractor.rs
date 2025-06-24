// src/extractor/file_extractor.rs

use crate::external::blueband::ContentType;
use crate::extractor::types::{
    ExtractionMetadata, ExtractionResult, FileExtractionConfig, FileType,
};
use crate::extractor::{sanitize_content, validate_content_size};
use crate::types::{ClanopediaError, ClanopediaResult};
use crate::AddDocumentRequest;

// File parsing libraries
use chrono::DateTime;
use encoding_rs::{Encoding, UTF_8};
use lopdf::Document;
use quick_xml::{events::Event, Reader};
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Extract content from uploaded file buffer
pub fn extract_file_content(
    file_data: Vec<u8>,
    filename: String,
    collection_id: String,
) -> ClanopediaResult<Vec<AddDocumentRequest>> {
    let config = FileExtractionConfig::default();

    // Validate file size
    if file_data.len() as u64 > config.max_file_size {
        return Err(ClanopediaError::InvalidInput(format!(
            "File too large: {} bytes (max: {} bytes)",
            file_data.len(),
            config.max_file_size
        )));
    }

    let file_type = FileType::from_filename(&filename);

    // Check if file type is supported
    if !config.supported_types.contains(&file_type) {
        return Err(ClanopediaError::InvalidInput(format!(
            "Unsupported file type: {:?}",
            file_type
        )));
    }

    ic_cdk::println!(
        "Extracting content from file: {} (type: {:?}, size: {} bytes)",
        filename,
        file_type,
        file_data.len()
    );

    let extraction_result = match file_type {
        FileType::PlainText => extract_text_file(&file_data, &filename)?,
        FileType::Markdown => extract_markdown_file(&file_data, &filename)?,
        FileType::Pdf => extract_pdf_file(&file_data, &filename)?,
        FileType::DocX => extract_docx_file(&file_data, &filename)?,
        FileType::Unknown => {
            return Err(ClanopediaError::InvalidInput(
                "Cannot extract content from unknown file type".to_string(),
            ));
        }
    };

    // Validate extracted content size
    validate_content_size(&extraction_result.content)?;

    // Create AddDocumentRequest
    let document_request = AddDocumentRequest {
        collection_id,
        title: extraction_result.title,
        content: extraction_result.content,
        content_type: Some(extraction_result.content_type),
        source_url: extraction_result.source_url,
        author: extraction_result
            .metadata
            .as_ref()
            .and_then(|m| m.author.clone()),
        tags: extraction_result
            .metadata
            .as_ref()
            .and_then(|m| m.tags.clone()),
    };

    ic_cdk::println!(
        "Successfully extracted content: {} characters",
        document_request.content.len()
    );

    Ok(vec![document_request])
}

/// Extract content from PDF files using lopdf
fn extract_pdf_file(file_data: &[u8], filename: &str) -> ClanopediaResult<ExtractionResult> {
    // Load PDF document from memory using lopdf
    let doc = Document::load_mem(file_data)
        .map_err(|e| ClanopediaError::InvalidInput(format!("Invalid PDF file: {}", e)))?;

    // Extract text from all pages
    let mut text = String::new();
    let pages = doc.get_pages();

    for (page_num, _) in pages.iter() {
        match doc.extract_text(&[*page_num]) {
            Ok(page_text) => {
                if !page_text.trim().is_empty() {
                    text.push_str(&page_text);
                    text.push('\n');
                }
            }
            Err(_) => {
                // Skip pages that can't be extracted (images, etc.)
                continue;
            }
        }
    }

    if text.trim().is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "No extractable text found in PDF. This may be an image-based PDF, encrypted, or contains only graphics."
                .to_string(),
        ));
    }

    let title = get_filename_without_extension(filename);

    // Extract metadata using lopdf
    let metadata = extract_pdf_metadata_lopdf(&doc, file_data);

    Ok(ExtractionResult {
        title: metadata.title.unwrap_or(title),
        content: sanitize_content(&text),
        content_type: ContentType::PlainText,
        source_url: None,
        metadata: Some(ExtractionMetadata {
            file_size: Some(file_data.len() as u64),
            page_count: Some(pages.len() as u32),
            author: metadata.author,
            created_at: Some(ic_cdk::api::time()),
            tags: None,
        }),
    })
}

/// Extract content from DOCX files
fn extract_docx_file(file_data: &[u8], filename: &str) -> ClanopediaResult<ExtractionResult> {
    let cursor = Cursor::new(file_data);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| ClanopediaError::InvalidInput(format!("Invalid DOCX file: {}", e)))?;

    let document_xml = read_document_xml(&mut archive)?;
    let text = parse_docx_xml(&document_xml)?;

    if text.trim().is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "No text content found in DOCX".to_string(),
        ));
    }

    let title = get_filename_without_extension(filename);
    let metadata = extract_docx_metadata(&mut archive);

    Ok(ExtractionResult {
        title: metadata.title.unwrap_or(title),
        content: sanitize_content(&text),
        content_type: ContentType::PlainText,
        source_url: None,
        metadata: Some(ExtractionMetadata {
            file_size: Some(file_data.len() as u64),
            page_count: None,
            author: metadata.author,
            created_at: metadata.created_at,
            tags: metadata.tags,
        }),
    })
}

/// Extract content from plain text files
fn extract_text_file(file_data: &[u8], filename: &str) -> ClanopediaResult<ExtractionResult> {
    let (content, encoding_used, had_errors) = UTF_8.decode(file_data);

    if had_errors {
        let detected_encoding = detect_encoding(file_data);
        let (content_retry, _, _) = detected_encoding.decode(file_data);

        ic_cdk::println!(
            "Text encoding detection: {} -> {}",
            encoding_used.name(),
            detected_encoding.name()
        );

        return create_text_result(content_retry.into_owned(), filename);
    }

    create_text_result(content.into_owned(), filename)
}

/// Extract content from markdown files
fn extract_markdown_file(file_data: &[u8], filename: &str) -> ClanopediaResult<ExtractionResult> {
    let (content, _, _) = UTF_8.decode(file_data);
    let content = content.into_owned();

    let sanitized_content = sanitize_content(&content);

    if sanitized_content.trim().is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "Markdown file is empty".to_string(),
        ));
    }

    let markdown_metadata = parse_markdown_metadata(&content);
    let title = markdown_metadata
        .title
        .unwrap_or_else(|| get_filename_without_extension(filename));

    Ok(ExtractionResult {
        title,
        content: sanitized_content,
        content_type: ContentType::Markdown,
        source_url: None,
        metadata: Some(ExtractionMetadata {
            file_size: Some(file_data.len() as u64),
            page_count: None,
            author: markdown_metadata.author,
            created_at: Some(ic_cdk::api::time()),
            tags: markdown_metadata.tags,
        }),
    })
}

// ================================
// DOCX processing functions
// ================================

fn read_document_xml(archive: &mut ZipArchive<Cursor<&[u8]>>) -> ClanopediaResult<String> {
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|_| ClanopediaError::InvalidInput("No document.xml found in DOCX".to_string()))?;

    let mut document_xml = String::new();
    file.read_to_string(&mut document_xml).map_err(|e| {
        ClanopediaError::InvalidInput(format!("Failed to read document.xml: {}", e))
    })?;

    Ok(document_xml)
}

fn parse_docx_xml(xml_content: &str) -> ClanopediaResult<String> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut text_content = String::new();
    let mut buf = Vec::new();
    let mut in_text_element = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"w:t" => in_text_element = true,
                b"w:br" | b"w:cr" => text_content.push('\n'),
                b"w:p" => {
                    if !text_content.is_empty() && !text_content.ends_with('\n') {
                        text_content.push('\n');
                    }
                }
                b"w:tab" => text_content.push('\t'),
                _ => {}
            },
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"w:t" {
                    in_text_element = false;
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let text = e.unescape().map_err(|e| {
                        ClanopediaError::InvalidInput(format!("XML parsing error: {}", e))
                    })?;
                    text_content.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ClanopediaError::InvalidInput(format!(
                    "XML parsing error: {}",
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    if text_content.trim().is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "No text content found in Word document".to_string(),
        ));
    }

    let cleaned_text = text_content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(cleaned_text)
}

#[derive(Debug, Default)]
struct DocxMetadata {
    title: Option<String>,
    author: Option<String>,
    created_at: Option<u64>,
    tags: Option<Vec<String>>,
}

fn extract_docx_metadata(archive: &mut ZipArchive<Cursor<&[u8]>>) -> DocxMetadata {
    let mut metadata = DocxMetadata::default();

    // Extract from core.xml (Dublin Core metadata)
    if let Ok(mut file) = archive.by_name("docProps/core.xml") {
        let mut core_xml = String::new();
        if file.read_to_string(&mut core_xml).is_ok() {
            if let Some(core_metadata) = parse_core_metadata(&core_xml) {
                metadata.title = core_metadata.title;
                metadata.author = core_metadata.author;
                metadata.created_at = core_metadata.created_at;
            }
        }
    }

    // Extract from app.xml (application properties)
    if let Ok(mut file) = archive.by_name("docProps/app.xml") {
        let mut app_xml = String::new();
        if file.read_to_string(&mut app_xml).is_ok() {
            if let Some(keywords) = parse_keywords(&app_xml) {
                metadata.tags = Some(keywords);
            }
        }
    }

    metadata
}

fn parse_core_metadata(xml_content: &str) -> Option<DocxMetadata> {
    let mut reader = Reader::from_str(xml_content);
    let mut buf = Vec::new();
    let mut metadata = DocxMetadata::default();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    match current_element.as_str() {
                        "dc:title" => metadata.title = Some(text.to_string()),
                        "dc:creator" => metadata.author = Some(text.to_string()),
                        "dcterms:created" => {
                            if let Ok(timestamp) = parse_iso8601(&text) {
                                metadata.created_at = Some(timestamp);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Some(metadata)
}

fn parse_keywords(xml_content: &str) -> Option<Vec<String>> {
    let mut reader = Reader::from_str(xml_content);
    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                if current_element == "Keywords" {
                    if let Ok(text) = e.unescape() {
                        let keywords: Vec<String> = text
                            .split(&[',', ';'])
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if !keywords.is_empty() {
                            return Some(keywords);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    None
}

// ================================
// PDF metadata extraction using lopdf
// ================================

#[derive(Debug, Default)]
struct PdfMetadata {
    title: Option<String>,
    author: Option<String>,
    page_count: Option<u32>,
}

fn extract_pdf_metadata_lopdf(doc: &Document, _file_data: &[u8]) -> PdfMetadata {
    let mut metadata = PdfMetadata::default();

    // Extract metadata from PDF document info
    if let Ok(info_dict) = doc.trailer.get(b"Info") {
        if let Ok(reference) = info_dict.as_reference() {
            if let Ok(info_dict) = doc.get_dictionary(reference) {
                // Extract title
                if let Ok(title_obj) = info_dict.get(b"Title") {
                    if let Ok(title_bytes) = title_obj.as_str() {
                        if let Ok(title) = String::from_utf8(title_bytes.to_vec()) {
                            metadata.title = Some(title);
                        }
                    }
                }

                // Extract author
                if let Ok(author_obj) = info_dict.get(b"Author") {
                    if let Ok(author_bytes) = author_obj.as_str() {
                        if let Ok(author) = String::from_utf8(author_bytes.to_vec()) {
                            metadata.author = Some(author);
                        }
                    }
                }
            }
        }
    }

    // Page count is available from the pages collection
    let pages = doc.get_pages();
    metadata.page_count = Some(pages.len() as u32);

    metadata
}

// ================================
// Markdown processing functions
// ================================

#[derive(Debug, Default)]
struct MarkdownMetadata {
    title: Option<String>,
    author: Option<String>,
    tags: Option<Vec<String>>,
}

fn parse_markdown_metadata(content: &str) -> MarkdownMetadata {
    let mut metadata = MarkdownMetadata::default();

    if let Some(front_matter) = extract_yaml_frontmatter(content) {
        metadata = parse_yaml_frontmatter(&front_matter);
    }

    if metadata.title.is_none() {
        metadata.title = extract_title_from_content(content);
    }

    if metadata.tags.is_none() {
        metadata.tags = extract_tags_from_content(content);
    }

    metadata
}

fn extract_yaml_frontmatter(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();

    if lines.first()?.trim() != "---" {
        return None;
    }

    let mut end_index = None;
    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" || line.trim() == "..." {
            end_index = Some(i);
            break;
        }
    }

    let end_index = end_index?;
    Some(lines[1..end_index].join("\n"))
}

fn parse_yaml_frontmatter(yaml_content: &str) -> MarkdownMetadata {
    let mut metadata = MarkdownMetadata::default();

    for line in yaml_content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "title" => {
                    // Remove quotes if present
                    let value = value.trim_matches(|c| c == '"' || c == '\'');
                    if !value.is_empty() {
                        metadata.title = Some(value.to_string());
                    }
                }
                "author" => {
                    let value = value.trim_matches(|c| c == '"' || c == '\'');
                    if !value.is_empty() {
                        metadata.author = Some(value.to_string());
                    }
                }
                "tags" => {
                    // Handle both array and comma-separated formats
                    let tags: Vec<String> = if value.starts_with('[') && value.ends_with(']') {
                        // Array format: [tag1, tag2, tag3]
                        value[1..value.len() - 1]
                            .split(',')
                            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        // Comma-separated format: tag1, tag2, tag3
                        value
                            .split(',')
                            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .collect()
                    };
                    if !tags.is_empty() {
                        metadata.tags = Some(tags);
                    }
                }
                _ => {}
            }
        }
    }

    metadata
}

fn extract_title_from_content(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") && trimmed.len() > 2 {
            return Some(trimmed[2..].trim().to_string());
        }
    }
    None
}

fn extract_tags_from_content(content: &str) -> Option<Vec<String>> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("tags:") {
            let tag_part = trimmed.strip_prefix("tags:")?.trim();
            let tags: Vec<String> = tag_part
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return if tags.is_empty() { None } else { Some(tags) };
        }
    }
    None
}

// ================================
// Utility functions
// ================================

fn create_text_result(content: String, filename: &str) -> ClanopediaResult<ExtractionResult> {
    let sanitized_content = sanitize_content(&content);

    if sanitized_content.trim().is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "Text file is empty".to_string(),
        ));
    }

    let title = get_filename_without_extension(filename);

    Ok(ExtractionResult {
        title,
        content: sanitized_content,
        content_type: ContentType::PlainText,
        source_url: None,
        metadata: Some(ExtractionMetadata {
            file_size: Some(content.len() as u64),
            page_count: None,
            author: None,
            created_at: Some(ic_cdk::api::time()),
            tags: None,
        }),
    })
}

fn get_filename_without_extension(filename: &str) -> String {
    filename
        .rfind('.')
        .map(|i| &filename[..i])
        .unwrap_or(filename)
        .to_string()
}

fn detect_encoding(data: &[u8]) -> &'static Encoding {
    if data.starts_with(&[0xFF, 0xFE]) {
        encoding_rs::UTF_16LE
    } else if data.starts_with(&[0xFE, 0xFF]) {
        encoding_rs::UTF_16BE
    } else if data.starts_with(&[0xEF, 0xBB, 0xBF]) {
        encoding_rs::UTF_8
    } else if data.iter().any(|&b| b > 127) {
        encoding_rs::WINDOWS_1252
    } else {
        encoding_rs::UTF_8
    }
}

fn parse_iso8601(iso_string: &str) -> Result<u64, ()> {
    // Try parsing as RFC3339 first (most common format)
    if let Ok(dt) = DateTime::parse_from_rfc3339(iso_string) {
        return Ok(dt.timestamp_nanos_opt().unwrap_or(0) as u64);
    }

    // Try parsing as ISO8601 with various formats
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d %H:%M:%S%.fZ",
        "%Y-%m-%d %H:%M:%SZ",
    ];

    for format in formats.iter() {
        if let Ok(dt) = DateTime::parse_from_str(iso_string, format) {
            return Ok(dt.timestamp_nanos_opt().unwrap_or(0) as u64);
        }
    }

    // If all parsing attempts fail, return current time
    Ok(ic_cdk::api::time())
}
