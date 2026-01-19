//! Skill Registry - Search and install skills from the community registry
//!
//! The registry contains ~50,000 skills scraped from GitHub repositories
//! that follow the agentskills.io SKILL.md specification.
//!
//! Skills can be searched by name, description, or author and installed
//! directly into `.claude/skills/` or `.codex/skills/` directories.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Embedded registry data (gzip compressed at compile time)
const REGISTRY_DATA_GZ: &[u8] = include_bytes!("../../assets/registry/skills.json.gz");

/// A skill entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkill {
    /// Unique identifier (e.g., "author-repo-skillname-skill-md")
    pub id: String,
    /// Skill name (e.g., "deep-research")
    pub name: String,
    /// GitHub author/owner
    pub author: String,
    /// Author's avatar URL
    #[serde(default)]
    pub author_avatar: String,
    /// Skill description
    #[serde(default)]
    pub description: String,
    /// Full GitHub URL to the skill directory
    pub github_url: String,
    /// Repository stars
    #[serde(default)]
    pub stars: u32,
    /// Repository forks
    #[serde(default)]
    pub forks: u32,
    /// Last updated timestamp (Unix epoch)
    #[serde(default)]
    pub updated_at: u64,
    /// Whether it's listed on a marketplace
    #[serde(default)]
    pub has_marketplace: bool,
    /// Path to SKILL.md within the skill directory
    #[serde(default)]
    pub path: String,
    /// Git branch
    #[serde(default)]
    pub branch: String,
}

impl RegistrySkill {
    /// Get the raw GitHub URL for downloading the SKILL.md content
    pub fn raw_skill_url(&self) -> Option<String> {
        // Convert:
        // - https://github.com/author/repo/tree/branch/path/to/skill
        // - https://github.com/author/repo
        // To:
        // - https://raw.githubusercontent.com/author/repo/branch/path/to/skill/SKILL.md
        // - https://raw.githubusercontent.com/author/repo/branch/SKILL.md
        let url = self.github_url.trim_end_matches('/');
        if !url.starts_with("https://github.com/") {
            return None;
        }

        // Parse the GitHub URL
        let path = url.strip_prefix("https://github.com/")?;
        let parts: Vec<&str> = path.split('/').collect();

        if parts.len() < 2 {
            return None;
        }

        let owner = parts[0];
        let repo = parts[1];
        let is_tree_url = parts.len() >= 4 && parts[2] == "tree";
        let branch: &str = if is_tree_url {
            parts[3]
        } else {
            self.branch.as_str()
        };

        let skill_dir_path = if is_tree_url && parts.len() > 4 {
            parts[4..].join("/")
        } else {
            String::new()
        };

        let skill_md_path = if skill_dir_path.is_empty() {
            self.path.clone()
        } else if self.path.is_empty() || self.path == "SKILL.md" {
            format!("{}/SKILL.md", skill_dir_path)
        } else {
            format!("{}/{}", skill_dir_path, self.path)
        };

        Some(format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            owner,
            repo,
            branch,
            skill_md_path.trim_start_matches('/')
        ))
    }

    /// Format for display in search results
    pub fn display_line(&self) -> String {
        let stars = if self.stars > 0 {
            format!(" ★{}", self.stars)
        } else {
            String::new()
        };
        format!(
            "{}/{} - {}{}",
            self.author,
            self.name,
            truncate_description(&self.description, 60),
            stars
        )
    }
}

/// The full registry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRegistry {
    pub total: usize,
    #[serde(default)]
    pub expected: usize,
    #[serde(default)]
    pub progress_pct: f64,
    #[serde(default)]
    pub dumped_at: String,
    pub skills: Vec<RegistrySkill>,
}

impl SkillRegistry {
    /// Load the embedded registry (decompresses gzip data)
    pub fn load_embedded() -> Result<Self> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(REGISTRY_DATA_GZ);
        let mut json_data = String::new();
        decoder.read_to_string(&mut json_data)?;

