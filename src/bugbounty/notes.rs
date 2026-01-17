//! Import/parse helpers for filesystem-based findings notes (`notes/findings/*.md`).

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::{Confidence, Finding, FindingStatus, Reachability, Severity};

#[derive(Debug, Clone)]
pub struct NoteField<T> {
    pub present: bool,
    pub value: T,
}

impl<T> NoteField<T> {
    pub fn new(present: bool, value: T) -> Self {
        Self { present, value }
    }
}

#[derive(Debug, Clone)]
pub struct ParsedNoteFinding {
    pub id: Option<String>,
    pub title: String,
    pub severity: NoteField<Option<Severity>>,
    pub status: NoteField<FindingStatus>,
    pub attack_scenario: NoteField<Option<String>>,
    pub preconditions: NoteField<Option<String>>,
    pub reachability: NoteField<Option<Reachability>>,
    pub impact: NoteField<Option<String>>,
    pub confidence: NoteField<Option<Confidence>>,
    pub cwe_id: NoteField<Option<String>>,
    pub cvss_score: NoteField<Option<f64>>,
    pub affected_assets: NoteField<Vec<String>>,
    pub taint_path: NoteField<Option<String>>,
    pub notes: NoteField<Option<String>>,
    pub fp_reason: NoteField<Option<String>>,
    pub source_file: NoteField<Option<String>>,
}

impl ParsedNoteFinding {
    pub fn to_finding(&self, project_id: &str, id: String) -> Finding {
        let mut finding = Finding::new(&id, project_id, &self.title);

        if let Some(sev) = self.severity.value {
            finding = finding.with_severity(sev);
        }
        if self.status.present {
            finding = finding.with_status(self.status.value);
        }
        if let Some(ref scenario) = self.attack_scenario.value {
            finding = finding.with_attack_scenario(scenario);
        }
        if let Some(ref preconditions) = self.preconditions.value {
            finding = finding.with_preconditions(preconditions);
        }
        if let Some(reachability) = self.reachability.value {
            finding = finding.with_reachability(reachability);
        }
        if let Some(ref impact) = self.impact.value {
            finding = finding.with_impact(impact);
        }
        if let Some(confidence) = self.confidence.value {
            finding = finding.with_confidence(confidence);
        }
        if let Some(ref cwe) = self.cwe_id.value {
            finding = finding.with_cwe(cwe);
        }
        if let Some(score) = self.cvss_score.value {
            finding.cvss_score = Some(score);
        }
        finding.affected_assets = self.affected_assets.value.clone();
        if let Some(ref taint) = self.taint_path.value {
            finding = finding.with_taint_path(taint);
        }
        finding.notes = self.notes.value.clone();
        finding.fp_reason = self.fp_reason.value.clone();
        if let Some(ref src) = self.source_file.value {
            finding.source_file = Some(src.clone());
        }

        finding
    }
}

pub fn default_note_rel_path(finding_id: &str) -> PathBuf {
    PathBuf::from("notes")
        .join("findings")
        .join(format!("{}.md", finding_id))
}

