use std::collections::HashMap;
use std::time::Duration;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::warn;

use crate::core::error::AppError;
use crate::features::translator::subtitle::{SubtitleCue, SubtitleFormat};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Clone)]
pub struct GeminiClient {
    http: Client,
}

#[derive(Debug, Serialize)]
struct InputCueItem<'a> {
    id: usize,
    text: &'a str,
}

#[derive(Debug, Deserialize)]
struct GeminiTranslateResponse {
    translations: Vec<TranslatedCueItem>,
}

#[derive(Debug, Deserialize)]
struct TranslatedCueItem {
    id: usize,
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiGenerateResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "promptFeedback")]
    prompt_feedback: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorEnvelope {
    error: Option<GeminiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorBody {
    message: Option<String>,
    status: Option<String>,
}

impl GeminiClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { http }
    }

    /// Validates an API key and model against Google AI Studio.
    pub async fn test_api_key(&self, api_key: &str, model: &str) -> Result<String, AppError> {
        let key = api_key.trim();
        if key.is_empty() {
            return Err(AppError::BadRequest("Google AI Studio API key cannot be empty".into()));
        }

        let clean_model = model.trim().trim_start_matches("models/");
        let model_name = if clean_model.is_empty() {
            "gemini-2.5-flash"
        } else {
            clean_model
        };

        let url = format!("{GEMINI_API_BASE}/models/{model_name}");
        let res = self
            .http
            .get(&url)
            .header("x-goog-api-key", key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to reach Google AI Studio API: {e}")))?;

        let status = res.status();
        let body_text = res.text().await.unwrap_or_default();

        if !status.is_success() {
            let msg = extract_google_error(&body_text)
                .unwrap_or_else(|| format!("Google AI Studio returned HTTP {}", status.as_u16()));
            return Err(AppError::BadRequest(msg));
        }

        let parsed: serde_json::Value = serde_json::from_str(&body_text).unwrap_or_default();
        let display_name = parsed
            .get("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or(model_name);

        Ok(display_name.to_string())
    }

    /// Translates a batch of subtitle cues using Gemini structured JSON output.
    pub async fn translate_batch(
        &self,
        api_key: &str,
        model: &str,
        source_language: Option<&str>,
        target_language: &str,
        format: SubtitleFormat,
        cues: &[SubtitleCue],
        prev_context: &[(String, String)],
        media_hint: Option<&str>,
    ) -> Result<HashMap<usize, String>, AppError> {
        if cues.is_empty() {
            return Ok(HashMap::new());
        }

        let clean_model = model.trim().trim_start_matches("models/");
        let model_name = if clean_model.is_empty() {
            "gemini-2.5-flash"
        } else {
            clean_model
        };

        let url = format!("{GEMINI_API_BASE}/models/{model_name}:generateContent");

        let format_rules = match format {
            SubtitleFormat::Ass => {
                "Input subtitles are ASS/SSA format. Preserve all ASS override tags like {\\i1}, {\\i0}, {\\b1}, {\\an8}, {\\pos(...)}, {\\c&H...&}, and line breaks \\N verbatim in their appropriate positions. Never convert \\N to raw newlines if avoidable, and never translate tag names."
            }
            SubtitleFormat::Srt => {
                "Input subtitles are SRT format. Preserve any inline HTML tags like <i>, </i>, <b>, </b> and line breaks cleanly."
            }
        };

        let src_lang_desc = source_language
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("the source language (generally English)");

        let system_instruction = format!(
            "You are an expert anime subtitle translator.\n\
            Translate each subtitle entry from {src_lang_desc} into natural, idiomatic {target_language}.\n\
            Rules:\n\
            1. Keep the exact integer `id` for every input item. Return exactly one output item per input `id` in `translations`.\n\
            2. Match anime conversational tone, pronouns, and emotional nuance naturally in {target_language}.\n\
            3. {format_rules}\n\
            4. Do not merge lines across different IDs and do not add commentary or notes."
        );

        let input_items: Vec<InputCueItem<'_>> = cues
            .iter()
            .map(|c| InputCueItem {
                id: c.id,
                text: &c.text,
            })
            .collect();

        let mut user_prompt = String::new();
        if let Some(hint) = media_hint.filter(|h| !h.trim().is_empty()) {
            user_prompt.push_str(&format!("Media filename / title context: {hint}\n"));
        }
        if !prev_context.is_empty() {
            user_prompt.push_str("Previous translated lines for dialogue continuity (do NOT include these in output):\n");
            for (src, dst) in prev_context {
                user_prompt.push_str(&format!("- \"{src}\" => \"{dst}\"\n"));
            }
            user_prompt.push('\n');
        }
        user_prompt.push_str("Translate these subtitle cues:\n");
        user_prompt.push_str(
            &serde_json::to_string(&input_items)
                .map_err(|e| AppError::Internal(format!("Failed to serialize cues: {e}")))?,
        );

        let payload = json!({
            "systemInstruction": {
                "parts": [{ "text": system_instruction }]
            },
            "contents": [{
                "role": "user",
                "parts": [{ "text": user_prompt }]
            }],
            "generationConfig": {
                "temperature": 0.25,
                "responseMimeType": "application/json",
                "responseSchema": {
                    "type": "OBJECT",
                    "properties": {
                        "translations": {
                            "type": "ARRAY",
                            "items": {
                                "type": "OBJECT",
                                "properties": {
                                    "id": { "type": "INTEGER" },
                                    "text": { "type": "STRING" }
                                },
                                "required": ["id", "text"]
                            }
                        }
                    },
                    "required": ["translations"]
                }
            }
        });

        let max_attempts = 4;
        let mut last_err = String::new();

        for attempt in 1..=max_attempts {
            let response = self
                .http
                .post(&url)
                .header("x-goog-api-key", api_key.trim())
                .json(&payload)
                .send()
                .await;

            match response {
                Ok(res) => {
                    let status = res.status();
                    let retry_after_secs = res
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok());
                    let body = res.text().await.unwrap_or_default();

                    if status.is_success() {
                        return parse_gemini_translation_payload(&body);
                    }

                    let google_msg = extract_google_error(&body)
                        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));

                    // Retry on rate limit or transient upstream errors
                    if status.as_u16() == 429 || status.is_server_error() {
                        last_err = google_msg.clone();
                        if attempt < max_attempts {
                            let wait_secs = retry_after_secs.unwrap_or(match attempt {
                                1 => 5,
                                2 => 15,
                                _ => 30,
                            });
                            warn!(
                                "Gemini API returned {} ({}). Retrying batch in {}s (attempt {}/{})",
                                status.as_u16(),
                                google_msg,
                                wait_secs,
                                attempt,
                                max_attempts
                            );
                            tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                            continue;
                        }
                    } else {
                        return Err(AppError::BadRequest(format!(
                            "Google AI Studio error: {google_msg}"
                        )));
                    }
                }
                Err(e) => {
                    last_err = format!("Network error contacting Google AI Studio: {e}");
                    if attempt < max_attempts {
                        let wait_secs = 4 * attempt as u64;
                        warn!("{last_err}. Retrying in {wait_secs}s...");
                        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                        continue;
                    }
                }
            }
        }

        Err(AppError::Internal(format!(
            "Gemini translation failed after {max_attempts} attempts: {last_err}"
        )))
    }
}

