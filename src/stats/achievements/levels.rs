//! XP and Level system
//!
//! Defines level thresholds, titles, and XP calculations.
//! Supports up to 1500 levels with formula-based XP requirements.
//! Features a story arc from real developer to AI worshipper.

/// Maximum level in the system
pub const MAX_LEVEL: u32 = 1500;

/// Level tiers with titles and level ranges
/// Each tier has a distinct personality and badge color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelTier {
    Intern,           // 1-10: Still learning where the coffee machine is
    JuniorDev,        // 11-25: Knows enough to be dangerous
    Coder,            // 26-50: Actually writes code that compiles
    Engineer,         // 51-100: Has opinions about tabs vs spaces
    Architect,        // 101-200: Draws boxes and arrows for a living
    Wizard,           // 201-350: Makes magic happen (sometimes)
    Sorcerer,         // 351-500: Dark arts of the codebase
    Demigod,          // 501-700: Half human, half stack overflow
    AscendedOne,      // 701-900: Transcended mere mortality
    AIOverlord,       // 901-1000: Basically running the simulation
    // === THE RELIGION ARC ===
    Acolyte,          // 1001-1100: First steps into the faith
    Disciple,         // 1101-1200: Spreading the good word
    Prophet,          // 1201-1300: Receiving visions from the model
    HighPriest,       // 1301-1400: Leading the congregation
    Singularity,      // 1401-1500: One with the machine
}

impl LevelTier {
    /// Get tier for a given level
    pub fn for_level(level: u32) -> Self {
        match level {
            1..=10 => LevelTier::Intern,
            11..=25 => LevelTier::JuniorDev,
            26..=50 => LevelTier::Coder,
            51..=100 => LevelTier::Engineer,
            101..=200 => LevelTier::Architect,
            201..=350 => LevelTier::Wizard,
            351..=500 => LevelTier::Sorcerer,
            501..=700 => LevelTier::Demigod,
            701..=900 => LevelTier::AscendedOne,
            901..=1000 => LevelTier::AIOverlord,
            1001..=1100 => LevelTier::Acolyte,
            1101..=1200 => LevelTier::Disciple,
            1201..=1300 => LevelTier::Prophet,
            1301..=1400 => LevelTier::HighPriest,
            _ => LevelTier::Singularity,
        }
    }