pub fn render_note_markdown(finding: &Finding) -> String {
    let mut s = String::new();

    s.push_str(&format!("# {}: {}\n\n", finding.id, finding.title));

    s.push_str(&format!(
        "**Severity:** {}  \n",
        finding.severity.map(|s| s.as_str()).unwrap_or("-")
    ));
    s.push_str(&format!("**Status:** {}  \n", finding.status.as_str()));

    s.push_str(&format!(
        "**Confidence:** {}  \n",
        finding.confidence.map(|c| c.as_str()).unwrap_or("-")
    ));
    s.push_str(&format!(
        "**Reachability:** {}  \n",
        finding.reachability.map(|r| r.as_str()).unwrap_or("-")
    ));
    s.push_str(&format!("**CWE:** {}  \n", finding.cwe_id.as_deref().unwrap_or("-")));
    s.push_str(&format!(
        "**CVSS:** {}  \n",
        finding
            .cvss_score
            .map(|s| format!("{:.1}", s))
            .unwrap_or_else(|| "-".to_string())
    ));
    s.push_str(&format!(
        "**FP Reason:** {}  \n",
        finding.fp_reason.as_deref().unwrap_or("-")
    ));

    s.push_str("\n## Attack Scenario\n\n");
    if let Some(ref scenario) = finding.attack_scenario {
        s.push_str(scenario.trim());
    }
    s.push_str("\n\n## Preconditions\n\n");
    if let Some(ref preconditions) = finding.preconditions {
        s.push_str(preconditions.trim());
    }
    s.push_str("\n\n## Impact\n\n");
    if let Some(ref impact) = finding.impact {
        s.push_str(impact.trim());
    }
    s.push_str("\n\n");

    s.push_str("## Affected Assets\n\n");
    if !finding.affected_assets.is_empty() {
        for asset in &finding.affected_assets {
            s.push_str(&format!("- {}\n", asset));
        }
    }
    s.push_str("\n");

    s.push_str("## Flow\n\n");
    if let Some(ref taint) = finding.taint_path {
        let taint = taint.trim();
        if !taint.is_empty() {
            s.push_str("```text\n");
            s.push_str(taint);
            s.push_str("\n```\n");
        }
    }
    s.push_str("\n");

    s.push_str("## Notes\n\n");
    if let Some(ref notes) = finding.notes {
        s.push_str(notes.trim());
    }
    s.push_str("\n");

    s
}