fn extract_google_error(body: &str) -> Option<String> {
    let parsed: GeminiErrorEnvelope = serde_json::from_str(body).ok()?;
    let err = parsed.error?;
    match (err.status, err.message) {
        (Some(status), Some(msg)) => Some(format!("{status}: {msg}")),
        (None, Some(msg)) => Some(msg),
        (Some(status), None) => Some(status),
        (None, None) => None,
    }
}

fn parse_gemini_translation_payload(body: &str) -> Result<HashMap<usize, String>, AppError> {
    let envelope: GeminiGenerateResponse = serde_json::from_str(body)
        .map_err(|e| AppError::Internal(format!("Invalid Gemini JSON response: {e}")))?;

    let candidate = envelope
        .candidates
        .and_then(|mut c| if c.is_empty() { None } else { Some(c.remove(0)) })
        .ok_or_else(|| {
            let reason = envelope
                .prompt_feedback
                .map(|v| v.to_string())
                .unwrap_or_else(|| "no candidates returned".into());
            AppError::Internal(format!("Gemini returned empty response ({reason})"))
        })?;

    let raw_text = candidate
        .content
        .and_then(|c| c.parts)
        .and_then(|parts| parts.into_iter().find_map(|p| p.text))
        .ok_or_else(|| {
            let reason = candidate.finish_reason.unwrap_or_else(|| "UNKNOWN".into());
            AppError::Internal(format!("Gemini candidate contained no text (finishReason: {reason})"))
        })?;

    let cleaned = strip_markdown_json_fence(&raw_text);
    let parsed: GeminiTranslateResponse = serde_json::from_str(cleaned).map_err(|e| {
        AppError::Internal(format!(
            "Failed to parse structured translation JSON from Gemini: {e}"
        ))
    })?;

    let mut map = HashMap::with_capacity(parsed.translations.len());
    for item in parsed.translations {
        map.insert(item.id, item.text);
    }
    Ok(map)
}

fn strip_markdown_json_fence(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.trim().strip_suffix("```").unwrap_or(rest).trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.trim().strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gemini_translation_payload() {
        let body = r#"{
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"translations\":[{\"id\":1,\"text\":\"Halo\"},{\"id\":2,\"text\":\"Dunia\"}]}"
                    }]
                },
                "finishReason": "STOP"
            }]
        }"#;
        let map = parse_gemini_translation_payload(body).unwrap();
        assert_eq!(map.get(&1).map(String::as_str), Some("Halo"));
        assert_eq!(map.get(&2).map(String::as_str), Some("Dunia"));
    }

    #[test]
    fn test_extract_google_error() {
        let body = r#"{"error":{"code":400,"message":"API key not valid.","status":"INVALID_ARGUMENT"}}"#;
        assert_eq!(
            extract_google_error(body).as_deref(),
            Some("INVALID_ARGUMENT: API key not valid.")
        );
    }
}
