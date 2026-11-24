use chrono::{TimeZone, Utc};
use scraper::{Html, Selector};
use std::sync::LazyLock;

use crate::core::error::AppError;
use crate::features::explore::models::{NyaaFileItem, NyaaRelease, NyaaReleaseDetail};
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

static PANEL_TITLE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".panel-title").unwrap()
});
static PANEL_BODY_ROW_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".panel-body .row").unwrap()
});
static VIEW_MAGNET_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a[href^='magnet:']").unwrap()
});
static VIEW_DOWNLOAD_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a[href^='/download/']").unwrap()
});
static TORRENT_DESC_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("#torrent-description").unwrap()
});
static FILE_LIST_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".torrent-file-list").unwrap()
});
static ANCHOR_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a").unwrap()
});
static KBD_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("kbd").unwrap()
});
static UL_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("ul").unwrap()
});
static FOLDER_A_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a.folder").unwrap()
});
static FILE_SIZE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".file-size").unwrap()
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

pub async fn scrape_nyaa_view(
    base_url: &str,
    id: i64,
    user_agent: &str,
) -> Result<NyaaReleaseDetail, AppError> {
    let url = format!("{}/view/{}", base_url.trim_end_matches('/'), id);
    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build HTTP client: {}", e)))?;

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Nyaa request failed: {}", e)))?;

    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound(format!("Nyaa release with ID {} not found", id)));
    }

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

    parse_nyaa_view_html(id, &html_content)
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

