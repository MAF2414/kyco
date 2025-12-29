//! Achievement definitions and metadata
//!
//! All achievements are defined here with their unlock conditions and rewards.

/// Unique identifier for each achievement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AchievementId {
    // Milestone achievements
    FirstJob,
    TenJobs,
    FiftyJobs,
    Century,
    FiveHundred,
    Thousand,

    // Chain achievements
    FirstChain,
    ChainMaster,

    // Mode achievements
    Polyglot,
    ModeExplorer,

    // Agent achievements
    MultiAgent,

    // Skill achievements
    Flawless10,
    Flawless25,
    SpeedDemon,
    TokenSaver,

    // Time achievements
    NightOwl,
    EarlyBird,
    WeekendWarrior,

    // Streak achievements
    Streak3,
    Streak7,
    Streak30,
}

impl AchievementId {
    /// Get the string ID for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FirstJob => "first_job",
            Self::TenJobs => "ten_jobs",
            Self::FiftyJobs => "fifty_jobs",
            Self::Century => "century",
            Self::FiveHundred => "five_hundred",
            Self::Thousand => "thousand",
            Self::FirstChain => "first_chain",
            Self::ChainMaster => "chain_master",
            Self::Polyglot => "polyglot",
            Self::ModeExplorer => "mode_explorer",
            Self::MultiAgent => "multi_agent",
            Self::Flawless10 => "flawless_10",
            Self::Flawless25 => "flawless_25",
            Self::SpeedDemon => "speed_demon",
            Self::TokenSaver => "token_saver",
            Self::NightOwl => "night_owl",
            Self::EarlyBird => "early_bird",
            Self::WeekendWarrior => "weekend_warrior",
            Self::Streak3 => "streak_3",
            Self::Streak7 => "streak_7",
            Self::Streak30 => "streak_30",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "first_job" => Some(Self::FirstJob),
            "ten_jobs" => Some(Self::TenJobs),
            "fifty_jobs" => Some(Self::FiftyJobs),
            "century" => Some(Self::Century),
            "five_hundred" => Some(Self::FiveHundred),
            "thousand" => Some(Self::Thousand),
            "first_chain" => Some(Self::FirstChain),
            "chain_master" => Some(Self::ChainMaster),
            "polyglot" => Some(Self::Polyglot),
            "mode_explorer" => Some(Self::ModeExplorer),
            "multi_agent" => Some(Self::MultiAgent),
            "flawless_10" => Some(Self::Flawless10),
            "flawless_25" => Some(Self::Flawless25),
            "speed_demon" => Some(Self::SpeedDemon),
            "token_saver" => Some(Self::TokenSaver),
            "night_owl" => Some(Self::NightOwl),
            "early_bird" => Some(Self::EarlyBird),
            "weekend_warrior" => Some(Self::WeekendWarrior),
            "streak_3" => Some(Self::Streak3),
            "streak_7" => Some(Self::Streak7),
            "streak_30" => Some(Self::Streak30),
            _ => None,
        }
    }

    /// Get all achievement IDs
    pub fn all() -> &'static [AchievementId] {
        &[
            Self::FirstJob,
            Self::TenJobs,
            Self::FiftyJobs,
            Self::Century,
            Self::FiveHundred,
            Self::Thousand,
            Self::FirstChain,
            Self::ChainMaster,
            Self::Polyglot,
            Self::ModeExplorer,
            Self::MultiAgent,
            Self::Flawless10,
            Self::Flawless25,
            Self::SpeedDemon,
            Self::TokenSaver,
            Self::NightOwl,
            Self::EarlyBird,
            Self::WeekendWarrior,
            Self::Streak3,
            Self::Streak7,
            Self::Streak30,
        ]
    }
}

/// Achievement category for grouping in UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchievementCategory {
    Milestone,
    Chain,
    Mode,
    Agent,
    Skill,
    Time,
    Streak,
}

impl AchievementCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Milestone => "Milestones",
            Self::Chain => "Chains",
            Self::Mode => "Modes",
            Self::Agent => "Agents",
            Self::Skill => "Skills",
            Self::Time => "Time",
            Self::Streak => "Streaks",
        }
    }
}

/// Achievement definition with all metadata
#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: AchievementId,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub category: AchievementCategory,
    pub xp_reward: u32,
    /// For progressive achievements, the target count
    pub target: Option<u32>,
}