pub fn discover_note_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let dir = project_root.join("notes").join("findings");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("Failed to read {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn parse_note_finding(
    project_id: &str,
    project_root: &Path,
    path: &Path,
    content: &str,
) -> Result<ParsedNoteFinding> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "finding".to_string());

    let (heading_id, heading_title) = parse_heading_id_and_title(content);
    let mut id = heading_id.or_else(|| id_from_stem(&stem));
    id = id.map(|raw| normalize_id(project_id, &raw));

    let title = heading_title.unwrap_or_else(|| stem.clone());

    let (sev_inline_present, sev_inline_value) = extract_inline_value_present(content, "**Severity:**");
    let (sev_kv_present, sev_kv_value) = extract_key_value_present(content, "severity");
    let severity_present = sev_inline_present || sev_kv_present;
    let severity_value = sev_inline_value
        .or(sev_kv_value)
        .as_deref()
        .and_then(Severity::from_str);

    let (status_inline_present, status_inline_value) = extract_inline_value_present(content, "**Status:**");
    let (status_kv_present, status_kv_value) = extract_key_value_present(content, "status");
    let status_present = status_inline_present || status_kv_present;
    let status_value = status_inline_value
        .or(status_kv_value)
        .as_deref()
        .and_then(FindingStatus::from_str)
        .unwrap_or(FindingStatus::Raw);

    let (cwe_inline_present, cwe_inline_value) = extract_inline_value_present(content, "**CWE:**");
    let (cwe_kv_present, cwe_kv_value) = extract_key_value_present(content, "cwe");
    let cwe_present = cwe_inline_present || cwe_kv_present;
    let cwe_id = cwe_inline_value.or(cwe_kv_value).and_then(|s| normalize_placeholder(&s)).map(|s| {
        if s.to_uppercase().starts_with("CWE-") {
            s
        } else if s.chars().all(|c| c.is_ascii_digit()) {
            format!("CWE-{}", s)
        } else {
            s
        }
    });

    let (attack_present, attack_value) = extract_section_present(content, "Attack Scenario");
    let (pre_present, pre_value) = extract_section_present(content, "Preconditions");
    let (impact_present, impact_value) = extract_section_present(content, "Impact");

    let (reach_inline_present, reach_inline_value) =
        extract_inline_value_present(content, "**Reachability:**");
    let (reach_kv_present, reach_kv_value) = extract_key_value_present(content, "reachability");
    let reachability_present = reach_inline_present || reach_kv_present;
    let reachability_value = reach_inline_value
        .or(reach_kv_value)
        .as_deref()
        .and_then(Reachability::from_str);

    let (conf_inline_present, conf_inline_value) =
        extract_inline_value_present(content, "**Confidence:**");
    let (conf_kv_present, conf_kv_value) = extract_key_value_present(content, "confidence");
    let confidence_present = conf_inline_present || conf_kv_present;
    let confidence_value = conf_inline_value
        .or(conf_kv_value)
        .as_deref()
        .and_then(Confidence::from_str);

    let (assets_present, affected_assets) = extract_list_section_present(content, "Affected Assets");
    let (flow_present, taint_path) = extract_code_or_text_section_present(content, "Flow");
    let (notes_present, notes_value) = extract_section_present(content, "Notes");

    let (fp_inline_present, fp_inline_value) = extract_inline_value_present(content, "**FP Reason:**");
    let (fp_kv_present, fp_kv_value) = extract_key_value_present(content, "fp_reason");
    let (fp_section_present, fp_section_value) = extract_section_present(content, "FP Reason");
    let fp_present = fp_inline_present || fp_kv_present || fp_section_present;
    let fp_reason = fp_inline_value.or(fp_kv_value).or(fp_section_value);

    let (cvss_inline_present, cvss_inline_value) = extract_inline_value_present(content, "**CVSS:**");
    let (cvss_kv_present, cvss_kv_value) = extract_key_value_present(content, "cvss");
    let cvss_present = cvss_inline_present || cvss_kv_present;
    let cvss_score = cvss_inline_value
        .or(cvss_kv_value)
        .and_then(|s| normalize_placeholder(&s))
        .and_then(|s| s.parse::<f64>().ok());

    let source_file = path
        .canonicalize()
        .ok()
        .and_then(|p| project_root.canonicalize().ok().and_then(|root| p.strip_prefix(root).ok().map(|p| p.to_string_lossy().to_string())))
        .or_else(|| path.strip_prefix(project_root).ok().map(|p| p.to_string_lossy().to_string()));

    Ok(ParsedNoteFinding {
        id,
        title,
        severity: NoteField::new(severity_present, severity_value),
        status: NoteField::new(status_present, status_value),
        attack_scenario: NoteField::new(attack_present, attack_value),
        preconditions: NoteField::new(pre_present, pre_value),
        reachability: NoteField::new(reachability_present, reachability_value),
        impact: NoteField::new(impact_present, impact_value),
        confidence: NoteField::new(confidence_present, confidence_value),
        cwe_id: NoteField::new(cwe_present, cwe_id),
        cvss_score: NoteField::new(cvss_present, cvss_score),
        affected_assets: NoteField::new(assets_present, affected_assets),
        taint_path: NoteField::new(flow_present, taint_path),
        notes: NoteField::new(notes_present, notes_value),
        fp_reason: NoteField::new(fp_present, fp_reason),
        source_file: NoteField::new(true, source_file),
    })
}

fn normalize_id(project_id: &str, raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return raw.to_string();
    }

    if raw.starts_with(&format!("{project_id}-")) {
        return raw.to_string();
    }

    if raw.to_uppercase().starts_with("VULN-") {
        return format!("{project_id}-{raw}");
    }

    raw.to_string()
}

fn parse_heading_id_and_title(content: &str) -> (Option<String>, Option<String>) {
    let heading = content
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with('#'))
        .map(|l| l.trim_start_matches('#').trim().to_string());

    let Some(heading) = heading else {
        return (None, None);
    };

    if let Some((left, right)) = heading.split_once(':') {
        let id = left.trim();
        let title = right.trim();
        let id_ok = id.to_uppercase().contains("VULN-");
        let id = if id_ok { Some(id.to_string()) } else { None };
        let title = if title.is_empty() { None } else { Some(title.to_string()) };
        return (id, title);
    }

    (None, Some(heading))
}

fn id_from_stem(stem: &str) -> Option<String> {
    let stem_trim = stem.trim();
    if stem_trim.is_empty() {
        return None;
    }
    if stem_trim.to_uppercase().contains("VULN-") {
        return Some(stem_trim.to_string());
    }
    None
}

