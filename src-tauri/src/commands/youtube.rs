use crate::models::{SourceType, TranscriptionEntry, TranscriptionResult};
use crate::commands::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct YoutubeCaptionInfo {
    pub title: String,
    pub has_captions: bool,
    pub caption_languages: Vec<String>,
}

/// Build the Innertube client for YouTube API requests
fn build_innertube_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to build HTTP client")
}

/// Check if a YouTube video has captions available
#[tauri::command]
pub async fn check_youtube_captions(video_id: String) -> Result<YoutubeCaptionInfo, String> {
    let url = format!("https://www.youtube.com/watch?v={}", video_id);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let html = response.text().await.map_err(|e| e.to_string())?;

    // Extract video title
    let title = extract_title(&html).unwrap_or_else(|| format!("Video {}", video_id));

    // Check for captions
    let caption_languages = extract_caption_languages(&html);
    let has_captions = !caption_languages.is_empty();

    Ok(YoutubeCaptionInfo {
        title,
        has_captions,
        caption_languages,
    })
}

/// Get YouTube captions for a video using Innertube API
#[tauri::command]
pub async fn get_youtube_captions(
    state: State<'_, AppState>,
    video_id: String,
    language: String,
    include_timestamps: Option<bool>,
) -> Result<TranscriptionResult, String> {
    let with_timestamps = include_timestamps.unwrap_or(false);
    let start_time = std::time::Instant::now();
    let lang_code = if language == "auto" { "en" } else { &language };

    let client = build_innertube_client();

    // First, get the video page to extract initial player data
    let url = format!("https://www.youtube.com/watch?v={}", video_id);
    let response = client.get(&url)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let html = response.text().await.map_err(|e| e.to_string())?;

    let title = extract_title(&html).unwrap_or_else(|| format!("YouTube {}", video_id));

    // Try to get captions using the Innertube player API
    let text = match fetch_captions_innertube(&client, &video_id, lang_code, with_timestamps).await {
        Ok(t) => t,
        Err(_) => {
            // Fall back to legacy method
            fetch_captions_legacy(&client, &html, lang_code, with_timestamps).await?
        }
    };

    if text.is_empty() {
        return Err("Could not extract captions. Try using Whisper transcription instead.".to_string());
    }

    let processing_time = start_time.elapsed().as_secs_f64();

    let result = TranscriptionResult {
        text: text.clone(),
        language: Some(lang_code.to_string()),
        audio_info: None,
        processing_time,
    };

    // Save to history
    let entry = TranscriptionEntry::new(
        title,
        SourceType::YoutubeSubtitles,
        text,
        None,
        processing_time,
        Some(lang_code.to_string()),
    );

    state
        .history_manager
        .save_transcription(entry)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Fetch captions using YouTube Innertube API
async fn fetch_captions_innertube(client: &reqwest::Client, video_id: &str, lang: &str, include_timestamps: bool) -> Result<String, String> {
    // Innertube API endpoint
    let api_url = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";

    // Build the Innertube request payload (simulating Android client which has fewer restrictions)
    let payload = serde_json::json!({
        "context": {
            "client": {
                "hl": "en",
                "gl": "US",
                "clientName": "ANDROID",
                "clientVersion": "19.09.37",
                "androidSdkVersion": 30,
                "userAgent": "com.google.android.youtube/19.09.37 (Linux; U; Android 11) gzip"
            }
        },
        "videoId": video_id
    });

    let response = client.post(api_url)
        .header("Content-Type", "application/json")
        .header("X-YouTube-Client-Name", "3")
        .header("X-YouTube-Client-Version", "19.09.37")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Innertube request failed: {}", e))?;

    let json: serde_json::Value = response.json().await
        .map_err(|e| format!("Failed to parse Innertube response: {}", e))?;

    // Extract caption tracks from the response
    let caption_tracks = json
        .pointer("/captions/playerCaptionsTracklistRenderer/captionTracks")
        .and_then(|t| t.as_array())
        .ok_or_else(|| "No caption tracks found".to_string())?;

    // Find the best matching caption track
    let mut best_url: Option<String> = None;

    for track in caption_tracks {
        let track_lang = track.get("languageCode").and_then(|l| l.as_str()).unwrap_or("");
        let base_url = track.get("baseUrl").and_then(|u| u.as_str());

        if let Some(url) = base_url {
            if track_lang == lang {
                best_url = Some(url.to_string());
                break;
            } else if best_url.is_none() {
                best_url = Some(url.to_string());
            }
        }
    }

    let caption_url = best_url.ok_or_else(|| "No caption URL found".to_string())?;

    // Fetch the actual captions
    fetch_caption_content(client, &caption_url, include_timestamps).await
}

/// Fetch caption content from a URL
async fn fetch_caption_content(client: &reqwest::Client, base_url: &str, include_timestamps: bool) -> Result<String, String> {
    // Try JSON3 format first (best for timestamps)
    let json3_url = format!("{}&fmt=json3", base_url.replace("&fmt=srv3", "").replace("&fmt=json3", ""));

    let response = client.get(&json3_url)
        .send()
        .await
        .map_err(|e| format!("Caption fetch failed: {}", e))?;

    let content = response.text().await
        .map_err(|e| format!("Failed to read caption content: {}", e))?;

    if content.trim().starts_with('{') {
        if let Ok(text) = parse_json3_captions(&content, include_timestamps) {
            if !text.is_empty() {
                return Ok(text);
            }
        }
    }

    // Try srv3 format
    let srv3_url = format!("{}&fmt=srv3", base_url.replace("&fmt=srv3", "").replace("&fmt=json3", ""));
    let response = client.get(&srv3_url).send().await.map_err(|e| e.to_string())?;
    let content = response.text().await.map_err(|e| e.to_string())?;

    let text = parse_srv3_captions(&content, include_timestamps);
    if !text.is_empty() {
        return Ok(text);
    }

    // Try original format
    let response = client.get(base_url).send().await.map_err(|e| e.to_string())?;
    let content = response.text().await.map_err(|e| e.to_string())?;

    if content.contains("<text") {
        Ok(parse_xml_captions(&content, include_timestamps))
    } else {
        Ok(parse_srv3_captions(&content, include_timestamps))
    }
}

/// Legacy method to fetch captions from page HTML
async fn fetch_captions_legacy(client: &reqwest::Client, html: &str, lang: &str, include_timestamps: bool) -> Result<String, String> {
    let caption_url = extract_caption_url(html, lang)
        .ok_or_else(|| format!("No captions found for language '{}'", lang))?;

    fetch_caption_content(client, &caption_url, include_timestamps).await
}

/// Parse YouTube JSON3 caption format
fn parse_json3_captions(json_str: &str, include_timestamps: bool) -> Result<String, String> {
    // JSON3 format has structure: { "events": [ { "tStartMs": 1000, "segs": [ { "utf8": "text" } ] } ] }
    let json: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse caption JSON: {}", e))?;

    let mut result_parts: Vec<String> = Vec::new();
    let mut last_timestamp: Option<i64> = None;

    if let Some(events) = json.get("events").and_then(|e| e.as_array()) {
        for event in events {
            // Get timestamp for this event (in milliseconds)
            let t_start_ms = event.get("tStartMs").and_then(|t| t.as_i64());

            // Collect text from segments
            let mut event_text = String::new();
            if let Some(segs) = event.get("segs").and_then(|s| s.as_array()) {
                for seg in segs {
                    if let Some(utf8) = seg.get("utf8").and_then(|u| u.as_str()) {
                        let cleaned = utf8.trim();
                        if !cleaned.is_empty() && cleaned != "\n" {
                            if !event_text.is_empty() {
                                event_text.push(' ');
                            }
                            event_text.push_str(cleaned);
                        }
                    }
                }
            }

            if !event_text.is_empty() {
                if include_timestamps {
                    if let Some(ms) = t_start_ms {
                        // Only add timestamp if it's different from the last one (group by timestamp)
                        if last_timestamp != Some(ms) {
                            let timestamp = format_timestamp_ms(ms);
                            result_parts.push(format!("[{}] {}", timestamp, event_text));
                            last_timestamp = Some(ms);
                        } else {
                            // Same timestamp, append to last part
                            if let Some(last) = result_parts.last_mut() {
                                last.push(' ');
                                last.push_str(&event_text);
                            }
                        }
                    } else {
                        result_parts.push(event_text);
                    }
                } else {
                    result_parts.push(event_text);
                }
            }
        }
    }

    if result_parts.is_empty() {
        return Err("No text found in JSON3 captions".to_string());
    }

    // Join with appropriate separator
    let separator = if include_timestamps { "\n" } else { " " };
    let text = result_parts.join(separator);

    if include_timestamps {
        Ok(text)
    } else {
        Ok(normalize_text(&text))
    }
}

/// Format milliseconds to HH:MM:SS
fn format_timestamp_ms(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Parse YouTube SRV3 caption format
fn parse_srv3_captions(content: &str, include_timestamps: bool) -> String {
    let mut result_parts: Vec<String> = Vec::new();

    // SRV3 format uses <p t="ms" d="ms"> tags with text content
    let mut pos = 0;
    while let Some(start) = content[pos..].find("<p") {
        let abs_start = pos + start;

        if let Some(tag_end) = content[abs_start..].find(">") {
            let tag = &content[abs_start..abs_start + tag_end + 1];
            let content_start = abs_start + tag_end + 1;

            // Extract timestamp from t attribute
            let timestamp_ms = extract_attribute(tag, "t").and_then(|t| t.parse::<i64>().ok());

            // Find closing </p> tag
            if let Some(end) = content[content_start..].find("</p>") {
                let inner = &content[content_start..content_start + end];
                // Remove any inner tags like <s>
                let cleaned = strip_inner_tags(inner);
                let decoded = html_decode(&cleaned);
                let trimmed = decoded.trim();

                if !trimmed.is_empty() {
                    if include_timestamps {
                        if let Some(ms) = timestamp_ms {
                            let timestamp = format_timestamp_ms(ms);
                            result_parts.push(format!("[{}] {}", timestamp, trimmed));
                        } else {
                            result_parts.push(trimmed.to_string());
                        }
                    } else {
                        result_parts.push(trimmed.to_string());
                    }
                }

                pos = content_start + end + 4;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // If no <p> tags found, try <s> tags directly
    if result_parts.is_empty() {
        pos = 0;
        while let Some(start) = content[pos..].find("<s") {
            let abs_start = pos + start;

            if let Some(tag_end) = content[abs_start..].find(">") {
                let content_start = abs_start + tag_end + 1;

                if let Some(end) = content[content_start..].find("</s>") {
                    let inner = &content[content_start..content_start + end];
                    let decoded = html_decode(inner);
                    let trimmed = decoded.trim();

                    if !trimmed.is_empty() {
                        result_parts.push(trimmed.to_string());
                    }

                    pos = content_start + end + 4;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    if include_timestamps {
        result_parts.join("\n")
    } else {
        normalize_text(&result_parts.join(" "))
    }
}

/// Extract attribute value from an XML tag
fn extract_attribute(tag: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = tag.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = tag[value_start..].find("\"") {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    None
}

/// Strip inner XML tags from content
fn strip_inner_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }

    result
}

/// Parse YouTube XML caption format (legacy)
fn parse_xml_captions(xml: &str, include_timestamps: bool) -> String {
    let mut result_parts: Vec<String> = Vec::new();
    let mut pos = 0;

    while let Some(start) = xml[pos..].find("<text") {
        let abs_start = pos + start;

        if let Some(tag_end) = xml[abs_start..].find(">") {
            let tag = &xml[abs_start..abs_start + tag_end + 1];
            let content_start = abs_start + tag_end + 1;

            // Extract timestamp from start attribute (in seconds)
            let timestamp_secs = extract_attribute(tag, "start").and_then(|t| t.parse::<f64>().ok());

            if let Some(end) = xml[content_start..].find("</text>") {
                let content = &xml[content_start..content_start + end];
                let decoded = html_decode(content);
                let cleaned = decoded.trim();

                if !cleaned.is_empty() {
                    if include_timestamps {
                        if let Some(secs) = timestamp_secs {
                            let ms = (secs * 1000.0) as i64;
                            let timestamp = format_timestamp_ms(ms);
                            result_parts.push(format!("[{}] {}", timestamp, cleaned));
                        } else {
                            result_parts.push(cleaned.to_string());
                        }
                    } else {
                        result_parts.push(cleaned.to_string());
                    }
                }

                pos = content_start + end + 7;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if include_timestamps {
        result_parts.join("\n")
    } else {
        normalize_text(&result_parts.join(" "))
    }
}

/// Extract video title from YouTube page
fn extract_title(html: &str) -> Option<String> {
    // Try og:title first (most reliable)
    if let Some(start) = html.find("og:title\" content=\"") {
        let rest = &html[start + 19..];
        if let Some(end) = rest.find("\"") {
            let title = html_decode(&rest[..end]);
            if !title.is_empty() {
                return Some(title);
            }
        }
    }

    // Try <title> tag
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html[start..].find("</title>") {
            let title = &html[start + 7..start + end];
            let title = title.replace(" - YouTube", "").trim().to_string();
            if !title.is_empty() {
                return Some(html_decode(&title));
            }
        }
    }

    None
}

/// Extract available caption languages
fn extract_caption_languages(html: &str) -> Vec<String> {
    let mut languages = Vec::new();

    // Look for captionTracks in the player response
    if let Some(start) = html.find("\"captionTracks\":") {
        let section_end = std::cmp::min(start + 5000, html.len());
        let section = &html[start..section_end];

        // Extract language codes from the caption tracks
        let lang_codes = ["en", "es", "fr", "de", "pt", "ja", "zh", "ko", "ru", "it", "nl", "pl", "tr", "ar", "hi"];

        for lang in lang_codes {
            // Check for various patterns YouTube uses
            let has_lang = section.contains(&format!("\"languageCode\":\"{}\"", lang)) ||
                section.contains(&format!("\"vssId\":\".{}\"", lang)) ||
                section.contains(&format!("\"vssId\":\"a.{}\"", lang));

            if has_lang && !languages.contains(&lang.to_string()) {
                languages.push(lang.to_string());
            }
        }
    }

    // Also check for auto-generated captions marker
    if languages.is_empty() && html.contains("\"asr\"") && html.contains("captionTracks") {
        languages.push("en".to_string()); // Auto-generated usually in English
    }

    languages
}

/// Extract caption URL from YouTube page
fn extract_caption_url(html: &str, lang: &str) -> Option<String> {
    // Find the captionTracks section
    let caption_section = html.find("\"captionTracks\":")?;
    let section_start = caption_section;
    let section_end = std::cmp::min(section_start + 10000, html.len());
    let section = &html[section_start..section_end];

    // Look for baseUrl with the target language
    // Pattern: "baseUrl":"https://...","vssId":".en" or "a.en"

    // First try to find exact language match
    let lang_patterns = [
        format!("\"vssId\":\".{}\"", lang),
        format!("\"vssId\":\"a.{}\"", lang),
    ];

    for pattern in &lang_patterns {
        if let Some(lang_pos) = section.find(pattern) {
            // Search backwards for baseUrl
            let search_area = &section[..lang_pos];
            if let Some(base_url_pos) = search_area.rfind("\"baseUrl\":\"") {
                let url_start = base_url_pos + 11;
                if let Some(url_end) = section[url_start..].find("\"") {
                    let url = &section[url_start..url_start + url_end];
                    return Some(url.replace("\\u0026", "&"));
                }
            }
        }
    }

    // If specific language not found, try to get any caption URL
    if let Some(base_url_pos) = section.find("\"baseUrl\":\"") {
        let url_start = base_url_pos + 11;
        if let Some(url_end) = section[url_start..].find("\"") {
            let url = &section[url_start..url_start + url_end];
            if url.contains("timedtext") {
                return Some(url.replace("\\u0026", "&"));
            }
        }
    }

    None
}

/// Decode HTML entities
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("\\n", " ")
        .replace("\n", " ")
}

/// Normalize text (clean up whitespace, etc.)
fn normalize_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
