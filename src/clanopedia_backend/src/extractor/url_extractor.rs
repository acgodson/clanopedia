// src/extractor/url_extractor.rs

use chrono::DateTime;
use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
    TransformContext, TransformFunc,
};
use ic_cdk_macros::query;
use serde_json::Value;

use crate::external::blueband::ContentType;
use crate::extractor::types::{ExtractionProgress, ExtractionStatus, UrlType, YouTubeVideoInfo};
use crate::extractor::{sanitize_content, validate_content_size, Extractor};
use crate::{AddDocumentRequest, ClanopediaError, ClanopediaResult};

/// Structure to track YouTube playlist pagination state
#[derive(Debug, Clone)]
struct YouTubePaginationState {
    playlist_id: String,
    next_page_token: Option<String>,
    total_videos: Option<u32>,
    processed_videos: u32,
}

impl YouTubePaginationState {
    fn new(playlist_id: String) -> Self {
        Self {
            playlist_id,
            next_page_token: None,
            total_videos: None,
            processed_videos: 0,
        }
    }

    fn update_from_response(&mut self, response: &Value) {
        // Update total videos count
        if let Some(page_info) = response.get("pageInfo") {
            if let Some(total) = page_info.get("totalResults").and_then(|v| v.as_u64()) {
                self.total_videos = Some(total as u32);
            }
        }

        // Update next page token
        self.next_page_token = response
            .get("nextPageToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Update processed videos count
        if let Some(items) = response.get("items").and_then(|v| v.as_array()) {
            self.processed_videos += items.len() as u32;
        }
    }

    fn has_more_pages(&self) -> bool {
        self.next_page_token.is_some()
            && self
                .total_videos
                .is_none_or(|total| self.processed_videos < total)
    }
}

/// Maximum number of videos to extract in a single batch
const YOUTUBE_BATCH_SIZE: u32 = 50;

/// Extract content from URL (YouTube, GitHub, etc.)
pub async fn extract_url_content(
    url: String,
    collection_id: String,
    api_key: Option<String>,
) -> ClanopediaResult<Vec<AddDocumentRequest>> {
    let url_type = UrlType::from_url(&url);
    let documents = match url_type {
        UrlType::YouTube => {
            if let Some(api_key) = api_key {
                extract_youtube_content(&url, &collection_id, &api_key).await?
            } else {
                return Err(ClanopediaError::InvalidInput(
                    "YouTube API key is required".to_string(),
                ));
            }
        }
        UrlType::GitHub => extract_github_content(&url, &collection_id).await?,
        UrlType::Unknown => {
            return Err(ClanopediaError::InvalidInput(
                "Unsupported URL type".to_string(),
            ))
        }
    };

    // Update extraction progress to completed
    let progress = ExtractionProgress {
        url: url.clone(),
        collection_id: collection_id.clone(),
        playlist_id: String::new(),
        next_page_token: None,
        total_videos: None,
        processed_videos: documents.len() as u32,
        last_updated: ic_cdk::api::time(),
        status: ExtractionStatus::Completed,
    };
    Extractor::update_progress(progress);

    Ok(documents)
}

/// Extract YouTube content with pagination support and progress tracking
async fn extract_youtube_content(
    url: &str,
    collection_id: &str,
    api_key: &str,
) -> ClanopediaResult<Vec<AddDocumentRequest>> {
    let playlist_id = extract_youtube_playlist_id(url)?;

    // Check if there's existing progress for this URL/collection
    let mut pagination_state =
        if let Some(existing_progress) = Extractor::get_progress(collection_id, url) {
            ic_cdk::println!(
                "Resuming extraction from video {}",
                existing_progress.processed_videos
            );

            YouTubePaginationState {
                playlist_id: existing_progress.playlist_id,
                next_page_token: existing_progress.next_page_token,
                total_videos: existing_progress.total_videos,
                processed_videos: existing_progress.processed_videos,
            }
        } else {
            YouTubePaginationState::new(playlist_id.clone())
        };

    // Update progress to "InProgress"
    let progress = ExtractionProgress {
        url: url.to_string(),
        collection_id: collection_id.to_string(),
        playlist_id: playlist_id.clone(),
        next_page_token: pagination_state.next_page_token.clone(),
        total_videos: pagination_state.total_videos,
        processed_videos: pagination_state.processed_videos,
        last_updated: ic_cdk::api::time(),
        status: ExtractionStatus::InProgress,
    };
    Extractor::update_progress(progress);

    // Fetch videos (single batch for now - 50 videos max)
    let videos = match fetch_youtube_batch(&mut pagination_state, api_key).await {
        Ok(videos) => videos,
        Err(e) => {
            // Update progress to failed
            let failed_progress = ExtractionProgress {
                url: url.to_string(),
                collection_id: collection_id.to_string(),
                playlist_id,
                next_page_token: pagination_state.next_page_token.clone(),
                total_videos: pagination_state.total_videos,
                processed_videos: pagination_state.processed_videos,
                last_updated: ic_cdk::api::time(),
                status: ExtractionStatus::Failed(e.to_string()),
            };
            Extractor::update_progress(failed_progress);
            return Err(e);
        }
    };

    if videos.is_empty() {
        // Update progress to completed/failed
        let final_progress = ExtractionProgress {
            url: url.to_string(),
            collection_id: collection_id.to_string(),
            playlist_id,
            next_page_token: None,
            total_videos: pagination_state.total_videos,
            processed_videos: pagination_state.processed_videos,
            last_updated: ic_cdk::api::time(),
            status: ExtractionStatus::Failed("No videos found".to_string()),
        };
        Extractor::update_progress(final_progress);

        return Err(ClanopediaError::InvalidInput(
            "No videos found in YouTube playlist".to_string(),
        ));
    }

    // Transform videos to documents
    let mut documents = Vec::new();
    for video in videos {
        let document = youtube_video_to_document(video, collection_id)?;
        documents.push(document);
    }

    // Update final progress
    let has_more = pagination_state.has_more_pages();
    let final_status = if has_more {
        ExtractionStatus::Paused // More content available
    } else {
        ExtractionStatus::Completed
    };

    let final_progress = ExtractionProgress {
        url: url.to_string(),
        collection_id: collection_id.to_string(),
        playlist_id,
        next_page_token: pagination_state.next_page_token.clone(),
        total_videos: pagination_state.total_videos,
        processed_videos: pagination_state.processed_videos,
        last_updated: ic_cdk::api::time(),
        status: final_status,
    };
    Extractor::update_progress(final_progress);

    ic_cdk::println!(
        "Extraction batch completed: {} videos processed, Total: {}/{}, Has more: {}",
        documents.len(),
        pagination_state.processed_videos,
        pagination_state.total_videos.unwrap_or(0),
        has_more
    );

    Ok(documents)
}

/// Fetch a single batch of YouTube videos (up to YOUTUBE_BATCH_SIZE)
async fn fetch_youtube_batch(
    state: &mut YouTubePaginationState,
    api_key: &str,
) -> ClanopediaResult<Vec<YouTubeVideoInfo>> {
    let url = format!(
        "https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&playlistId={}&maxResults={}&key={}{}",
        state.playlist_id,
        YOUTUBE_BATCH_SIZE,
        api_key,
        state.next_page_token.as_ref()
            .map(|token| format!("&pageToken={}", token))
            .unwrap_or_default()
    );

    let cycles_needed = calculate_youtube_api_cycles();
    let request = CanisterHttpRequestArgument {
        url: url.clone(),
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: Some(1_000_000),
        transform: Some(TransformContext {
            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "transform_youtube_response".to_string(),
            }),
            context: vec![],
        }),
        headers: vec![HttpHeader {
            name: "User-Agent".to_string(),
            value: "IC-Clanopedia/1.0".to_string(),
        }],
    };

    match http_request(request, cycles_needed).await {
        Ok((response,)) => {
            let status = response.status.to_string().parse::<u32>().unwrap_or(0);
            if !(200..300).contains(&status) {
                return Err(ClanopediaError::ExternalCallError(format!(
                    "YouTube API error {}: {}",
                    response.status,
                    String::from_utf8_lossy(&response.body)
                )));
            }

            let json: Value = serde_json::from_slice(&response.body).map_err(|e| {
                ClanopediaError::ExternalCallError(format!("JSON parse error: {}", e))
            })?;

            // Check for API errors
            if let Some(error) = json.get("error") {
                return Err(ClanopediaError::ExternalCallError(format!(
                    "YouTube API error: {}",
                    error
                )));
            }

            // Update pagination state BEFORE parsing videos
            state.update_from_response(&json);

            // Parse and return videos
            parse_youtube_response(&response.body)
        }
        Err((rejection_code, message)) => {
            if message.contains("cycles") || message.contains("OutOfCycles") {
                Err(ClanopediaError::ExternalCallError(format!(
                    "Insufficient cycles: sent {} cycles but need more. Error: {}",
                    cycles_needed, message
                )))
            } else if message.contains("SysTransient") || message.contains("timeout") {
                Err(ClanopediaError::ExternalCallError(format!(
                    "Network error (consider retry): {:?} - {}",
                    rejection_code, message
                )))
            } else {
                Err(ClanopediaError::ExternalCallError(format!(
                    "HTTP request failed: {:?} - {}",
                    rejection_code, message
                )))
            }
        }
    }
}