fn extract_inline_value(content: &str, prefix: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(prefix) {
            let value = rest.trim().trim_end_matches("  ").trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn extract_inline_value_present(content: &str, prefix: &str) -> (bool, Option<String>) {
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(prefix) {
            let value = rest.trim().trim_end_matches("  ").trim();
            return (true, normalize_placeholder(value));
        }
    }
    (false, None)
}

fn extract_key_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        let Some((k, v)) = line.split_once(':') else { continue };
        if k.trim().eq_ignore_ascii_case(key) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn extract_key_value_present(content: &str, key: &str) -> (bool, Option<String>) {
    for line in content.lines() {
        let line = line.trim();
        let Some((k, v)) = line.split_once(':') else { continue };
        if k.trim().eq_ignore_ascii_case(key) {
            let v = v.trim();
            return (true, normalize_placeholder(v));
        }
    }
    (false, None)
}

fn extract_section(content: &str, heading: &str) -> Option<String> {
    let start = format!("## {}", heading);
    let mut in_section = false;
    let mut buf = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().eq_ignore_ascii_case(&start) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.trim_start().starts_with("## ") {
            break;
        }
        if in_section {
            buf.push(trimmed);
        }
    }

    let text = buf.join("\n").trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_section_present(content: &str, heading: &str) -> (bool, Option<String>) {
    let start = format!("## {}", heading);
    let mut found_heading = false;
    let mut in_section = false;
    let mut buf = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().eq_ignore_ascii_case(&start) {
            found_heading = true;
            in_section = true;
            continue;
        }
        if in_section && trimmed.trim_start().starts_with("## ") {
            break;
        }
        if in_section {
            buf.push(trimmed);
        }
    }

    let text = buf.join("\n").trim().to_string();
    if text.is_empty() {
        (found_heading, None)
    } else {
        (found_heading, Some(text))
    }
}

fn extract_list_section(content: &str, heading: &str) -> Vec<String> {
    let Some(section) = extract_section(content, heading) else {
        return Vec::new();
    };

    section
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            let item = t
                .strip_prefix("- ")
                .or_else(|| t.strip_prefix("* "))
                .map(str::trim)?;
            if item.is_empty() { None } else { Some(item.to_string()) }
        })
        .collect()
}

fn extract_code_or_text_section(content: &str, heading: &str) -> Option<String> {
    let section = extract_section(content, heading)?;
    let mut in_code = false;
    let mut code = Vec::new();
    for line in section.lines() {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            code.push(t);
        }
    }

    let code = code.join("\n").trim().to_string();
    if !code.is_empty() {
        return Some(code);
    }

    let text = section.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn extract_list_section_present(content: &str, heading: &str) -> (bool, Vec<String>) {
    let (present, section) = extract_section_present(content, heading);
    let Some(section) = section else {
        return (present, Vec::new());
    };

    let items = section
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            let item = t
                .strip_prefix("- ")
                .or_else(|| t.strip_prefix("* "))
                .map(str::trim)?;
            if item.is_empty() { None } else { Some(item.to_string()) }
        })
        .collect();

    (present, items)
}

fn extract_code_or_text_section_present(content: &str, heading: &str) -> (bool, Option<String>) {
    let (present, section) = extract_section_present(content, heading);
    let Some(section) = section else {
        return (present, None);
    };
    let mut in_code = false;
    let mut code = Vec::new();
    for line in section.lines() {
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            code.push(t);
        }
    }

    let code = code.join("\n").trim().to_string();
    if !code.is_empty() {
        return (present, Some(code));
    }

    let text = section.trim().to_string();
    if text.is_empty() {
        (present, None)
    } else {
        (present, Some(text))
    }
}

fn normalize_placeholder(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if trimmed == "-" || trimmed == "—" || lower == "n/a" || lower == "na" {
        return None;
    }
    if lower == "(not specified)" || lower == "(unspecified)" {
        return None;
    }
    Some(trimmed.to_string())
}