        let registry: SkillRegistry = serde_json::from_str(&json_data)?;
        Ok(registry)
    }

    /// Load from a local JSON file (for updates or custom registries)
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let registry: SkillRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }

    /// Search skills by query with fuzzy matching
    ///
    /// Uses a combination of:
    /// - Exact matching (highest priority)
    /// - Substring matching
    /// - Jaro-Winkler fuzzy similarity (for typos)
    ///
    /// Results are scored and sorted by relevance.
    pub fn search(&self, query: &str, limit: usize) -> Vec<&RegistrySkill> {
        use strsim::jaro_winkler;

        let query_lower = query.to_lowercase();
        let query_parts: Vec<&str> = query_lower.split_whitespace().collect();

        // Minimum fuzzy similarity threshold (0.0 - 1.0)
        const FUZZY_THRESHOLD: f64 = 0.75;

        let mut results: Vec<(&RegistrySkill, u32)> = self
            .skills
            .iter()
            .filter_map(|skill| {
                let name_lower = skill.name.to_lowercase();
                let desc_lower = skill.description.to_lowercase();
                let author_lower = skill.author.to_lowercase();

                // Calculate relevance score (0-1000+ scale)
                let mut score: u32 = 0;
                let mut matched = false;

                // === Exact Matching (highest priority) ===

                // Exact name match
                if name_lower == query_lower {
                    score += 1000;
                    matched = true;
                }
                // Exact author/name match
                else if format!("{}/{}", author_lower, name_lower) == query_lower {
                    score += 950;
                    matched = true;
                }
                // Name starts with query
                else if name_lower.starts_with(&query_lower) {
                    score += 500;
                    matched = true;
                }
                // Name contains query
                else if name_lower.contains(&query_lower) {
                    score += 200;
                    matched = true;
                }

                // === Substring Matching ===

                // All query parts found in name/desc/author
                let all_parts_match = query_parts.iter().all(|part| {
                    name_lower.contains(part)
                        || desc_lower.contains(part)
                        || author_lower.contains(part)
                });

                if all_parts_match {
                    score += 100;
                    matched = true;
                }

                // Description contains full query
                if desc_lower.contains(&query_lower) {
                    score += 50;
                    matched = true;
                }

                // Author match
                if author_lower.contains(&query_lower) {
                    score += 30;
                    matched = true;
                }

                // === Fuzzy Matching (for typos) ===

                if !matched {
                    // Fuzzy match on skill name
                    let name_similarity = jaro_winkler(&name_lower, &query_lower);
                    if name_similarity >= FUZZY_THRESHOLD {
                        score += (name_similarity * 150.0) as u32;
                        matched = true;
                    }

                    // Fuzzy match on name parts (e.g., "code-review" matches "codereview")
                    let name_parts: Vec<&str> = name_lower.split('-').collect();
                    for part in &name_parts {
                        let part_similarity = jaro_winkler(part, &query_lower);
                        if part_similarity >= FUZZY_THRESHOLD {
                            score += (part_similarity * 80.0) as u32;
                            matched = true;
                        }
                    }

                    // Fuzzy match each query word against name parts
                    for query_part in &query_parts {
                        for name_part in &name_parts {
                            let similarity = jaro_winkler(name_part, query_part);
                            if similarity >= FUZZY_THRESHOLD {
                                score += (similarity * 60.0) as u32;
                                matched = true;
                            }
                        }
                    }

                    // Fuzzy match on author
                    let author_similarity = jaro_winkler(&author_lower, &query_lower);
                    if author_similarity >= FUZZY_THRESHOLD {
                        score += (author_similarity * 40.0) as u32;
                        matched = true;
                    }

                    // Fuzzy match on description words (for semantic-ish matching)
                    // Only check longer words (>3 chars) to avoid noise
                    let desc_words: Vec<&str> = desc_lower
                        .split(|c: char| !c.is_alphanumeric())
                        .filter(|w| w.len() > 3)
                        .collect();

                    for query_part in &query_parts {
                        if query_part.len() < 3 {
                            continue;
                        }
                        for desc_word in &desc_words {
                            let similarity = jaro_winkler(desc_word, query_part);
                            if similarity >= 0.85 {
                                // Higher threshold for descriptions
                                score += (similarity * 30.0) as u32;
                                matched = true;
                                break; // One match per query part is enough
                            }
                        }
                    }
                }

                // Skip if no match at all
                if !matched {
                    return None;
                }

                // === Bonus Points ===

                // Stars bonus (logarithmic, capped)
                if skill.stars > 0 {
                    let star_bonus = ((skill.stars as f64).log2() * 3.0).min(30.0) as u32;
                    score += star_bonus;
                }

                // Marketplace listing bonus
                if skill.has_marketplace {
                    score += 15;
                }

                Some((skill, score))
            })
            .collect();

        // Sort by score descending, then by stars as tiebreaker
        results.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.0.stars.cmp(&a.0.stars))
        });

        results.into_iter().take(limit).map(|(s, _)| s).collect()
    }

    /// Get a skill by exact name (returns first match)
    pub fn get_by_name(&self, name: &str) -> Option<&RegistrySkill> {
        let name_lower = name.to_lowercase();
        self.skills
            .iter()
            .find(|s| s.name.to_lowercase() == name_lower)
    }

    /// Get a skill by author/name format
    pub fn get_by_full_name(&self, full_name: &str) -> Option<&RegistrySkill> {
        let parts: Vec<&str> = full_name.split('/').collect();
        if parts.len() != 2 {
            return None;
        }
        let (author, name) = (parts[0].to_lowercase(), parts[1].to_lowercase());
        self.skills
            .iter()
            .find(|s| s.author.to_lowercase() == author && s.name.to_lowercase() == name)
    }
}