/// Extract content from GitHub URL (for markdown files)
async fn extract_github_content(
    url: &str,
    collection_id: &str,
) -> ClanopediaResult<Vec<AddDocumentRequest>> {
    // Convert GitHub URL to raw content URL
    let raw_url = convert_github_url_to_raw(url)?;

    ic_cdk::println!("Fetching GitHub content from: {}", raw_url);

    // Fetch raw content
    let content = fetch_github_raw_content(&raw_url).await?;

    if content.trim().is_empty() {
        return Err(ClanopediaError::InvalidInput(
            "GitHub file is empty".to_string(),
        ));
    }

    // Validate content size
    validate_content_size(&content)?;

    // Extract filename from URL
    let filename = extract_filename_from_url(url).unwrap_or_else(|| "github_document".to_string());

    // Create document
    let document = AddDocumentRequest {
        collection_id: collection_id.to_string(),
        title: filename.clone(),
        content: sanitize_content(&content),
        content_type: Some(ContentType::Markdown),
        source_url: Some(url.to_string()),
        author: None,
        tags: Some(vec!["github".to_string()]),
    };

    ic_cdk::println!(
        "Successfully extracted GitHub content: {} characters",
        content.len()
    );

    Ok(vec![document])
}

/// Extract YouTube playlist ID from various URL formats
fn extract_youtube_playlist_id(url: &str) -> ClanopediaResult<String> {
    // Handle various YouTube URL formats
    if url.contains("list=") {
        if let Some(start) = url.find("list=") {
            let list_part = &url[start + 5..];
            let end = list_part.find('&').unwrap_or(list_part.len());
            return Ok(list_part[..end].to_string());
        }
    }

    // If it's a channel URL, we need to get the uploads playlist
    if url.contains("youtube.com/channel/")
        || url.contains("youtube.com/c/")
        || url.contains("youtube.com/@")
    {
        return Err(ClanopediaError::InvalidInput(
            "Please provide a YouTube playlist URL or we'll need to implement channel uploads extraction".to_string()
        ));
    }

    Err(ClanopediaError::InvalidInput(
        "Could not extract playlist ID from YouTube URL".to_string(),
    ))
}

