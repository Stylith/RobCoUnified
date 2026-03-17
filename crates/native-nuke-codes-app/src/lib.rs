use chrono::Local;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct NukeCodesData {
    pub alpha: String,
    pub bravo: String,
    pub charlie: String,
    pub source: String,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Default)]
pub enum NukeCodesView {
    #[default]
    Unloaded,
    Data(NukeCodesData),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NukeCodesEvent {
    None,
    Refresh,
    Back,
}

const PROVIDERS: &[(&str, &str)] = &[
    ("NukaCrypt", "https://nukacrypt.com/"),
    (
        "NukaCrypt Legacy",
        "https://nukacrypt.com/php/home.php?hm=1",
    ),
    ("NukaPD Mirror", "https://www.nukapd.com/silo-codes"),
    ("NukaTrader Mirror", "https://nukatrader.com/launchcodes/"),
];

pub fn resolve_nuke_codes_event(refresh: bool, back: bool) -> NukeCodesEvent {
    if refresh {
        NukeCodesEvent::Refresh
    } else if back {
        NukeCodesEvent::Back
    } else {
        NukeCodesEvent::None
    }
}

pub fn fetch_nuke_codes() -> NukeCodesView {
    let mut last_error = "no provider attempts".to_string();
    for (source, url) in PROVIDERS {
        match fetch_html(url).and_then(|html| extract_codes(&html).map(|(a, b, c)| (a, b, c))) {
            Ok((alpha, bravo, charlie)) => {
                return NukeCodesView::Data(NukeCodesData {
                    alpha,
                    bravo,
                    charlie,
                    source: (*source).to_string(),
                    fetched_at: Local::now().format("%Y-%m-%d %I:%M %p").to_string(),
                });
            }
            Err(err) => {
                last_error = format!("{source}: {err}");
            }
        }
    }
    NukeCodesView::Error(last_error)
}

fn fetch_html(url: &str) -> Result<String, String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--connect-timeout", "8", "--max-time", "16", url])
        .output()
        .map_err(|e| format!("curl spawn failed: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {}", err.trim()));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("invalid utf8: {e}"))
}

fn extract_codes(html: &str) -> Result<(String, String, String), String> {
    let alpha = extract_code_for(html, &["alpha", "site alpha", "silo alpha"]);
    let bravo = extract_code_for(html, &["bravo", "site bravo", "silo bravo"]);
    let charlie = extract_code_for(html, &["charlie", "site charlie", "silo charlie"]);

    match (alpha, bravo, charlie) {
        (Some(a), Some(b), Some(c)) => Ok((a, b, c)),
        _ => Err("could not parse alpha/bravo/charlie codes".to_string()),
    }
}

fn extract_code_for(html: &str, labels: &[&str]) -> Option<String> {
    let lower = html.to_lowercase();
    labels
        .iter()
        .find_map(|label| {
            let mut start = 0usize;
            while let Some(pos) = lower[start..].find(label) {
                let abs = start + pos;
                let left = abs.saturating_sub(120);
                let right = (abs + 220).min(html.len());
                if let Some(code) = first_eight_digit_code(&html[left..right]) {
                    return Some(code);
                }
                start = abs + label.len();
            }
            None
        })
        .or_else(|| first_eight_digit_code(html))
}

fn first_eight_digit_code(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.len() < 8 {
        return None;
    }
    for i in 0..=(bytes.len() - 8) {
        let window = &bytes[i..i + 8];
        if !window.iter().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let prev_ok = i == 0 || !bytes[i - 1].is_ascii_digit();
        let next_ok = i + 8 == bytes.len() || !bytes[i + 8].is_ascii_digit();
        if prev_ok && next_ok {
            return Some(String::from_utf8_lossy(window).to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_nuke_codes_event_prefers_refresh() {
        assert_eq!(
            resolve_nuke_codes_event(true, true),
            NukeCodesEvent::Refresh
        );
        assert_eq!(resolve_nuke_codes_event(false, true), NukeCodesEvent::Back);
        assert_eq!(resolve_nuke_codes_event(false, false), NukeCodesEvent::None);
    }

    #[test]
    fn first_eight_digit_code_finds_isolated_code() {
        assert_eq!(
            first_eight_digit_code("abc 12345678 def"),
            Some("12345678".to_string())
        );
        assert_eq!(first_eight_digit_code("abc 123456789 def"), None);
    }
}
