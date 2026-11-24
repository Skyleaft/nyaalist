use regex::Regex;
use std::sync::LazyLock;
use crate::features::explore::models::ParsedRelease;

static UPLOADER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\[|\()([^\]\)]+)(?:\]|\))").expect("Valid regex")
});

static TAGS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[([^\]]+)\]").expect("Valid regex")
});

static CRC32_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b([0-9a-f]{8})\b").expect("Valid regex")
});

static RESOLUTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(480p|720p|1080p|2160p|4k)\b").expect("Valid regex")
});

static CODEC_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(hevc|x264|x265|h\.?264|h\.?265|avc|av1|10bit|8bit)\b").expect("Valid regex")
});

static AUDIO_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(aac|flac|ac3|eac3|opus|dual-audio|multisub|multiaudio)\b").expect("Valid regex")
});

static SOURCE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(cr\s+webrip|webrip|web-dl|bdrip|bluray|hdtv|dvdrip)\b").expect("Valid regex")
});

static EPISODE_PART_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:s(?:eason)?\s*\d{1,2}\s*(?:[-_ ]\s*)?)?(?:e(?:p(?:isode)?)?\.?|#|-)?\s*(\d{1,4}(?:\.\d+)?)(?:\s*v\d+)?$").expect("Valid regex")
});

static INLINE_EPISODE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[\s_\-]|\b)(?:s(?:eason)?\s*\d{1,2}\s*(?:[-_ ]\s*)?)?(?:e(?:p(?:isode)?)?\.?|#)\s*(\d{1,4}(?:\.\d+)?)(?:\s*v\d+)?(?:\b|$)").expect("Valid regex")
});

static TRAILING_EPISODE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[\s_\-])(\d{1,4}(?:\.\d+)?)(?:\s*v\d+)?$").expect("Valid regex")
});

static STRIP_TRAILING_TAGS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\[[^\]]+\]|\([^\)]+\)|\.(mkv|mp4|avi|webm|mov|m4v|flv|ts))$").expect("Valid regex")
});

static SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+").expect("Valid regex")
});