/// Truncate description for display
fn truncate_description(desc: &str, max_len: usize) -> String {
    // Take first line only
    let first_line = desc.lines().next().unwrap_or(desc);
    truncate_chars(first_line, max_len)
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }

    if max_chars <= 3 {
        return s.chars().take(max_chars).collect();
    }

    let truncated: String = s.chars().take(max_chars - 3).collect();
    format!("{}...", truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded_registry() {
        let registry = SkillRegistry::load_embedded().expect("Failed to load registry");
        assert!(registry.total > 0);
        assert!(!registry.skills.is_empty());
    }

    #[test]
    fn test_search_skills() {
        let registry = SkillRegistry::load_embedded().expect("Failed to load registry");
        let results = registry.search("review", 10);
        assert!(!results.is_empty());
        // Should find skills with "review" in name or description
    }

    #[test]
    fn test_raw_skill_url() {
        let skill = RegistrySkill {
            id: "test".to_string(),
            name: "test-skill".to_string(),
            author: "testauthor".to_string(),
            author_avatar: String::new(),
            description: "Test".to_string(),
            github_url: "https://github.com/testauthor/repo/tree/main/skills/test-skill"
                .to_string(),
            stars: 0,
            forks: 0,
            updated_at: 0,
            has_marketplace: false,
            path: "SKILL.md".to_string(),
            branch: "main".to_string(),
        };

        let url = skill.raw_skill_url().unwrap();
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/testauthor/repo/main/skills/test-skill/SKILL.md"
        );
    }

    #[test]
    fn test_raw_skill_url_repo_root() {
        let skill = RegistrySkill {
            id: "test".to_string(),
            name: "test-skill".to_string(),
            author: "testauthor".to_string(),
            author_avatar: String::new(),
            description: "Test".to_string(),
            github_url: "https://github.com/testauthor/repo".to_string(),
            stars: 0,
            forks: 0,
            updated_at: 0,
            has_marketplace: false,
            path: "SKILL.md".to_string(),
            branch: "main".to_string(),
        };

        let url = skill.raw_skill_url().unwrap();
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/testauthor/repo/main/SKILL.md"
        );
    }

    #[test]
    fn test_fuzzy_search() {
        let registry = SkillRegistry::load_embedded().expect("Failed to load registry");

        // Typo: "reveiw" instead of "review"
        let results = registry.search("reveiw", 10);
        assert!(!results.is_empty(), "Fuzzy search should find 'review' skills with typo 'reveiw'");

        // Partial match: "refact" should find "refactor"
        let results = registry.search("refact", 10);
        assert!(!results.is_empty(), "Should find refactor skills");

        // Multi-word query
        let results = registry.search("code quality", 10);
        assert!(!results.is_empty(), "Should find skills about code quality");
    }
}
