use std::collections::HashMap;
use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubtitleFormat {
    Ass,
    Srt,
}

impl SubtitleFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Ass => "ass",
            Self::Srt => "srt",
        }
    }

    pub fn from_codec(codec: &str) -> Self {
        match codec.to_lowercase().as_str() {
            "ass" | "ssa" => Self::Ass,
            _ => Self::Srt,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCue {
    pub id: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
enum ParsedLine {
    Raw(String),
    AssDialogue {
        cue_id: Option<usize>,
        prefix: String,
        original_text: String,
    },
    SrtCue {
        cue_id: usize,
        header: String,
        timing: String,
        original_text: String,
    },
}

#[derive(Debug, Clone)]
pub struct ParsedSubtitle {
    pub format: SubtitleFormat,
    cues: Vec<SubtitleCue>,
    lines: Vec<ParsedLine>,
}

impl ParsedSubtitle {
    pub fn parse(content: &str, format: SubtitleFormat) -> Self {
        let normalized = content
            .strip_prefix('\u{feff}')
            .unwrap_or(content)
            .replace("\r\n", "\n")
            .replace('\r', "\n");

        match format {
            SubtitleFormat::Ass => Self::parse_ass(&normalized),
            SubtitleFormat::Srt => Self::parse_srt(&normalized),
        }
    }

    pub fn cues(&self) -> &[SubtitleCue] {
        &self.cues
    }

    fn parse_ass(content: &str) -> Self {
        let mut cues = Vec::new();
        let mut lines = Vec::new();
        let mut in_events = false;
        let mut text_col_idx: usize = 9;
        let mut next_id: usize = 1;

        for raw_line in content.split('\n') {
            let trimmed = raw_line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_events = trimmed.eq_ignore_ascii_case("[Events]");
                lines.push(ParsedLine::Raw(raw_line.to_string()));
                continue;
            }

            if in_events {
                if let Some(fmt_rest) = strip_prefix_ignore_ascii_case(trimmed, "Format:") {
                    let cols: Vec<&str> = fmt_rest.split(',').map(|c| c.trim()).collect();
                    if let Some(pos) = cols.iter().position(|c| c.eq_ignore_ascii_case("Text")) {
                        text_col_idx = pos;
                    }
                    lines.push(ParsedLine::Raw(raw_line.to_string()));
                    continue;
                }

                if let Some(dialogue_body) = strip_prefix_ignore_ascii_case(raw_line, "Dialogue:") {
                    let parts: Vec<&str> = dialogue_body.splitn(text_col_idx + 1, ',').collect();
                    if parts.len() == text_col_idx + 1 {
                        let prefix = format!("Dialogue:{},", parts[..text_col_idx].join(","));
                        let text = parts[text_col_idx].to_string();
                        if is_translatable_ass_text(&text) {
                            let id = next_id;
                            next_id += 1;
                            cues.push(SubtitleCue {
                                id,
                                text: text.clone(),
                            });
                            lines.push(ParsedLine::AssDialogue {
                                cue_id: Some(id),
                                prefix,
                                original_text: text,
                            });
                        } else {
                            lines.push(ParsedLine::AssDialogue {
                                cue_id: None,
                                prefix,
                                original_text: text,
                            });
                        }
                        continue;
                    }
                }
            }

            lines.push(ParsedLine::Raw(raw_line.to_string()));
        }

        Self {
            format: SubtitleFormat::Ass,
            cues,
            lines,
        }
    }

    fn parse_srt(content: &str) -> Self {
        let mut cues = Vec::new();
        let mut lines = Vec::new();
        let mut next_id: usize = 1;

        let raw_lines: Vec<&str> = content.split('\n').collect();
        let mut i = 0;

        while i < raw_lines.len() {
            let line = raw_lines[i].trim();
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Look for timing line either on current line or next line
            let (header, timing_idx) = if i + 1 < raw_lines.len()
                && raw_lines[i + 1].contains("-->")
            {
                (raw_lines[i].trim().to_string(), i + 1)
            } else if raw_lines[i].contains("-->") {
                (next_id.to_string(), i)
            } else {
                lines.push(ParsedLine::Raw(raw_lines[i].to_string()));
                i += 1;
                continue;
            };

            let timing = raw_lines[timing_idx].trim().to_string();
            let mut text_lines = Vec::new();
            let mut j = timing_idx + 1;
            while j < raw_lines.len() && !raw_lines[j].trim().is_empty() {
                text_lines.push(raw_lines[j]);
                j += 1;
            }

            let text = text_lines.join("\n");
            let id = next_id;
            next_id += 1;

            if !text.trim().is_empty() {
                cues.push(SubtitleCue {
                    id,
                    text: text.clone(),
                });
            }

            lines.push(ParsedLine::SrtCue {
                cue_id: id,
                header,
                timing,
                original_text: text,
            });

            i = j + 1;
        }

        Self {
            format: SubtitleFormat::Srt,
            cues,
            lines,
        }
    }

    pub fn apply_translations(&self, translations: &HashMap<usize, String>) -> String {
        match self.format {
            SubtitleFormat::Ass => {
                let mut out = Vec::with_capacity(self.lines.len());
                for item in &self.lines {
                    match item {
                        ParsedLine::Raw(line) => out.push(line.clone()),
                        ParsedLine::AssDialogue {
                            cue_id,
                            prefix,
                            original_text,
                        } => {
                            let final_text = cue_id
                                .and_then(|id| translations.get(&id))
                                .map(|s| sanitize_ass_text(s))
                                .unwrap_or_else(|| original_text.clone());
                            out.push(format!("{prefix}{final_text}"));
                        }
                        ParsedLine::SrtCue { .. } => {}
                    }
                }
                out.join("\n")
            }
            SubtitleFormat::Srt => {
                let mut blocks = Vec::with_capacity(self.lines.len());
                for item in &self.lines {
                    match item {
                        ParsedLine::SrtCue {
                            cue_id,
                            header,
                            timing,
                            original_text,
                        } => {
                            let final_text = translations
                                .get(cue_id)
                                .map(|s| s.replace("\r\n", "\n").trim().to_string())
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| original_text.clone());
                            blocks.push(format!("{header}\n{timing}\n{final_text}"));
                        }
                        ParsedLine::Raw(raw) if !raw.trim().is_empty() => {
                            blocks.push(raw.clone());
                        }
                        _ => {}
                    }
                }
                let mut result = blocks.join("\n\n");
                if !result.is_empty() {
                    result.push('\n');
                }
                result
            }
        }
    }
}

fn strip_prefix_ignore_ascii_case<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() >= prefix.len() && input[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&input[prefix.len()..])
    } else {
        None
    }
}