pub fn parse_nyaa_view_html(id: i64, html: &str) -> Result<NyaaReleaseDetail, AppError> {
    let document = Html::parse_document(html);

    let title = match document.select(&PANEL_TITLE_SELECTOR).next() {
        Some(el) => el.text().collect::<String>().trim().to_string(),
        None => return Err(AppError::NotFound(format!("Torrent with id {} not found", id))),
    };

    if title.is_empty() || title == "404 Not Found" {
        return Err(AppError::NotFound(format!("Torrent with id {} not found", id)));
    }

    let mut category = "Anime".to_string();
    let mut nyaa_created_at = Utc::now();
    let mut uploader: Option<String> = None;
    let mut uploader_url: Option<String> = None;
    let mut information: Option<String> = None;
    let mut file_size_formatted = "0 B".to_string();
    let mut file_size_bytes = 0i64;
    let mut seeders = 0i32;
    let mut leechers = 0i32;
    let mut completed_count = 0i32;
    let mut info_hash: Option<String> = None;

    for row in document.select(&PANEL_BODY_ROW_SELECTOR) {
        let cells: Vec<_> = row.children().filter_map(scraper::ElementRef::wrap).collect();
        for i in 0..cells.len() {
            let label = cells[i].text().collect::<String>().trim().to_lowercase();
            if label.ends_with(':') && i + 1 < cells.len() {
                let val_el = cells[i + 1];
                let val_text = val_el.text().collect::<String>().trim().to_string();

                if label.contains("category") {
                    if !val_text.is_empty() {
                        category = val_text;
                    }
                } else if label.contains("date") {
                    if let Some(ts_str) = val_el.value().attr("data-timestamp") {
                        if let Ok(ts) = ts_str.parse::<i64>() {
                            if let Some(dt) = Utc.timestamp_opt(ts, 0).single() {
                                nyaa_created_at = dt;
                            }
                        }
                    }
                } else if label.contains("submitter") {
                    if let Some(a) = val_el.select(&ANCHOR_SELECTOR).next() {
                        uploader = Some(a.text().collect::<String>().trim().to_string());
                        uploader_url = a.value().attr("href").map(String::from);
                    } else if !val_text.is_empty() && val_text != "Anonymous" {
                        uploader = Some(val_text);
                    }
                } else if label.contains("information") {
                    if !val_text.is_empty() && val_text != "No information." {
                        information = Some(val_text);
                    }
                } else if label.contains("file size") {
                    file_size_formatted = val_text.clone();
                    file_size_bytes = parse_size_bytes(&val_text);
                } else if label.contains("seeders") {
                    seeders = val_text.parse().unwrap_or(0);
                } else if label.contains("leechers") {
                    leechers = val_text.parse().unwrap_or(0);
                } else if label.contains("completed") {
                    completed_count = val_text.parse().unwrap_or(0);
                } else if label.contains("info hash") {
                    if let Some(kbd) = val_el.select(&KBD_SELECTOR).next() {
                        info_hash = Some(kbd.text().collect::<String>().trim().to_string());
                    } else if !val_text.is_empty() {
                        info_hash = Some(val_text);
                    }
                }
            }
        }
    }

    let magnet_uri = document
        .select(&VIEW_MAGNET_SELECTOR)
        .next()
        .and_then(|el| el.value().attr("href"))
        .unwrap_or("")
        .to_string();

    let torrent_file_url = document
        .select(&VIEW_DOWNLOAD_SELECTOR)
        .next()
        .and_then(|el| el.value().attr("href"))
        .map(String::from);

    let description_markdown = document
        .select(&TORRENT_DESC_SELECTOR)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|desc| !desc.is_empty() && desc != "#### No description.");

    let file_list = if let Some(container) = document.select(&FILE_LIST_SELECTOR).next() {
        if let Some(root_ul) = container.select(&UL_SELECTOR).next() {
            parse_file_tree(root_ul)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let parsed = parse_release_title(&title);
    if uploader.is_none() && parsed.uploader.is_some() {
        uploader = parsed.uploader.clone();
    }

    Ok(NyaaReleaseDetail {
        id,
        title,
        uploader,
        uploader_url,
        magnet_uri,
        torrent_file_url,
        category,
        file_size_bytes,
        file_size_formatted,
        seeders,
        leechers,
        completed_count,
        nyaa_created_at,
        info_hash,
        information,
        description_markdown,
        file_list,
        parsed,
        anilist_metadata: None,
    })
}

fn parse_file_tree(ul_el: scraper::ElementRef) -> Vec<NyaaFileItem> {
    let mut items = Vec::new();
    for child in ul_el.children() {
        if let Some(child_el) = scraper::ElementRef::wrap(child) {
            if child_el.value().name() == "li" {
                let sub_ul = child_el.select(&UL_SELECTOR).next();
                let folder_a = child_el.select(&FOLDER_A_SELECTOR).next();

                if folder_a.is_some() || sub_ul.is_some() {
                    let folder_name = folder_a
                        .map(|a| a.text().collect::<String>().trim().to_string())
                        .unwrap_or_else(|| {
                            let mut name = String::new();
                            for li_child in child_el.children() {
                                if let Some(c_el) = scraper::ElementRef::wrap(li_child) {
                                    if c_el.value().name() == "ul" {
                                        break;
                                    }
                                }
                                if let Some(t) = li_child.value().as_text() {
                                    name.push_str(t);
                                }
                            }
                            let trimmed = name.trim().to_string();
                            if trimmed.is_empty() {
                                "Folder".to_string()
                            } else {
                                trimmed
                            }
                        });

                    let children = if let Some(sub) = sub_ul {
                        parse_file_tree(sub)
                    } else {
                        Vec::new()
                    };

                    items.push(NyaaFileItem {
                        name: folder_name,
                        size_formatted: None,
                        size_bytes: None,
                        is_folder: true,
                        children,
                    });
                } else {
                    let size_formatted = child_el.select(&FILE_SIZE_SELECTOR).next().map(|el| {
                        el.text()
                            .collect::<String>()
                            .trim()
                            .trim_start_matches('(')
                            .trim_end_matches(')')
                            .trim()
                            .to_string()
                    });

                    let size_bytes = size_formatted.as_deref().map(parse_size_bytes);

                    let mut file_name = String::new();
                    for node in child_el.children() {
                        if let Some(node_el) = scraper::ElementRef::wrap(node) {
                            if node_el.value().name() == "i" {
                                continue;
                            }
                            if node_el.value().classes().any(|c| c == "file-size") {
                                continue;
                            }
                        }
                        if let Some(t) = node.value().as_text() {
                            file_name.push_str(t);
                        }
                    }

                    let clean_name = file_name.trim().to_string();
                    let final_name = if clean_name.is_empty() {
                        let full = child_el.text().collect::<String>();
                        if let Some(ref sz) = size_formatted {
                            full.replace(&format!("({})", sz), "")
                                .replace(sz, "")
                                .trim()
                                .to_string()
                        } else {
                            full.trim().to_string()
                        }
                    } else {
                        clean_name
                    };

                    items.push(NyaaFileItem {
                        name: final_name,
                        size_formatted,
                        size_bytes,
                        is_folder: false,
                        children: Vec::new(),
                    });
                }
            }
        }
    }
    items
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nyaa_view_html() {
        let html = r#"
        <div class="panel panel-default">
            <div class="panel-heading">
                <h3 class="panel-title">[Erai-raws] Sousou no Frieren - 01 [1080p][HEVC].mkv</h3>
            </div>
            <div class="panel-body">
                <div class="row">
                    <div class="col-md-1">Category:</div>
                    <div class="col-md-5"><a href="/?c=1_2">Anime - English-translated</a></div>
                    <div class="col-md-1">Date:</div>
                    <div class="col-md-5" data-timestamp="1700000000">2023-11-14 22:13 UTC</div>
                </div>
                <div class="row">
                    <div class="col-md-1">Submitter:</div>
                    <div class="col-md-5"><a href="/user/Erai-raws">Erai-raws</a></div>
                    <div class="col-md-1">Seeders:</div>
                    <div class="col-md-5"><span style="color: green;">120</span></div>
                </div>
                <div class="row">
                    <div class="col-md-1">Information:</div>
                    <div class="col-md-5">https://erai-raws.info</div>
                    <div class="col-md-1">Leechers:</div>
                    <div class="col-md-5"><span style="color: red;">5</span></div>
                </div>
                <div class="row">
                    <div class="col-md-1">File size:</div>
                    <div class="col-md-5">1.4 GiB</div>
                    <div class="col-md-1">Completed:</div>
                    <div class="col-md-5">1500</div>
                </div>
                <div class="row">
                    <div class="col-md-offset-6 col-md-1">Info hash:</div>
                    <div class="col-md-5"><kbd>abcdef1234567890abcdef1234567890abcdef12</kbd></div>
                </div>
            </div>
            <div class="panel-footer clearfix">
                <a href="/download/12345.torrent">Download Torrent</a>
                <a href="magnet:?xt=urn:btih:abcdef1234567890abcdef1234567890abcdef12&dn=test">Magnet</a>
            </div>
        </div>
        <div class="panel panel-default">
            <div class="panel-body" id="torrent-description">
                Episode 01 of Frieren. Encoded in HEVC.
            </div>
        </div>
        <div class="panel panel-default">
            <div class="torrent-file-list panel-body">
                <ul>
                    <li><i class="fa fa-file"></i>[Erai-raws] Sousou no Frieren - 01 [1080p][HEVC].mkv <span class="file-size">(1.4 GiB)</span></li>
                </ul>
            </div>
        </div>
        "#;

        let detail = parse_nyaa_view_html(12345, html).expect("should parse view html");
        assert_eq!(detail.id, 12345);
        assert_eq!(detail.title, "[Erai-raws] Sousou no Frieren - 01 [1080p][HEVC].mkv");
        assert_eq!(detail.uploader.as_deref(), Some("Erai-raws"));
        assert_eq!(detail.uploader_url.as_deref(), Some("/user/Erai-raws"));
        assert_eq!(detail.category, "Anime - English-translated");
        assert_eq!(detail.file_size_formatted, "1.4 GiB");
        assert_eq!(detail.seeders, 120);
        assert_eq!(detail.leechers, 5);
        assert_eq!(detail.completed_count, 1500);
        assert_eq!(detail.info_hash.as_deref(), Some("abcdef1234567890abcdef1234567890abcdef12"));
        assert_eq!(detail.information.as_deref(), Some("https://erai-raws.info"));
        assert_eq!(detail.torrent_file_url.as_deref(), Some("/download/12345.torrent"));
        assert!(detail.magnet_uri.starts_with("magnet:?"));
        assert_eq!(detail.description_markdown.as_deref(), Some("Episode 01 of Frieren. Encoded in HEVC."));
        assert_eq!(detail.file_list.len(), 1);
        assert_eq!(detail.file_list[0].name, "[Erai-raws] Sousou no Frieren - 01 [1080p][HEVC].mkv");
        assert_eq!(detail.file_list[0].size_formatted.as_deref(), Some("1.4 GiB"));
    }

    #[test]
    fn test_parse_nyaa_view_folder_files() {
        let html = r#"
        <div class="panel panel-default">
            <div class="panel-heading"><h3 class="panel-title">Batch Release</h3></div>
            <div class="panel-body">
                <div class="row">
                    <div class="col-md-1">Category:</div><div class="col-md-5">Anime</div>
                    <div class="col-md-1">Date:</div><div class="col-md-5" data-timestamp="1700000000">2023-11-14</div>
                </div>
            </div>
            <div class="panel-footer clearfix">
                <a href="magnet:?xt=urn:btih:1111111111111111111111111111111111111111">Magnet</a>
            </div>
        </div>
        <div class="panel panel-default">
            <div class="torrent-file-list panel-body">
                <ul>
                    <li>
                        <a href="" class="folder"><i class="fa fa-folder-open"></i>[Season 1]</a>
                        <ul>
                            <li><i class="fa fa-file"></i>Episode 01.mkv <span class="file-size">(300.0 MiB)</span></li>
                            <li><i class="fa fa-file"></i>Episode 02.mkv <span class="file-size">(310.0 MiB)</span></li>
                        </ul>
                    </li>
                </ul>
            </div>
        </div>
        "#;
        let detail = parse_nyaa_view_html(999, html).expect("should parse");
        assert_eq!(detail.file_list.len(), 1);
        assert!(detail.file_list[0].is_folder);
        assert_eq!(detail.file_list[0].name, "[Season 1]");
        assert_eq!(detail.file_list[0].children.len(), 2);
        assert_eq!(detail.file_list[0].children[0].name, "Episode 01.mkv");
        assert_eq!(detail.file_list[0].children[0].size_formatted.as_deref(), Some("300.0 MiB"));
        assert_eq!(detail.file_list[0].children[1].name, "Episode 02.mkv");
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_scrape_nyaa_view() {
        let detail = scrape_nyaa_view("https://nyaa.si", 2189813, "Mozilla/5.0")
            .await
            .expect("live scrape");
        assert_eq!(detail.id, 2189813);
        assert!(!detail.title.is_empty());
        assert!(detail.magnet_uri.starts_with("magnet:?"));
        assert!(detail.info_hash.is_some());
        assert!(!detail.file_list.is_empty());
    }
}