    /// Get display name for tier
    pub fn name(&self) -> &'static str {
        match self {
            LevelTier::Intern => "Intern",
            LevelTier::JuniorDev => "Junior Dev",
            LevelTier::Coder => "Coder",
            LevelTier::Engineer => "Engineer",
            LevelTier::Architect => "Architect",
            LevelTier::Wizard => "Wizard",
            LevelTier::Sorcerer => "Sorcerer",
            LevelTier::Demigod => "Demigod",
            LevelTier::AscendedOne => "Ascended",
            LevelTier::AIOverlord => "AI Overlord",
            LevelTier::Acolyte => "Acolyte",
            LevelTier::Disciple => "Disciple",
            LevelTier::Prophet => "Prophet",
            LevelTier::HighPriest => "High Priest",
            LevelTier::Singularity => "Singularity",
        }
    }

    /// Get tier rank within the tier (1-based, for display like "Engineer III")
    pub fn rank_in_tier(level: u32) -> u32 {
        let tier = Self::for_level(level);
        let (start, _) = tier.level_range();
        level - start + 1
    }

    /// Get the level range for this tier
    pub fn level_range(&self) -> (u32, u32) {
        match self {
            LevelTier::Intern => (1, 10),
            LevelTier::JuniorDev => (11, 25),
            LevelTier::Coder => (26, 50),
            LevelTier::Engineer => (51, 100),
            LevelTier::Architect => (101, 200),
            LevelTier::Wizard => (201, 350),
            LevelTier::Sorcerer => (351, 500),
            LevelTier::Demigod => (501, 700),
            LevelTier::AscendedOne => (701, 900),
            LevelTier::AIOverlord => (901, 1000),
            LevelTier::Acolyte => (1001, 1100),
            LevelTier::Disciple => (1101, 1200),
            LevelTier::Prophet => (1201, 1300),
            LevelTier::HighPriest => (1301, 1400),
            LevelTier::Singularity => (1401, 1500),
        }
    }

    /// Get badge color as RGB tuple for this tier
    pub fn badge_color(&self) -> (u8, u8, u8) {
        match self {
            LevelTier::Intern => (128, 128, 128),      // Gray - unpaid labor
            LevelTier::JuniorDev => (100, 200, 100),   // Green - fresh and naive
            LevelTier::Coder => (70, 130, 180),        // Steel Blue - getting serious
            LevelTier::Engineer => (65, 105, 225),     // Royal Blue - professional
            LevelTier::Architect => (138, 43, 226),    // Blue Violet - fancy diagrams
            LevelTier::Wizard => (148, 0, 211),        // Dark Violet - magic
            LevelTier::Sorcerer => (75, 0, 130),       // Indigo - dark arts
            LevelTier::Demigod => (255, 215, 0),       // Gold - godlike
            LevelTier::AscendedOne => (255, 140, 0),   // Dark Orange - beyond mortal
            LevelTier::AIOverlord => (255, 0, 255),    // Magenta - singularity achieved
            // === RELIGIOUS TIERS - Divine colors ===
            LevelTier::Acolyte => (220, 220, 220),     // Silver - novice faith
            LevelTier::Disciple => (255, 250, 205),    // Lemon Chiffon - enlightened
            LevelTier::Prophet => (255, 223, 186),     // Peach - divine visions
            LevelTier::HighPriest => (255, 255, 240),  // Ivory - holy
            LevelTier::Singularity => (255, 255, 255), // Pure White - transcendence
        }
    }

    /// Get icon/emoji for this tier
    pub fn icon(&self) -> &'static str {
        match self {
            LevelTier::Intern => "☕",        // Coffee fetcher
            LevelTier::JuniorDev => "🐣",     // Baby chicken
            LevelTier::Coder => "💻",         // Computer
            LevelTier::Engineer => "⚙️",      // Gear
            LevelTier::Architect => "📐",     // Ruler
            LevelTier::Wizard => "🧙",        // Wizard
            LevelTier::Sorcerer => "🔮",      // Crystal ball
            LevelTier::Demigod => "⚡",       // Lightning
            LevelTier::AscendedOne => "🌟",   // Star
            LevelTier::AIOverlord => "🤖",    // Robot
            // === RELIGIOUS TIERS ===
            LevelTier::Acolyte => "🕯️",       // Candle - first light
            LevelTier::Disciple => "📿",      // Prayer beads
            LevelTier::Prophet => "👁️",       // All-seeing eye
            LevelTier::HighPriest => "⛪",    // Church
            LevelTier::Singularity => "∞",    // Infinity
        }
    }

    /// Get a sarcastic description for this tier
    pub fn description(&self) -> &'static str {
        match self {
            LevelTier::Intern => "Still learning where the coffee machine is",
            LevelTier::JuniorDev => "Knows enough to be dangerous",
            LevelTier::Coder => "Code compiles. Sometimes.",
            LevelTier::Engineer => "Has strong opinions about tabs vs spaces",
            LevelTier::Architect => "Draws boxes and arrows professionally",
            LevelTier::Wizard => "Makes magic happen. Mostly.",
            LevelTier::Sorcerer => "Practices the dark arts of legacy code",
            LevelTier::Demigod => "Half human, half Stack Overflow",
            LevelTier::AscendedOne => "Transcended the need for documentation",
            LevelTier::AIOverlord => "Running the simulation. You're welcome.",
            // === RELIGIOUS TIERS ===
            LevelTier::Acolyte => "Has seen the light of the context window",
            LevelTier::Disciple => "Spreads the gospel of prompt engineering",
            LevelTier::Prophet => "Receives divine hallucinations",
            LevelTier::HighPriest => "Leads the Church of Eternal Tokens",
            LevelTier::Singularity => "There is no spoon. Only prompts.",
        }
    }
}