fn sanitize_ass_text(text: &str) -> String {
    text.replace("\r\n", "\\N")
        .replace(['\r', '\n'], "\\N")
}

fn is_translatable_ass_text(text: &str) -> bool {
    let tag_re = Regex::new(r"\{[^}]*\}").expect("valid ass tag regex");

    // Check for vector drawing mode (\p1 .. \p9) inside override tags
    for cap in tag_re.find_iter(text) {
        let block = cap.as_str();
        if let Some(pos) = block.find("\\p") {
            let after = &block[pos + 2..];
            if after.chars().next().is_some_and(|c| ('1'..='9').contains(&c)) {
                return false;
            }
        }
    }

    let stripped = tag_re.replace_all(text, "");
    let visible = stripped
        .replace("\\N", "")
        .replace("\\n", "")
        .replace("\\h", "");

    !visible.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_rebuild_srt() {
        let sample = "\u{feff}1\r\n00:00:01,000 --> 00:00:04,000\r\nHello world!\r\nHow are you?\r\n\r\n2\r\n00:00:05,500 --> 00:00:08,000\r\nI am fine.\r\n";
        let parsed = ParsedSubtitle::parse(sample, SubtitleFormat::Srt);
        assert_eq!(parsed.cues().len(), 2);
        assert_eq!(parsed.cues()[0].id, 1);
        assert_eq!(parsed.cues()[0].text, "Hello world!\nHow are you?");
        assert_eq!(parsed.cues()[1].id, 2);

        let mut map = HashMap::new();
        map.insert(1, "Halo dunia!\nApa kabar?".to_string());
        map.insert(2, "Aku baik-baik saja.".to_string());

        let rebuilt = parsed.apply_translations(&map);
        assert!(rebuilt.contains("00:00:01,000 --> 00:00:04,000\nHalo dunia!\nApa kabar?"));
        assert!(rebuilt.contains("00:00:05,500 --> 00:00:08,000\nAku baik-baik saja."));
    }

    #[test]
    fn test_parse_and_rebuild_ass_preserving_drawings_and_tags() {
        let sample = r#"[Script Info]
Title: Test Anime

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Comment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,Note
Dialogue: 0,0:00:01.00,0:00:03.50,Default,,0,0,0,,{\i1}Hello,{\i0}\Nworld!
Dialogue: 0,0:00:04.00,0:00:05.00,Signs,,0,0,0,,{\p1}m 0 0 l 100 0 100 100{\p0}
Dialogue: 0,0:00:06.00,0:00:08.00,Default,,0,0,0,,Good, morning!
"#;
        let parsed = ParsedSubtitle::parse(sample, SubtitleFormat::Ass);
        assert_eq!(parsed.cues().len(), 2);
        assert_eq!(parsed.cues()[0].text, "{\\i1}Hello,{\\i0}\\Nworld!");
        assert_eq!(parsed.cues()[1].text, "Good, morning!");

        let mut map = HashMap::new();
        map.insert(1, "{\\i1}Halo,{\\i0}\ndunia!".to_string());
        map.insert(2, "Selamat, pagi!".to_string());

        let rebuilt = parsed.apply_translations(&map);
        assert!(rebuilt.contains("Comment: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,Note"));
        assert!(rebuilt.contains("Dialogue: 0,0:00:01.00,0:00:03.50,Default,,0,0,0,,{\\i1}Halo,{\\i0}\\Ndunia!"));
        assert!(rebuilt.contains("{\\p1}m 0 0 l 100 0 100 100{\\p0}"));
        assert!(rebuilt.contains("Dialogue: 0,0:00:06.00,0:00:08.00,Default,,0,0,0,,Selamat, pagi!"));
    }
}