pub fn parse_release_title(raw_title: &str) -> ParsedRelease {
    let mut working_title = raw_title.trim().to_string();

    // 1. Extract leading uploader/release group, e.g. [Erai-raws], [Doomdos]
    let mut uploader = None;
    if let Some(caps) = UPLOADER_REGEX.captures(&working_title) {
        if let Some(m) = caps.get(1) {
            uploader = Some(m.as_str().trim().to_string());
            working_title = working_title[caps.get(0).unwrap().end()..].to_string();
            // Clean any leading separators following uploader, e.g. " - " or "_"
            working_title = working_title
                .trim_start_matches(|c: char| c == '-' || c == '_' || c == '|' || c == ':' || c.is_whitespace())
                .trim()
                .to_string();
        }
    }

    // 2. Extract technical tags and metadata from all bracketed blocks
    let mut resolution = None;
    let mut codec = None;
    let mut audio = None;
    let mut source = None;
    let mut crc32 = None;

    for caps in TAGS_REGEX.captures_iter(raw_title) {
        if let Some(m) = caps.get(1) {
            let content = m.as_str();

            if resolution.is_none() {
                if let Some(c) = RESOLUTION_REGEX.captures(content) {
                    resolution = Some(c.get(1).unwrap().as_str().to_string());
                }
            }
            if codec.is_none() {
                if let Some(c) = CODEC_REGEX.captures(content) {
                    codec = Some(c.get(1).unwrap().as_str().to_string());
                }
            }
            if audio.is_none() {
                if let Some(c) = AUDIO_REGEX.captures(content) {
                    audio = Some(c.get(1).unwrap().as_str().to_string());
                }
            }
            if source.is_none() {
                if let Some(c) = SOURCE_REGEX.captures(content) {
                    source = Some(c.get(1).unwrap().as_str().to_string());
                }
            }
            if crc32.is_none() {
                if let Some(c) = CRC32_REGEX.captures(content) {
                    crc32 = Some(c.get(1).unwrap().as_str().to_uppercase());
                }
            }
        }
    }

    // 3. Remove trailing bracket tags and file extensions from working_title
    while let Some(m) = STRIP_TRAILING_TAGS.find(&working_title) {
        working_title = working_title[..m.start()].trim().to_string();
    }
    working_title = working_title
        .trim_end_matches(|c: char| c == '-' || c == '_' || c == '|' || c == ':' || c.is_whitespace())
        .trim()
        .to_string();

    // 4. Extract Episode and Anime Title
    // Common pattern: "<Title> - <Episode>" or "<Title> <Episode>"
    let mut anime_title = working_title.clone();
    let mut episode = None;

    if let Some(pos) = working_title.rfind(" - ") {
        let potential_title = working_title[..pos].trim();
        let potential_ep_part = working_title[pos + 3..].trim();

        if let Some(ep_caps) = EPISODE_PART_REGEX.captures(potential_ep_part) {
            if let Some(ep_match) = ep_caps.get(1) {
                episode = Some(ep_match.as_str().to_string());
                anime_title = potential_title.to_string();
            }
        }
    }

    if episode.is_none() {
        if let Some(caps) = INLINE_EPISODE_REGEX.captures(&working_title) {
            if let Some(ep_match) = caps.get(1) {
                let ep_start = caps.get(0).unwrap().start();
                let prefix = working_title[..ep_start].trim().trim_end_matches(['-', '_', ' ']);
                if !prefix.is_empty() {
                    anime_title = prefix.to_string();
                    episode = Some(ep_match.as_str().to_string());
                }
            }
        } else if let Some(caps) = TRAILING_EPISODE_REGEX.captures(&working_title) {
            if let Some(ep_match) = caps.get(1) {
                let ep_start = caps.get(0).unwrap().start();
                let prefix = working_title[..ep_start].trim().trim_end_matches(['-', '_', ' ']);
                if !prefix.is_empty() {
                    anime_title = prefix.to_string();
                    episode = Some(ep_match.as_str().to_string());
                }
            }
        }
    }

    // Final cleanup of Anime Title
    anime_title = anime_title
        .trim_start_matches(|c: char| c == '-' || c == '_' || c == '|' || c == ':' || c.is_whitespace())
        .trim_end_matches(|c: char| c == '-' || c == '_' || c == '|' || c == ':' || c.is_whitespace())
        .trim()
        .to_string();
    anime_title = anime_title.replace('_', " ");
    anime_title = SPACE_REGEX.replace_all(&anime_title, " ").trim().to_string();

    ParsedRelease {
        uploader,
        anime_title,
        episode,
        resolution,
        codec,
        audio,
        source,
        crc32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_erai_raws() {
        let raw = "[Erai-raws] Dragon Ball Super: Beerus - 07 [1080p CR WEBRip HEVC AAC][MultiSub][4C1D82D9]";
        let parsed = parse_release_title(raw);

        assert_eq!(parsed.uploader.as_deref(), Some("Erai-raws"));
        assert_eq!(parsed.anime_title, "Dragon Ball Super: Beerus");
        assert_eq!(parsed.episode.as_deref(), Some("07"));
        assert_eq!(parsed.resolution.as_deref(), Some("1080p"));
        assert_eq!(parsed.crc32.as_deref(), Some("4C1D82D9"));
    }

    #[test]
    fn test_parse_kozame() {
        let raw = "[Kozame] Odekake Kozame (Little Shark's Outings) - 93 [1080p] [CC2F9599]";
        let parsed = parse_release_title(raw);

        assert_eq!(parsed.uploader.as_deref(), Some("Kozame"));
        assert_eq!(parsed.anime_title, "Odekake Kozame (Little Shark's Outings)");
        assert_eq!(parsed.episode.as_deref(), Some("93"));
        assert_eq!(parsed.resolution.as_deref(), Some("1080p"));
        assert_eq!(parsed.crc32.as_deref(), Some("CC2F9599"));
    }

    #[test]
    fn test_parse_doomdos_tensei_goblin() {
        let raw = "[Doomdos] - Tensei Goblin dakedo Shitsumon Aru - S01E08 [2160p IQ WEB-DL] [So What's Wrong with Getting Reborn as a Goblin_]";
        let parsed = parse_release_title(raw);

        assert_eq!(parsed.uploader.as_deref(), Some("Doomdos"));
        assert_eq!(parsed.anime_title, "Tensei Goblin dakedo Shitsumon Aru");
        assert_eq!(parsed.episode.as_deref(), Some("08"));
        assert_eq!(parsed.resolution.as_deref(), Some("2160p"));
        assert_eq!(parsed.source.as_deref(), Some("WEB-DL"));
    }

    #[test]
    fn test_parse_doomdos_seitokai() {
        let raw = "[Doomdos] - Seitokai ni mo Ana wa Aru! - S01E08 [2160p IQ WEB-DL]";
        let parsed = parse_release_title(raw);

        assert_eq!(parsed.uploader.as_deref(), Some("Doomdos"));
        assert_eq!(parsed.anime_title, "Seitokai ni mo Ana wa Aru!");
        assert_eq!(parsed.episode.as_deref(), Some("08"));
        assert_eq!(parsed.resolution.as_deref(), Some("2160p"));
        assert_eq!(parsed.source.as_deref(), Some("WEB-DL"));
    }
}