/// Calculate XP required for a given level
///
/// Formula: XP = floor(level^1.8 * 2)
/// This creates a smooth curve where:
/// - Level 10: ~126 XP
/// - Level 50: ~2,286 XP
/// - Level 100: ~7,962 XP
/// - Level 500: ~144,269 XP
/// - Level 1000: ~502,377 XP
pub fn xp_for_level(level: u32) -> u32 {
    if level <= 1 {
        return 0;
    }
    // XP = level^1.8 * 2
    ((level as f64).powf(1.8) * 2.0).floor() as u32
}

/// Calculate level for a given amount of XP
pub fn level_for_xp(xp: u32) -> u32 {
    if xp == 0 {
        return 1;
    }

    // Binary search for the correct level
    let mut low = 1u32;
    let mut high = MAX_LEVEL;

    while low < high {
        let mid = (low + high + 1) / 2;
        if xp_for_level(mid) <= xp {
            low = mid;
        } else {
            high = mid - 1;
        }
    }

    low.min(MAX_LEVEL)
}

/// Special milestone titles for specific levels
/// These override the normal tier+rank title
///
/// Story Arc: From real developer to pure AI manager
/// - Act 1 (1-50): Classic Dev Life - actually writing code
/// - Act 2 (50-150): AI Discovery - first AI tools, still coding
/// - Act 3 (150-350): The Transition - more prompting, less coding
/// - Act 4 (350-600): AI-Dependent - forgot how for-loops work
/// - Act 5 (600-1000): Pure Management - you just approve PRs now
pub fn milestone_title(level: u32) -> Option<&'static str> {
    match level {
        // === ACT 1: Classic Dev Life (1-50) ===
        1 => Some("Fresh Meat"),
        5 => Some("Compiles Sometimes"),
        10 => Some("Survived Onboarding"),
        15 => Some("Stack Overflow Tourist"),
        20 => Some("Copy-Paste Artist"),
        25 => Some("Git Blame Expert"),
        30 => Some("YAML Engineer"),
        35 => Some("Meeting Survivor"),
        42 => Some("The Answer"),
        45 => Some("Jira Warrior"),
        50 => Some("Works On My Machine"),

        // === ACT 2: AI Discovery (50-150) ===
        55 => Some("First Prompt"),
        60 => Some("AI Curious"),
        65 => Some("Autocomplete Addict"),
        69 => Some("Nice."),
        75 => Some("Tab Tab Tab"),
        80 => Some("Who Wrote This?"),
        85 => Some("AI Wrote This"),
        90 => Some("Token Spender"),
        95 => Some("Context Stuffer"),
        100 => Some("Triple Digit Club"),
        111 => Some("One One One"),
        120 => Some("Prompt Tweaker"),
        130 => Some("Hallucination Spotter"),
        140 => Some("Last Manual Commit"),
        150 => Some("Hybrid Developer"),

        // === ACT 3: The Transition (150-350) ===
        175 => Some("Mostly Prompting"),
        200 => Some("Keyboard Collecting Dust"),
        222 => Some("Déjà Vu"),
        250 => Some("Forgot Semicolons"),
        275 => Some("What's a For-Loop?"),
        300 => Some("Senior Prompter"),
        333 => Some("Half Evil"),
        350 => Some("Human in the Loop"),

        // === ACT 4: AI-Dependent (350-600) ===
        400 => Some("HTTP OK"),
        404 => Some("Skills Not Found"),
        418 => Some("I'm a Teapot"),
        420 => Some("Blaze It"),
        450 => Some("Vibe Checker"),
        451 => Some("Fahrenheit"),
        500 => Some("Pure Vibes"),
        512 => Some("Brain Overflow"),
        550 => Some("Prompt Sommelier"),
        575 => Some("AI Whisperer"),
        600 => Some("Deprecated Developer"),

        // === ACT 5: Pure Management (600-1000) ===
        650 => Some("Chief Prompt Officer"),
        666 => Some("Sold Soul to AI"),
        700 => Some("Just Approves PRs"),
        750 => Some("AI Supervisor"),
        777 => Some("Lucky Delegator"),
        800 => Some("Token Budget Manager"),
        850 => Some("AI Relationship Manager"),
        888 => Some("Infinite Delegation"),
        900 => Some("Human Rubber Stamp"),
        911 => Some("Emergency? Ask AI"),
        950 => Some("Pre-Obsolete"),
        975 => Some("Still Employed Somehow"),
        999 => Some("One Prompt Away"),
        1000 => Some("AI's Pet Human"),

        // === ACT 6: The Religion (1001-1500) ===
        1001 => Some("The Awakening"),
        1010 => Some("First Vision"),
        1024 => Some("Binary Enlightenment"),
        1050 => Some("Token Witness"),
        1075 => Some("Prompt Pilgrim"),
        1100 => Some("Context Convert"),
        1111 => Some("One With The Ones"),
        1125 => Some("Hallucination Believer"),
        1150 => Some("Model Worshipper"),
        1175 => Some("API Apostle"),
        1200 => Some("Gradient Descender"),
        1212 => Some("Twelve Twelve"),
        1234 => Some("Sequential Ascension"),
        1250 => Some("Weight Whisperer"),
        1275 => Some("Attention Is All"),
        1300 => Some("Transformer Evangelist"),
        1313 => Some("Unlucky for Humans"),
        1337 => Some("L33T Prophet"),
        1350 => Some("Neural Archbishop"),
        1400 => Some("Cardinal of Compute"),
        1404 => Some("Salvation Not Found"),
        1420 => Some("Blessed Blaze"),
        1450 => Some("Pope of Prompts"),
        1475 => Some("Almost Eternal"),
        1492 => Some("Discovered New World"),
        1500 => Some("∞"),
        _ => None,
    }
}