/// Parse YouTube API response
fn parse_youtube_response(response_body: &[u8]) -> ClanopediaResult<Vec<YouTubeVideoInfo>> {
    let body_str = String::from_utf8(response_body.to_vec()).map_err(|e| {
        ClanopediaError::ExternalCallError(format!("Invalid UTF-8 response: {}", e))
    })?;

    let json: Value = serde_json::from_str(&body_str)
        .map_err(|e| ClanopediaError::ExternalCallError(format!("JSON parse error: {}", e)))?;

    // Check for API errors
    if let Some(error) = json.get("error") {
        return Err(ClanopediaError::ExternalCallError(format!(
            "YouTube API error: {}",
            error
        )));
    }

    let items = json["items"]
        .as_array()
        .ok_or_else(|| ClanopediaError::ExternalCallError("Missing items array".to_string()))?;

    let mut videos = Vec::new();

    for item in items {
        let snippet = &item["snippet"];

        let title = snippet["title"]
            .as_str()
            .unwrap_or("Untitled Video")
            .to_string();
        let description = snippet["description"].as_str().map(|s| s.to_string());
        let video_id = snippet["resourceId"]["videoId"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let published_at = snippet["publishedAt"]
            .as_str()
            .and_then(parse_rfc3339_to_timestamp)
            .unwrap_or(ic_cdk::api::time());

        let creator = snippet["channelTitle"].as_str().map(|s| s.to_string());

        if !video_id.is_empty() {
            videos.push(YouTubeVideoInfo {
                title,
                description,
                video_id,
                published_at,
                creator,
                tags: None, // Could extract from snippet.tags if available
            });
        }
    }

    Ok(videos)
}

/// Convert YouTube video info to AddDocumentRequest
fn youtube_video_to_document(
    video: YouTubeVideoInfo,
    collection_id: &str,
) -> ClanopediaResult<AddDocumentRequest> {
    // Use description as content, or create basic content from title
    let content = video.description.clone().unwrap_or_else(|| {
        format!(
            "YouTube Video: {}\n\nVideo ID: {}\nPublished: {}",
            video.title,
            video.video_id,
            format_timestamp(video.published_at)
        )
    });

    // Validate content size
    validate_content_size(&content)?;

    let source_url = format!("https://www.youtube.com/watch?v={}", video.video_id);

    Ok(AddDocumentRequest {
        collection_id: collection_id.to_string(),
        title: video.title,
        content: sanitize_content(&content),
        content_type: Some(ContentType::PlainText),
        source_url: Some(source_url),
        author: video.creator,
        tags: Some(vec!["youtube".to_string(), "video".to_string()]),
    })
}

/// Convert GitHub URL to raw content URL
fn convert_github_url_to_raw(url: &str) -> ClanopediaResult<String> {
    if url.contains("github.com") && url.contains("/blob/") {
        // Convert from: https://github.com/user/repo/blob/branch/file.md
        // To: https://raw.githubusercontent.com/user/repo/refs/heads/branch/file.md
        let raw_url = url
            .replace("github.com", "raw.githubusercontent.com")
            .replace("/blob/", "/refs/heads/");
        Ok(raw_url)
    } else if url.contains("raw.githubusercontent.com") {
        // Already a raw URL
        Ok(url.to_string())
    } else {
        Err(ClanopediaError::InvalidInput(
            "Invalid GitHub URL format. Expected github.com/user/repo/blob/branch/file.md"
                .to_string(),
        ))
    }
}

/// Fetch raw content from GitHub
async fn fetch_github_raw_content(url: &str) -> ClanopediaResult<String> {
    let cycles_needed = calculate_github_fetch_cycles();

    let request = CanisterHttpRequestArgument {
        url: url.to_string(),
        method: HttpMethod::GET,
        body: None,
        max_response_bytes: Some(2_000_000),
        transform: Some(TransformContext {
            function: TransformFunc(candid::Func {
                principal: ic_cdk::api::id(),
                method: "transform_github_response".to_string(),
            }),
            context: vec![],
        }),
        headers: vec![
            HttpHeader {
                name: "User-Agent".to_string(),
                value: "IC-Clanopedia/1.0".to_string(),
            },
            HttpHeader {
                name: "Accept".to_string(),
                value: "text/plain".to_string(),
            },
        ],
    };

    match http_request(request, cycles_needed).await {
        Ok((response,)) => {
            let status = response.status.to_string().parse::<u32>().unwrap_or(0);
            if !(200..300).contains(&status) {
                return Err(ClanopediaError::ExternalCallError(format!(
                    "GitHub fetch error {}: {}",
                    response.status,
                    String::from_utf8_lossy(&response.body)
                )));
            }

            String::from_utf8(response.body).map_err(|e| {
                ClanopediaError::ExternalCallError(format!("Invalid UTF-8 content: {}", e))
            })
        }
        Err((rejection_code, message)) => {
            if message.contains("cycles") || message.contains("OutOfCycles") {
                Err(ClanopediaError::ExternalCallError(format!(
                    "Insufficient cycles: sent {} cycles but need more. Error: {}",
                    cycles_needed, message
                )))
            } else if message.contains("SysTransient") || message.contains("timeout") {
                Err(ClanopediaError::ExternalCallError(format!(
                    "Network error (consider retry): {:?} - {}",
                    rejection_code, message
                )))
            } else {
                Err(ClanopediaError::ExternalCallError(format!(
                    "HTTP request failed: {:?} - {}",
                    rejection_code, message
                )))
            }
        }
    }
}

/// Extract filename from URL
fn extract_filename_from_url(url: &str) -> Option<String> {
    url.split('/').next_back().map(|s| s.to_string())
}

/// Calculate cycles needed for YouTube API call
fn calculate_youtube_api_cycles() -> u128 {
    let n = 13u128; // 13-node subnet
    let base_fee = (3_000_000 + 60_000 * n) * n;
    
    // Much more conservative estimates for YouTube API
    let request_size = 1000; // Increased estimate for URL + headers + query params
    let request_fee = 400 * n * request_size;
    
    // YouTube API responses can be large with video metadata
    let response_size = 500_000; // Increased to 500KB for playlist data
    let response_fee = 800 * n * response_size;

    let total_calculated = base_fee + request_fee + response_fee;
    
    // Use 4x buffer for YouTube API (more conservative)
    let with_buffer = (total_calculated as f64 * 4.0) as u128;
    
    // Ensure minimum of 10B cycles for YouTube API
    with_buffer.max(10_000_000_000)
}

/// Calculate cycles needed for GitHub fetch
fn calculate_github_fetch_cycles() -> u128 {
    let n = 13u128; // 13-node subnet
    let base_fee = (3_000_000 + 60_000 * n) * n;
    
    // Conservative estimates for GitHub
    let request_size = 500; // URL + headers
    let request_fee = 400 * n * request_size;
    
    // GitHub responses are typically smaller
    let response_size = 200_000; // 200KB for markdown files
    let response_fee = 800 * n * response_size;

    let total_calculated = base_fee + request_fee + response_fee;
    
    // Use 3x buffer for GitHub (less conservative than YouTube)
    let with_buffer = (total_calculated as f64 * 3.0) as u128;
    
    // Ensure minimum of 2B cycles for GitHub
    with_buffer.max(2_000_000_000)
}

/// Parse RFC3339 timestamp to nanoseconds
fn parse_rfc3339_to_timestamp(rfc3339: &str) -> Option<u64> {
    // Try parsing as RFC3339 first (most common format)
    if let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339) {
        return Some(dt.timestamp_nanos_opt().unwrap_or(0) as u64);
    }

    // Try parsing as ISO8601 with various formats
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d %H:%M:%S%.fZ",
        "%Y-%m-%d %H:%M:%SZ",
    ];

    for format in formats.iter() {
        if let Ok(dt) = DateTime::parse_from_str(rfc3339, format) {
            return Some(dt.timestamp_nanos_opt().unwrap_or(0) as u64);
        }
    }

    // If all parsing attempts fail, return current time
    Some(ic_cdk::api::time())
}

/// Format timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    timestamp.to_string()
}

/// Transform function for YouTube API responses
#[query]
fn transform_youtube_response(args: TransformArgs) -> HttpResponse {
    let mut response = args.response;

    // Remove non-deterministic headers
    response.headers.retain(|header| {
        let name_lower = header.name.to_lowercase();
        !name_lower.contains("date")
            && !name_lower.contains("server")
            && !name_lower.contains("x-request-id")
            && !name_lower.contains("x-ratelimit")
            && !name_lower.contains("cf-")
            && !name_lower.contains("set-cookie")
            && name_lower != "age"
            && name_lower != "vary"
    });

    response
}

/// Transform function for GitHub responses
#[query]
fn transform_github_response(args: TransformArgs) -> HttpResponse {
    let mut response = args.response;

    // Remove non-deterministic headers
    response.headers.retain(|header| {
        let name_lower = header.name.to_lowercase();
        !name_lower.contains("date")
            && !name_lower.contains("server")
            && !name_lower.contains("x-request-id")
            && !name_lower.contains("x-ratelimit")
            && !name_lower.contains("etag")
            && !name_lower.contains("last-modified")
            && !name_lower.contains("set-cookie")
            && name_lower != "age"
            && name_lower != "vary"
    });

    response
}
