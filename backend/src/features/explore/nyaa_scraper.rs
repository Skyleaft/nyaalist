use chrono::{TimeZone, Utc};
use scraper::{Html, Selector};
use std::sync::LazyLock;

use crate::core::error::AppError;
use crate::features::explore::models::NyaaRelease;
use crate::features::explore::title_parser::parse_release_title;

static ROW_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("table.torrent-list tbody tr").unwrap()
});
static TITLE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("td:nth-child(2) a:not(.comments)").unwrap()
});
static MAGNET_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("td:nth-child(3) a[href^='magnet:']").unwrap()
});
static TORRENT_FILE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("td:nth-child(3) a[href^='/download/']").unwrap()
});
static CATEGORY_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("td:nth-child(1) a").unwrap()
});
static DATE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("td[data-timestamp]").unwrap()
});

pub async fn scrape_nyaa_category(
    base_url: &str,
    category: &str,
    page: u32,
    query: Option<&str>,
    sort: Option<&str>,
    order: Option<&str>,
    user_agent: &str,
) -> Result<Vec<NyaaRelease>, AppError> {
    let mut url = format!("{}/?c={}&p={}", base_url.trim_end_matches('/'), category, page);
    if let Some(q) = query {
        if !q.trim().is_empty() {
            url.push_str(&format!("&q={}", urlencoding::encode(q.trim())));
        }
    }
    if let Some(s) = normalize_sort(sort) {
        url.push_str(&format!("&s={}", urlencoding::encode(s)));
    }
    if let Some(o) = normalize_order(order) {
        url.push_str(&format!("&o={}", urlencoding::encode(o)));
    }

    scrape_url(&url, user_agent).await
}

pub async fn scrape_nyaa_user(
    base_url: &str,
    username: &str,
    page: u32,
    query: Option<&str>,
    sort: Option<&str>,
    order: Option<&str>,
    user_agent: &str,
) -> Result<Vec<NyaaRelease>, AppError> {
    let mut url = format!(
        "{}/user/{}?p={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(username.trim()),
        page
    );
    if let Some(q) = query {
        if !q.trim().is_empty() {
            url.push_str(&format!("&q={}", urlencoding::encode(q.trim())));
        }
    }
    if let Some(s) = normalize_sort(sort) {
        url.push_str(&format!("&s={}", urlencoding::encode(s)));
    }
    if let Some(o) = normalize_order(order) {
        url.push_str(&format!("&o={}", urlencoding::encode(o)));
    }

    let mut releases = scrape_url(&url, user_agent).await?;
    // Explicitly set uploader to the requested user if not parsed from brackets
    for rel in &mut releases {
        if rel.uploader.is_none() {
            rel.uploader = Some(username.to_string());
        }
    }

    Ok(releases)
}

fn normalize_sort(sort: Option<&str>) -> Option<&'static str> {
    match sort {
        Some("date") | Some("id") => Some("id"),
        Some("size") => Some("size"),
        Some("seeders") => Some("seeders"),
        Some("leechers") => Some("leechers"),
        Some("downloads") => Some("downloads"),
        _ => None,
    }
}

fn normalize_order(order: Option<&str>) -> Option<&'static str> {
    match order {
        Some("asc") => Some("asc"),
        Some("desc") => Some("desc"),
        _ => None,
    }
}

async fn scrape_url(url: &str, user_agent: &str) -> Result<Vec<NyaaRelease>, AppError> {
    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {}", e)))?;

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Nyaa request failed: {}", e)))?;

    if !res.status().is_success() {
        return Err(AppError::Internal(format!(
            "Nyaa responded with HTTP status {}",
            res.status()
        )));
    }

    let html_content = res
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to read Nyaa HTML: {}", e)))?;

    parse_nyaa_html(&html_content)
}

pub fn parse_nyaa_html(html: &str) -> Result<Vec<NyaaRelease>, AppError> {
    let document = Html::parse_document(html);
    let mut releases = Vec::new();

    for row in document.select(&ROW_SELECTOR) {
        let title_elem = match row.select(&TITLE_SELECTOR).last() {
            Some(elem) => elem,
            None => continue,
        };

        let title = title_elem.text().collect::<String>().trim().to_string();
        let view_href = title_elem.value().attr("href").unwrap_or("");
        let id: i64 = view_href
            .strip_prefix("/view/")
            .and_then(|s| s.split('#').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if id == 0 {
            continue;
        }

        let magnet_uri = row
            .select(&MAGNET_SELECTOR)
            .next()
            .and_then(|el| el.value().attr("href"))
            .unwrap_or("")
            .to_string();

        let torrent_file_url = row
            .select(&TORRENT_FILE_SELECTOR)
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|href| href.to_string());

        let category = row
            .select(&CATEGORY_SELECTOR)
            .next()
            .and_then(|el| el.value().attr("title"))
            .unwrap_or("Anime")
            .to_string();

        let cells: Vec<_> = row.select(&Selector::parse("td").unwrap()).collect();
        if cells.len() < 8 {
            continue;
        }

        let size_str = cells[3].text().collect::<String>().trim().to_string();
        let file_size_bytes = parse_size_bytes(&size_str);

        let timestamp: i64 = row
            .select(&DATE_SELECTOR)
            .next()
            .and_then(|el| el.value().attr("data-timestamp"))
            .and_then(|ts| ts.parse().ok())
            .unwrap_or_else(|| Utc::now().timestamp());

        let nyaa_created_at = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_else(Utc::now);

        let seeders: i32 = cells[5]
            .text()
            .collect::<String>()
            .trim()
            .parse()
            .unwrap_or(0);
        let leechers: i32 = cells[6]
            .text()
            .collect::<String>()
            .trim()
            .parse()
            .unwrap_or(0);
        let completed_count: i32 = cells[7]
            .text()
            .collect::<String>()
            .trim()
            .parse()
            .unwrap_or(0);

        let parsed = parse_release_title(&title);
        let uploader = parsed.uploader.clone();

        releases.push(NyaaRelease {
            id,
            title,
            uploader,
            magnet_uri,
            torrent_file_url,
            category,
            file_size_bytes,
            file_size_formatted: size_str,
            seeders,
            leechers,
            completed_count,
            nyaa_created_at,
            parsed,
            anilist_metadata: None,
        });
    }

    Ok(releases)
}

fn parse_size_bytes(size_str: &str) -> i64 {
    let parts: Vec<&str> = size_str.split_whitespace().collect();
    if parts.len() < 2 {
        return 0;
    }

    let val: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = parts[1].to_uppercase();

    let multiplier = match unit.as_str() {
        "B" => 1.0,
        "KIB" | "KB" => 1024.0,
        "MIB" | "MB" => 1024.0 * 1024.0,
        "GIB" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "TIB" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };

    (val * multiplier) as i64
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(b as char)
                }
                b' ' => encoded.push('+'),
                _ => encoded.push_str(&format!("%{:02X}", b)),
            }
        }
        encoded
    }
}