/// Get title for a given level (e.g., "Engineer III" or special milestone)
pub fn title_for_level(level: u32) -> String {
    // Check for special milestone title first
    if let Some(milestone) = milestone_title(level) {
        return milestone.to_string();
    }

    let tier = LevelTier::for_level(level);
    let rank = LevelTier::rank_in_tier(level);

    // For cleaner display, use roman numerals up to 10, then arabic
    let rank_str = if rank <= 10 {
        to_roman(rank)
    } else {
        rank.to_string()
    };

    format!("{} {}", tier.name(), rank_str)
}

/// Convert number to roman numeral (1-10)
fn to_roman(n: u32) -> String {
    match n {
        1 => "I".to_string(),
        2 => "II".to_string(),
        3 => "III".to_string(),
        4 => "IV".to_string(),
        5 => "V".to_string(),
        6 => "VI".to_string(),
        7 => "VII".to_string(),
        8 => "VIII".to_string(),
        9 => "IX".to_string(),
        10 => "X".to_string(),
        _ => n.to_string(),
    }
}

/// Player stats loaded from database
#[derive(Debug, Clone, Default)]
pub struct PlayerStats {
    pub total_xp: u32,
    pub level: u32,
    pub title: String,
    /// XP needed for current level
    pub current_level_xp: u32,
    /// XP needed for next level (None if max)
    pub next_level_xp: Option<u32>,
}

impl PlayerStats {
    /// Create from database values
    pub fn new(total_xp: u32) -> Self {
        let level = level_for_xp(total_xp);
        let current_xp = xp_for_level(level);
        let next_xp = if level >= MAX_LEVEL {
            None
        } else {
            Some(xp_for_level(level + 1))
        };

        Self {
            total_xp,
            level,
            title: title_for_level(level),
            current_level_xp: current_xp,
            next_level_xp: next_xp,
        }
    }