/// All achievement definitions
pub static ACHIEVEMENTS: &[Achievement] = &[
    // === MILESTONE ===
    Achievement {
        id: AchievementId::FirstJob,
        name: "First Steps",
        description: "Complete your first job",
        icon: "🎯",
        category: AchievementCategory::Milestone,
        xp_reward: 10,
        target: Some(1),
    },
    Achievement {
        id: AchievementId::TenJobs,
        name: "Getting Started",
        description: "Complete 10 jobs",
        icon: "📈",
        category: AchievementCategory::Milestone,
        xp_reward: 25,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::FiftyJobs,
        name: "Productive",
        description: "Complete 50 jobs",
        icon: "💪",
        category: AchievementCategory::Milestone,
        xp_reward: 50,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Century,
        name: "Century",
        description: "Complete 100 jobs",
        icon: "💯",
        category: AchievementCategory::Milestone,
        xp_reward: 100,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::FiveHundred,
        name: "Veteran",
        description: "Complete 500 jobs",
        icon: "🏅",
        category: AchievementCategory::Milestone,
        xp_reward: 250,
        target: Some(500),
    },
    Achievement {
        id: AchievementId::Thousand,
        name: "Legend",
        description: "Complete 1000 jobs",
        icon: "🏆",
        category: AchievementCategory::Milestone,
        xp_reward: 500,
        target: Some(1000),
    },
    // === CHAIN ===
    Achievement {
        id: AchievementId::FirstChain,
        name: "Chain Reaction",
        description: "Complete your first chain",
        icon: "🔗",
        category: AchievementCategory::Chain,
        xp_reward: 25,
        target: Some(1),
    },
    Achievement {
        id: AchievementId::ChainMaster,
        name: "Chain Master",
        description: "Complete 10 chains",
        icon: "⛓️",
        category: AchievementCategory::Chain,
        xp_reward: 100,
        target: Some(10),
    },
    // === MODE ===
    Achievement {
        id: AchievementId::Polyglot,
        name: "Polyglot",
        description: "Use 5 different modes",
        icon: "🎭",
        category: AchievementCategory::Mode,
        xp_reward: 50,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::ModeExplorer,
        name: "Mode Explorer",
        description: "Use 10 different modes",
        icon: "🗺️",
        category: AchievementCategory::Mode,
        xp_reward: 100,
        target: Some(10),
    },
    // === AGENT ===
    Achievement {
        id: AchievementId::MultiAgent,
        name: "Multi-Agent",
        description: "Use both Claude and Codex",
        icon: "🤖",
        category: AchievementCategory::Agent,
        xp_reward: 25,
        target: Some(2),
    },
    // === SKILL ===
    Achievement {
        id: AchievementId::Flawless10,
        name: "Flawless",
        description: "Complete 10 jobs in a row without failures",
        icon: "✨",
        category: AchievementCategory::Skill,
        xp_reward: 75,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::Flawless25,
        name: "Perfectionist",
        description: "Complete 25 jobs in a row without failures",
        icon: "💎",
        category: AchievementCategory::Skill,
        xp_reward: 150,
        target: Some(25),
    },
    Achievement {
        id: AchievementId::SpeedDemon,
        name: "Speed Demon",
        description: "Complete a job in under 30 seconds",
        icon: "⚡",
        category: AchievementCategory::Skill,
        xp_reward: 50,
        target: None,
    },
    Achievement {
        id: AchievementId::TokenSaver,
        name: "Token Saver",
        description: "Complete a job using less than 500 tokens",
        icon: "🪙",
        category: AchievementCategory::Skill,
        xp_reward: 25,
        target: None,
    },
    // === TIME ===
    Achievement {
        id: AchievementId::NightOwl,
        name: "Night Owl",
        description: "Complete a job between midnight and 5 AM",
        icon: "🦉",
        category: AchievementCategory::Time,
        xp_reward: 15,
        target: None,
    },
    Achievement {
        id: AchievementId::EarlyBird,
        name: "Early Bird",
        description: "Complete a job between 5 AM and 7 AM",
        icon: "🐦",
        category: AchievementCategory::Time,
        xp_reward: 15,
        target: None,
    },
    Achievement {
        id: AchievementId::WeekendWarrior,
        name: "Weekend Warrior",
        description: "Complete a job on the weekend",
        icon: "🎮",
        category: AchievementCategory::Time,
        xp_reward: 10,
        target: None,
    },
    // === STREAK ===
    Achievement {
        id: AchievementId::Streak3,
        name: "On Fire",
        description: "Maintain a 3-day streak",
        icon: "🔥",
        category: AchievementCategory::Streak,
        xp_reward: 30,
        target: Some(3),
    },
    Achievement {
        id: AchievementId::Streak7,
        name: "Week Warrior",
        description: "Maintain a 7-day streak",
        icon: "📅",
        category: AchievementCategory::Streak,
        xp_reward: 75,
        target: Some(7),
    },
    Achievement {
        id: AchievementId::Streak30,
        name: "Monthly Master",
        description: "Maintain a 30-day streak",
        icon: "👑",
        category: AchievementCategory::Streak,
        xp_reward: 300,
        target: Some(30),
    },
];

impl Achievement {
    /// Get achievement definition by ID
    pub fn get(id: AchievementId) -> &'static Achievement {
        ACHIEVEMENTS
            .iter()
            .find(|a| a.id == id)
            .expect("All achievements should be defined")
    }

    /// Get total number of achievements
    pub fn total_count() -> usize {
        ACHIEVEMENTS.len()
    }

    /// Get total possible XP from all achievements
    pub fn total_xp() -> u32 {
        ACHIEVEMENTS.iter().map(|a| a.xp_reward).sum()
    }
}