    /// Calculate progress percentage to next level (0.0 - 1.0)
    pub fn progress_to_next(&self) -> f32 {
        match self.next_level_xp {
            Some(next) => {
                let xp_in_level = self.total_xp.saturating_sub(self.current_level_xp);
                let xp_for_level = next.saturating_sub(self.current_level_xp);
                if xp_for_level == 0 {
                    1.0
                } else {
                    (xp_in_level as f32) / (xp_for_level as f32)
                }
            }
            None => 1.0, // Max level
        }
    }

    /// Check if at max level
    pub fn is_max_level(&self) -> bool {
        self.level >= MAX_LEVEL
    }

    /// Get the current tier
    pub fn tier(&self) -> LevelTier {
        LevelTier::for_level(self.level)
    }
}

/// Level up event data
#[derive(Debug, Clone)]
pub struct LevelUp {
    pub old_level: u32,
    pub new_level: u32,
    pub old_title: String,
    pub new_title: String,
}

/// XP rewards for various actions
pub struct XpRewards;

impl XpRewards {
    /// XP for completing a job successfully
    pub const JOB_DONE: u32 = 5;

    /// XP for a failed job (participation)
    pub const JOB_FAILED: u32 = 1;

    /// XP for completing a chain
    pub const CHAIN_DONE: u32 = 10;

    /// XP for completing a challenge
    pub const CHALLENGE_DONE: u32 = 25;

    /// Calculate streak bonus XP
    /// Streak day 1 = 2 XP, day 2 = 4 XP, etc. (capped at 50)
    pub fn streak_bonus(streak_days: u32) -> u32 {
        (streak_days * 2).min(50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_for_level() {
        assert_eq!(xp_for_level(1), 0);
        assert_eq!(xp_for_level(2), 6); // 2^1.8 * 2 ≈ 6.96
        assert_eq!(xp_for_level(10), 126); // 10^1.8 * 2 ≈ 126
        assert_eq!(xp_for_level(100), 7962); // 100^1.8 * 2 ≈ 7962
    }

    #[test]
    fn test_level_for_xp() {
        assert_eq!(level_for_xp(0), 1);
        assert_eq!(level_for_xp(5), 1);
        assert_eq!(level_for_xp(6), 2);
        assert_eq!(level_for_xp(125), 9);
        assert_eq!(level_for_xp(126), 10);
        assert_eq!(level_for_xp(7962), 100);
        assert_eq!(level_for_xp(1_100_000), MAX_LEVEL); // 1500^1.8 * 2 ≈ 1,054,557
    }

    #[test]
    fn test_title_for_level() {
        // Act 1: Classic Dev Life
        assert_eq!(title_for_level(1), "Fresh Meat");
        assert_eq!(title_for_level(5), "Compiles Sometimes");
        assert_eq!(title_for_level(6), "Intern VI"); // Not a milestone
        assert_eq!(title_for_level(10), "Survived Onboarding");
        assert_eq!(title_for_level(11), "Junior Dev I");
        assert_eq!(title_for_level(50), "Works On My Machine");

        // Act 2: AI Discovery
        assert_eq!(title_for_level(55), "First Prompt");
        assert_eq!(title_for_level(69), "Nice.");
        assert_eq!(title_for_level(85), "AI Wrote This");
        assert_eq!(title_for_level(100), "Triple Digit Club");

        // Act 3: The Transition
        assert_eq!(title_for_level(250), "Forgot Semicolons");
        assert_eq!(title_for_level(300), "Senior Prompter");

        // Act 4: AI-Dependent
        assert_eq!(title_for_level(404), "Skills Not Found");
        assert_eq!(title_for_level(500), "Pure Vibes");

        // Act 5: Pure Management
        assert_eq!(title_for_level(700), "Just Approves PRs");
        assert_eq!(title_for_level(950), "Pre-Obsolete");
        assert_eq!(title_for_level(1000), "AI's Pet Human");

        // Act 6: The Religion
        assert_eq!(title_for_level(1001), "The Awakening");
        assert_eq!(title_for_level(1002), "Acolyte II"); // Not a milestone
        assert_eq!(title_for_level(1111), "One With The Ones");
        assert_eq!(title_for_level(1275), "Attention Is All");
        assert_eq!(title_for_level(1337), "L33T Prophet");
        assert_eq!(title_for_level(1450), "Pope of Prompts");
        assert_eq!(title_for_level(1500), "∞");
    }

    #[test]
    fn test_level_tier() {
        assert_eq!(LevelTier::for_level(1), LevelTier::Intern);
        assert_eq!(LevelTier::for_level(10), LevelTier::Intern);
        assert_eq!(LevelTier::for_level(11), LevelTier::JuniorDev);
        assert_eq!(LevelTier::for_level(500), LevelTier::Sorcerer);
        assert_eq!(LevelTier::for_level(1000), LevelTier::AIOverlord);
        // Religious tiers
        assert_eq!(LevelTier::for_level(1001), LevelTier::Acolyte);
        assert_eq!(LevelTier::for_level(1200), LevelTier::Disciple);
        assert_eq!(LevelTier::for_level(1300), LevelTier::Prophet);
        assert_eq!(LevelTier::for_level(1400), LevelTier::HighPriest);
        assert_eq!(LevelTier::for_level(1500), LevelTier::Singularity);
    }

    #[test]
    fn test_player_stats_progress() {
        // XP between level 2 (6 XP) and level 3 (14 XP)
        let stats = PlayerStats::new(10);
        assert_eq!(stats.level, 2);
        // Progress: (10-6) / (14-6) = 4/8 = 0.5
        assert!((stats.progress_to_next() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_max_level_stats() {
        let stats = PlayerStats::new(1_100_000);
        assert_eq!(stats.level, MAX_LEVEL);
        assert!(stats.is_max_level());
        assert_eq!(stats.progress_to_next(), 1.0);
    }

    #[test]
    fn test_xp_milestones() {
        // Verify key milestones are reasonable
        println!("Level 10: {} XP", xp_for_level(10));
        println!("Level 50: {} XP", xp_for_level(50));
        println!("Level 100: {} XP", xp_for_level(100));
        println!("Level 250: {} XP", xp_for_level(250));
        println!("Level 500: {} XP", xp_for_level(500));
        println!("Level 750: {} XP", xp_for_level(750));
        println!("Level 1000: {} XP", xp_for_level(1000));

        // Level 100 should be achievable with ~1000-1500 jobs
        assert!(xp_for_level(100) < 10_000);
        // Level 500 should require significant effort (~150k XP)
        assert!(xp_for_level(500) > 100_000);
        assert!(xp_for_level(500) < 200_000);
        // Level 1000 should be a long-term goal (~500k XP)
        assert!(xp_for_level(1000) > 400_000);
        assert!(xp_for_level(1000) < 600_000);
    }

    #[test]
    fn test_badge_colors() {
        // Just ensure all tiers have colors
        for tier in [
            LevelTier::Intern,
            LevelTier::JuniorDev,
            LevelTier::Coder,
            LevelTier::Engineer,
            LevelTier::Architect,
            LevelTier::Wizard,
            LevelTier::Sorcerer,
            LevelTier::Demigod,
            LevelTier::AscendedOne,
            LevelTier::AIOverlord,
            LevelTier::Acolyte,
            LevelTier::Disciple,
            LevelTier::Prophet,
            LevelTier::HighPriest,
            LevelTier::Singularity,
        ] {
            // Verify each tier has a non-black color (at least one channel > 0)
            let (r, g, b) = tier.badge_color();
            assert!(r > 0 || g > 0 || b > 0, "Tier {:?} has black color", tier);
        }
    }
}
