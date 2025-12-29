//! Progressive Challenge System
//!
//! 50 progressive challenges that unlock sequentially.
//! Each challenge must be completed before the next one becomes available.
//! Unlike weekly challenges, these are permanent and never reset.

/// Unique identifier for each challenge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChallengeId {
    // === TIER 1: Getting Started (1-10) ===
    FirstSteps,        // Complete 1 job
    WarmUp,            // Complete 3 jobs
    GettingComfortable, // Complete 5 jobs
    FirstStreak,       // Get a 2-day streak
    TryAMode,          // Use 2 different modes
    FirstTool,         // Use the Edit tool
    QuickOne,          // Complete a job under 60 seconds
    FirstFiles,        // Access 5 files
    KeepGoing,         // Complete 10 jobs
    WeekStreak,        // Get a 3-day streak

    // === TIER 2: Building Momentum (11-20) ===
    Productive,        // Complete 15 jobs
    ModeExplorer,      // Use 3 different modes
    ToolUser,          // Use 3 different tools
    Consistent,        // Complete 5 jobs in a row without failures
    SpeedUp,           // Complete a job under 45 seconds
    FileNavigator,     // Access 20 files
    TwentyJobs,        // Complete 20 jobs
    WeekendCoder,      // Complete a job on weekend
    TokenTracker,      // Process 5,000 tokens
    Momentum,          // Get a 5-day streak

    // === TIER 3: Getting Serious (21-30) ===
    ThirtyJobs,        // Complete 30 jobs
    MultiAgent,        // Use 2 different agents
    ModeVariety,       // Use 5 different modes
    ToolCollection,    // Use 5 different tools
    PerfectTen,        // 10 jobs in a row without failures
    FastWorker,        // Complete a job under 30 seconds
    FileMaster,        // Access 50 files
    FiftyJobs,         // Complete 50 jobs
    TokenMilestone,    // Process 20,000 tokens
    WeekStreak2,       // Get a 7-day streak

    // === TIER 4: Power User (31-40) ===
    SeventyFive,       // Complete 75 jobs
    ChainStarter,      // Complete your first chain
    ModeAdept,         // Use 7 different modes
    ToolMaster,        // Use 7 different tools
    FlawlessRun,       // 15 jobs in a row without failures
    LightningFast,     // Complete a job under 15 seconds
    FileExplorer,      // Access 100 files
    Century,           // Complete 100 jobs
    TokenPro,          // Process 50,000 tokens
    TwoWeekStreak,     // Get a 14-day streak

    // === TIER 5: Expert (41-50) ===
    Expert125,         // Complete 125 jobs
    ChainMaster,       // Complete 5 chains
    ModeCollector,     // Use 10 different modes
    ToolVirtuoso,      // Use 10 different tools
    Perfectionist,     // 25 jobs in a row without failures
    Instant,           // Complete a job under 10 seconds
    FileGuru,          // Access 200 unique files
    OneHundredFifty,   // Complete 150 jobs
    TokenMaster,       // Process 100,000 tokens
    MonthStreak,       // Get a 30-day streak
}

impl ChallengeId {
    /// Get the string ID for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            // Tier 1
            Self::FirstSteps => "ch_first_steps",
            Self::WarmUp => "ch_warm_up",
            Self::GettingComfortable => "ch_getting_comfortable",
            Self::FirstStreak => "ch_first_streak",
            Self::TryAMode => "ch_try_a_mode",
            Self::FirstTool => "ch_first_tool",
            Self::QuickOne => "ch_quick_one",
            Self::FirstFiles => "ch_first_files",
            Self::KeepGoing => "ch_keep_going",
            Self::WeekStreak => "ch_week_streak",
            // Tier 2
            Self::Productive => "ch_productive",
            Self::ModeExplorer => "ch_mode_explorer",
            Self::ToolUser => "ch_tool_user",
            Self::Consistent => "ch_consistent",
            Self::SpeedUp => "ch_speed_up",
            Self::FileNavigator => "ch_file_navigator",
            Self::TwentyJobs => "ch_twenty_jobs",
            Self::WeekendCoder => "ch_weekend_coder",
            Self::TokenTracker => "ch_token_tracker",
            Self::Momentum => "ch_momentum",
            // Tier 3
            Self::ThirtyJobs => "ch_thirty_jobs",
            Self::MultiAgent => "ch_multi_agent",
            Self::ModeVariety => "ch_mode_variety",
            Self::ToolCollection => "ch_tool_collection",
            Self::PerfectTen => "ch_perfect_ten",
            Self::FastWorker => "ch_fast_worker",
            Self::FileMaster => "ch_file_master",
            Self::FiftyJobs => "ch_fifty_jobs",
            Self::TokenMilestone => "ch_token_milestone",
            Self::WeekStreak2 => "ch_week_streak_2",
            // Tier 4
            Self::SeventyFive => "ch_seventy_five",
            Self::ChainStarter => "ch_chain_starter",
            Self::ModeAdept => "ch_mode_adept",
            Self::ToolMaster => "ch_tool_master",
            Self::FlawlessRun => "ch_flawless_run",
            Self::LightningFast => "ch_lightning_fast",
            Self::FileExplorer => "ch_file_explorer",
            Self::Century => "ch_century",
            Self::TokenPro => "ch_token_pro",
            Self::TwoWeekStreak => "ch_two_week_streak",
            // Tier 5
            Self::Expert125 => "ch_expert_125",
            Self::ChainMaster => "ch_chain_master",
            Self::ModeCollector => "ch_mode_collector",
            Self::ToolVirtuoso => "ch_tool_virtuoso",
            Self::Perfectionist => "ch_perfectionist",
            Self::Instant => "ch_instant",
            Self::FileGuru => "ch_file_guru",
            Self::OneHundredFifty => "ch_one_hundred_fifty",
            Self::TokenMaster => "ch_token_master",
            Self::MonthStreak => "ch_month_streak",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // Tier 1
            "ch_first_steps" => Some(Self::FirstSteps),
            "ch_warm_up" => Some(Self::WarmUp),
            "ch_getting_comfortable" => Some(Self::GettingComfortable),
            "ch_first_streak" => Some(Self::FirstStreak),
            "ch_try_a_mode" => Some(Self::TryAMode),
            "ch_first_tool" => Some(Self::FirstTool),
            "ch_quick_one" => Some(Self::QuickOne),
            "ch_first_files" => Some(Self::FirstFiles),
            "ch_keep_going" => Some(Self::KeepGoing),
            "ch_week_streak" => Some(Self::WeekStreak),
            // Tier 2
            "ch_productive" => Some(Self::Productive),
            "ch_mode_explorer" => Some(Self::ModeExplorer),
            "ch_tool_user" => Some(Self::ToolUser),
            "ch_consistent" => Some(Self::Consistent),
            "ch_speed_up" => Some(Self::SpeedUp),
            "ch_file_navigator" => Some(Self::FileNavigator),
            "ch_twenty_jobs" => Some(Self::TwentyJobs),
            "ch_weekend_coder" => Some(Self::WeekendCoder),
            "ch_token_tracker" => Some(Self::TokenTracker),
            "ch_momentum" => Some(Self::Momentum),
            // Tier 3
            "ch_thirty_jobs" => Some(Self::ThirtyJobs),
            "ch_multi_agent" => Some(Self::MultiAgent),
            "ch_mode_variety" => Some(Self::ModeVariety),
            "ch_tool_collection" => Some(Self::ToolCollection),
            "ch_perfect_ten" => Some(Self::PerfectTen),
            "ch_fast_worker" => Some(Self::FastWorker),
            "ch_file_master" => Some(Self::FileMaster),
            "ch_fifty_jobs" => Some(Self::FiftyJobs),
            "ch_token_milestone" => Some(Self::TokenMilestone),
            "ch_week_streak_2" => Some(Self::WeekStreak2),
            // Tier 4
            "ch_seventy_five" => Some(Self::SeventyFive),
            "ch_chain_starter" => Some(Self::ChainStarter),
            "ch_mode_adept" => Some(Self::ModeAdept),
            "ch_tool_master" => Some(Self::ToolMaster),
            "ch_flawless_run" => Some(Self::FlawlessRun),
            "ch_lightning_fast" => Some(Self::LightningFast),
            "ch_file_explorer" => Some(Self::FileExplorer),
            "ch_century" => Some(Self::Century),
            "ch_token_pro" => Some(Self::TokenPro),
            "ch_two_week_streak" => Some(Self::TwoWeekStreak),
            // Tier 5
            "ch_expert_125" => Some(Self::Expert125),
            "ch_chain_master" => Some(Self::ChainMaster),
            "ch_mode_collector" => Some(Self::ModeCollector),
            "ch_tool_virtuoso" => Some(Self::ToolVirtuoso),
            "ch_perfectionist" => Some(Self::Perfectionist),
            "ch_instant" => Some(Self::Instant),
            "ch_file_guru" => Some(Self::FileGuru),
            "ch_one_hundred_fifty" => Some(Self::OneHundredFifty),
            "ch_token_master" => Some(Self::TokenMaster),
            "ch_month_streak" => Some(Self::MonthStreak),
            _ => None,
        }
    }

    /// Get the challenge number (1-50)
    pub fn number(&self) -> u32 {
        match self {
            // Tier 1: 1-10
            Self::FirstSteps => 1,
            Self::WarmUp => 2,
            Self::GettingComfortable => 3,
            Self::FirstStreak => 4,
            Self::TryAMode => 5,
            Self::FirstTool => 6,
            Self::QuickOne => 7,
            Self::FirstFiles => 8,
            Self::KeepGoing => 9,
            Self::WeekStreak => 10,
            // Tier 2: 11-20
            Self::Productive => 11,
            Self::ModeExplorer => 12,
            Self::ToolUser => 13,
            Self::Consistent => 14,
            Self::SpeedUp => 15,
            Self::FileNavigator => 16,
            Self::TwentyJobs => 17,
            Self::WeekendCoder => 18,
            Self::TokenTracker => 19,
            Self::Momentum => 20,
            // Tier 3: 21-30
            Self::ThirtyJobs => 21,
            Self::MultiAgent => 22,
            Self::ModeVariety => 23,
            Self::ToolCollection => 24,
            Self::PerfectTen => 25,
            Self::FastWorker => 26,
            Self::FileMaster => 27,
            Self::FiftyJobs => 28,
            Self::TokenMilestone => 29,
            Self::WeekStreak2 => 30,
            // Tier 4: 31-40
            Self::SeventyFive => 31,
            Self::ChainStarter => 32,
            Self::ModeAdept => 33,
            Self::ToolMaster => 34,
            Self::FlawlessRun => 35,
            Self::LightningFast => 36,
            Self::FileExplorer => 37,
            Self::Century => 38,
            Self::TokenPro => 39,
            Self::TwoWeekStreak => 40,
            // Tier 5: 41-50
            Self::Expert125 => 41,
            Self::ChainMaster => 42,
            Self::ModeCollector => 43,
            Self::ToolVirtuoso => 44,
            Self::Perfectionist => 45,
            Self::Instant => 46,
            Self::FileGuru => 47,
            Self::OneHundredFifty => 48,
            Self::TokenMaster => 49,
            Self::MonthStreak => 50,
        }
    }

    /// Get all challenge IDs in order
    pub fn all() -> &'static [ChallengeId] {
        &[
            // Tier 1
            Self::FirstSteps,
            Self::WarmUp,
            Self::GettingComfortable,
            Self::FirstStreak,
            Self::TryAMode,
            Self::FirstTool,
            Self::QuickOne,
            Self::FirstFiles,
            Self::KeepGoing,
            Self::WeekStreak,
            // Tier 2
            Self::Productive,
            Self::ModeExplorer,
            Self::ToolUser,
            Self::Consistent,
            Self::SpeedUp,
            Self::FileNavigator,
            Self::TwentyJobs,
            Self::WeekendCoder,
            Self::TokenTracker,
            Self::Momentum,
            // Tier 3
            Self::ThirtyJobs,
            Self::MultiAgent,
            Self::ModeVariety,
            Self::ToolCollection,
            Self::PerfectTen,
            Self::FastWorker,
            Self::FileMaster,
            Self::FiftyJobs,
            Self::TokenMilestone,
            Self::WeekStreak2,
            // Tier 4
            Self::SeventyFive,
            Self::ChainStarter,
            Self::ModeAdept,
            Self::ToolMaster,
            Self::FlawlessRun,
            Self::LightningFast,
            Self::FileExplorer,
            Self::Century,
            Self::TokenPro,
            Self::TwoWeekStreak,
            // Tier 5
            Self::Expert125,
            Self::ChainMaster,
            Self::ModeCollector,
            Self::ToolVirtuoso,
            Self::Perfectionist,
            Self::Instant,
            Self::FileGuru,
            Self::OneHundredFifty,
            Self::TokenMaster,
            Self::MonthStreak,
        ]
    }
}

/// Challenge tier (for display grouping)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeTier {
    GettingStarted,   // 1-10
    BuildingMomentum, // 11-20
    GettingSerious,   // 21-30
    PowerUser,        // 31-40
    Expert,           // 41-50
}

impl ChallengeTier {
    pub fn for_challenge(number: u32) -> Self {
        match number {
            1..=10 => Self::GettingStarted,
            11..=20 => Self::BuildingMomentum,
            21..=30 => Self::GettingSerious,
            31..=40 => Self::PowerUser,
            _ => Self::Expert,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::GettingStarted => "Getting Started",
            Self::BuildingMomentum => "Building Momentum",
            Self::GettingSerious => "Getting Serious",
            Self::PowerUser => "Power User",
            Self::Expert => "Expert",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::GettingStarted => "🌱",
            Self::BuildingMomentum => "🚀",
            Self::GettingSerious => "💪",
            Self::PowerUser => "⚡",
            Self::Expert => "👑",
        }
    }
}

/// Challenge requirement type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeRequirement {
    /// Complete N jobs total
    TotalJobs(u32),
    /// Complete N jobs in a row without failures
    SuccessStreak(u32),
    /// Maintain a N-day streak
    DailyStreak(u32),
    /// Use N different modes
    UniqueModes(u32),
    /// Use N different tools
    UniqueTools(u32),
    /// Use N different agents
    UniqueAgents(u32),
    /// Complete a job in under N milliseconds
    JobUnderMs(u64),
    /// Access N files
    FilesAccessed(u32),
    /// Process N tokens total
    TotalTokens(u64),
    /// Complete N chains
    TotalChains(u32),
    /// Complete a job on a weekend
    WeekendJob,
}

/// Challenge definition with all metadata
#[derive(Debug, Clone)]
pub struct Challenge {
    pub id: ChallengeId,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub tier: ChallengeTier,
    pub xp_reward: u32,
    pub requirement: ChallengeRequirement,
}

/// All 50 challenge definitions
pub static CHALLENGES: &[Challenge] = &[
    // ============================================================
    // TIER 1: Getting Started (1-10)
    // ============================================================
    Challenge {
        id: ChallengeId::FirstSteps,
        name: "First Steps",
        description: "Complete your first job",
        icon: "👣",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 10,
        requirement: ChallengeRequirement::TotalJobs(1),
    },
    Challenge {
        id: ChallengeId::WarmUp,
        name: "Warm Up",
        description: "Complete 3 jobs",
        icon: "🔥",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 15,
        requirement: ChallengeRequirement::TotalJobs(3),
    },
    Challenge {
        id: ChallengeId::GettingComfortable,
        name: "Getting Comfortable",
        description: "Complete 5 jobs",
        icon: "🛋️",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 20,
        requirement: ChallengeRequirement::TotalJobs(5),
    },
    Challenge {
        id: ChallengeId::FirstStreak,
        name: "First Streak",
        description: "Get a 2-day streak",
        icon: "🔥",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 25,
        requirement: ChallengeRequirement::DailyStreak(2),
    },
    Challenge {
        id: ChallengeId::TryAMode,
        name: "Try a Mode",
        description: "Use 2 different modes",
        icon: "🎭",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 20,
        requirement: ChallengeRequirement::UniqueModes(2),
    },
    Challenge {
        id: ChallengeId::FirstTool,
        name: "First Tool",
        description: "Use at least one tool",
        icon: "🔧",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 15,
        requirement: ChallengeRequirement::UniqueTools(1),
    },
    Challenge {
        id: ChallengeId::QuickOne,
        name: "Quick One",
        description: "Complete a job under 60 seconds",
        icon: "⏱️",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 25,
        requirement: ChallengeRequirement::JobUnderMs(60_000),
    },
    Challenge {
        id: ChallengeId::FirstFiles,
        name: "First Files",
        description: "Access 5 files",
        icon: "📁",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 20,
        requirement: ChallengeRequirement::FilesAccessed(5),
    },
    Challenge {
        id: ChallengeId::KeepGoing,
        name: "Keep Going",
        description: "Complete 10 jobs",
        icon: "🏃",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 30,
        requirement: ChallengeRequirement::TotalJobs(10),
    },
    Challenge {
        id: ChallengeId::WeekStreak,
        name: "Week Streak",
        description: "Get a 3-day streak",
        icon: "📅",
        tier: ChallengeTier::GettingStarted,
        xp_reward: 35,
        requirement: ChallengeRequirement::DailyStreak(3),
    },

    // ============================================================
    // TIER 2: Building Momentum (11-20)
    // ============================================================
    Challenge {
        id: ChallengeId::Productive,
        name: "Productive",
        description: "Complete 15 jobs",
        icon: "💼",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 35,
        requirement: ChallengeRequirement::TotalJobs(15),
    },
    Challenge {
        id: ChallengeId::ModeExplorer,
        name: "Mode Explorer",
        description: "Use 3 different modes",
        icon: "🗺️",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 30,
        requirement: ChallengeRequirement::UniqueModes(3),
    },
    Challenge {
        id: ChallengeId::ToolUser,
        name: "Tool User",
        description: "Use 3 different tools",
        icon: "🛠️",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 30,
        requirement: ChallengeRequirement::UniqueTools(3),
    },
    Challenge {
        id: ChallengeId::Consistent,
        name: "Consistent",
        description: "Complete 5 jobs in a row without failures",
        icon: "✅",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 40,
        requirement: ChallengeRequirement::SuccessStreak(5),
    },
    Challenge {
        id: ChallengeId::SpeedUp,
        name: "Speed Up",
        description: "Complete a job under 45 seconds",
        icon: "⚡",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 35,
        requirement: ChallengeRequirement::JobUnderMs(45_000),
    },
    Challenge {
        id: ChallengeId::FileNavigator,
        name: "File Navigator",
        description: "Access 20 files",
        icon: "📂",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 30,
        requirement: ChallengeRequirement::FilesAccessed(20),
    },
    Challenge {
        id: ChallengeId::TwentyJobs,
        name: "Twenty Jobs",
        description: "Complete 20 jobs",
        icon: "2️⃣0️⃣",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 40,
        requirement: ChallengeRequirement::TotalJobs(20),
    },
    Challenge {
        id: ChallengeId::WeekendCoder,
        name: "Weekend Coder",
        description: "Complete a job on the weekend",
        icon: "🎮",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 25,
        requirement: ChallengeRequirement::WeekendJob,
    },
    Challenge {
        id: ChallengeId::TokenTracker,
        name: "Token Tracker",
        description: "Process 5,000 tokens",
        icon: "🪙",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 35,
        requirement: ChallengeRequirement::TotalTokens(5_000),
    },
    Challenge {
        id: ChallengeId::Momentum,
        name: "Momentum",
        description: "Get a 5-day streak",
        icon: "🚀",
        tier: ChallengeTier::BuildingMomentum,
        xp_reward: 50,
        requirement: ChallengeRequirement::DailyStreak(5),
    },

    // ============================================================
    // TIER 3: Getting Serious (21-30)
    // ============================================================
    Challenge {
        id: ChallengeId::ThirtyJobs,
        name: "Thirty Jobs",
        description: "Complete 30 jobs",
        icon: "3️⃣0️⃣",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 50,
        requirement: ChallengeRequirement::TotalJobs(30),
    },
    Challenge {
        id: ChallengeId::MultiAgent,
        name: "Multi-Agent",
        description: "Use 2 different agents",
        icon: "🤖",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 40,
        requirement: ChallengeRequirement::UniqueAgents(2),
    },
    Challenge {
        id: ChallengeId::ModeVariety,
        name: "Mode Variety",
        description: "Use 5 different modes",
        icon: "🎨",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 45,
        requirement: ChallengeRequirement::UniqueModes(5),
    },
    Challenge {
        id: ChallengeId::ToolCollection,
        name: "Tool Collection",
        description: "Use 5 different tools",
        icon: "🧰",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 45,
        requirement: ChallengeRequirement::UniqueTools(5),
    },
    Challenge {
        id: ChallengeId::PerfectTen,
        name: "Perfect Ten",
        description: "Complete 10 jobs in a row without failures",
        icon: "🎯",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 60,
        requirement: ChallengeRequirement::SuccessStreak(10),
    },
    Challenge {
        id: ChallengeId::FastWorker,
        name: "Fast Worker",
        description: "Complete a job under 30 seconds",
        icon: "💨",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 50,
        requirement: ChallengeRequirement::JobUnderMs(30_000),
    },
    Challenge {
        id: ChallengeId::FileMaster,
        name: "File Master",
        description: "Access 50 files",
        icon: "🗂️",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 45,
        requirement: ChallengeRequirement::FilesAccessed(50),
    },
    Challenge {
        id: ChallengeId::FiftyJobs,
        name: "Fifty Jobs",
        description: "Complete 50 jobs",
        icon: "5️⃣0️⃣",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 65,
        requirement: ChallengeRequirement::TotalJobs(50),
    },
    Challenge {
        id: ChallengeId::TokenMilestone,
        name: "Token Milestone",
        description: "Process 20,000 tokens",
        icon: "💰",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 50,
        requirement: ChallengeRequirement::TotalTokens(20_000),
    },
    Challenge {
        id: ChallengeId::WeekStreak2,
        name: "Week Streak",
        description: "Get a 7-day streak",
        icon: "🗓️",
        tier: ChallengeTier::GettingSerious,
        xp_reward: 75,
        requirement: ChallengeRequirement::DailyStreak(7),
    },

    // ============================================================
    // TIER 4: Power User (31-40)
    // ============================================================
    Challenge {
        id: ChallengeId::SeventyFive,
        name: "Seventy Five",
        description: "Complete 75 jobs",
        icon: "7️⃣5️⃣",
        tier: ChallengeTier::PowerUser,
        xp_reward: 75,
        requirement: ChallengeRequirement::TotalJobs(75),
    },
    Challenge {
        id: ChallengeId::ChainStarter,
        name: "Chain Starter",
        description: "Complete your first chain",
        icon: "🔗",
        tier: ChallengeTier::PowerUser,
        xp_reward: 60,
        requirement: ChallengeRequirement::TotalChains(1),
    },
    Challenge {
        id: ChallengeId::ModeAdept,
        name: "Mode Adept",
        description: "Use 7 different modes",
        icon: "🎭",
        tier: ChallengeTier::PowerUser,
        xp_reward: 60,
        requirement: ChallengeRequirement::UniqueModes(7),
    },
    Challenge {
        id: ChallengeId::ToolMaster,
        name: "Tool Master",
        description: "Use 7 different tools",
        icon: "⚙️",
        tier: ChallengeTier::PowerUser,
        xp_reward: 60,
        requirement: ChallengeRequirement::UniqueTools(7),
    },
    Challenge {
        id: ChallengeId::FlawlessRun,
        name: "Flawless Run",
        description: "Complete 15 jobs in a row without failures",
        icon: "✨",
        tier: ChallengeTier::PowerUser,
        xp_reward: 80,
        requirement: ChallengeRequirement::SuccessStreak(15),
    },
    Challenge {
        id: ChallengeId::LightningFast,
        name: "Lightning Fast",
        description: "Complete a job under 15 seconds",
        icon: "⚡",
        tier: ChallengeTier::PowerUser,
        xp_reward: 70,
        requirement: ChallengeRequirement::JobUnderMs(15_000),
    },
    Challenge {
        id: ChallengeId::FileExplorer,
        name: "File Explorer",
        description: "Access 100 files",
        icon: "📚",
        tier: ChallengeTier::PowerUser,
        xp_reward: 65,
        requirement: ChallengeRequirement::FilesAccessed(100),
    },
    Challenge {
        id: ChallengeId::Century,
        name: "Century",
        description: "Complete 100 jobs",
        icon: "💯",
        tier: ChallengeTier::PowerUser,
        xp_reward: 100,
        requirement: ChallengeRequirement::TotalJobs(100),
    },
    Challenge {
        id: ChallengeId::TokenPro,
        name: "Token Pro",
        description: "Process 50,000 tokens",
        icon: "💎",
        tier: ChallengeTier::PowerUser,
        xp_reward: 75,
        requirement: ChallengeRequirement::TotalTokens(50_000),
    },
    Challenge {
        id: ChallengeId::TwoWeekStreak,
        name: "Two Week Streak",
        description: "Get a 14-day streak",
        icon: "🏆",
        tier: ChallengeTier::PowerUser,
        xp_reward: 100,
        requirement: ChallengeRequirement::DailyStreak(14),
    },

    // ============================================================
    // TIER 5: Expert (41-50)
    // ============================================================
    Challenge {
        id: ChallengeId::Expert125,
        name: "Expert 125",
        description: "Complete 125 jobs",
        icon: "🎖️",
        tier: ChallengeTier::Expert,
        xp_reward: 100,
        requirement: ChallengeRequirement::TotalJobs(125),
    },
    Challenge {
        id: ChallengeId::ChainMaster,
        name: "Chain Master",
        description: "Complete 5 chains",
        icon: "⛓️",
        tier: ChallengeTier::Expert,
        xp_reward: 90,
        requirement: ChallengeRequirement::TotalChains(5),
    },
    Challenge {
        id: ChallengeId::ModeCollector,
        name: "Mode Collector",
        description: "Use 10 different modes",
        icon: "🏛️",
        tier: ChallengeTier::Expert,
        xp_reward: 85,
        requirement: ChallengeRequirement::UniqueModes(10),
    },
    Challenge {
        id: ChallengeId::ToolVirtuoso,
        name: "Tool Virtuoso",
        description: "Use 10 different tools",
        icon: "🎸",
        tier: ChallengeTier::Expert,
        xp_reward: 85,
        requirement: ChallengeRequirement::UniqueTools(10),
    },
    Challenge {
        id: ChallengeId::Perfectionist,
        name: "Perfectionist",
        description: "Complete 25 jobs in a row without failures",
        icon: "💎",
        tier: ChallengeTier::Expert,
        xp_reward: 120,
        requirement: ChallengeRequirement::SuccessStreak(25),
    },
    Challenge {
        id: ChallengeId::Instant,
        name: "Instant",
        description: "Complete a job under 10 seconds",
        icon: "🌟",
        tier: ChallengeTier::Expert,
        xp_reward: 100,
        requirement: ChallengeRequirement::JobUnderMs(10_000),
    },
    Challenge {
        id: ChallengeId::FileGuru,
        name: "File Guru",
        description: "Access 200 unique files",
        icon: "🗃️",
        tier: ChallengeTier::Expert,
        xp_reward: 90,
        requirement: ChallengeRequirement::FilesAccessed(200),
    },
    Challenge {
        id: ChallengeId::OneHundredFifty,
        name: "One Hundred Fifty",
        description: "Complete 150 jobs",
        icon: "🌟",
        tier: ChallengeTier::Expert,
        xp_reward: 125,
        requirement: ChallengeRequirement::TotalJobs(150),
    },
    Challenge {
        id: ChallengeId::TokenMaster,
        name: "Token Master",
        description: "Process 100,000 tokens",
        icon: "👑",
        tier: ChallengeTier::Expert,
        xp_reward: 100,
        requirement: ChallengeRequirement::TotalTokens(100_000),
    },
    Challenge {
        id: ChallengeId::MonthStreak,
        name: "Month Streak",
        description: "Get a 30-day streak",
        icon: "🏅",
        tier: ChallengeTier::Expert,
        xp_reward: 150,
        requirement: ChallengeRequirement::DailyStreak(30),
    },
];

impl Challenge {
    /// Get challenge definition by ID
    pub fn get(id: ChallengeId) -> &'static Challenge {
        CHALLENGES
            .iter()
            .find(|c| c.id == id)
            .expect("All challenges should be defined")
    }

    /// Get challenge by number (1-50)
    pub fn get_by_number(number: u32) -> Option<&'static Challenge> {
        if number == 0 || number > 50 {
            return None;
        }
        CHALLENGES.get((number - 1) as usize)
    }

    /// Get total number of challenges
    pub fn total_count() -> usize {
        CHALLENGES.len()
    }

    /// Get total possible XP from all challenges
    pub fn total_xp() -> u32 {
        CHALLENGES.iter().map(|c| c.xp_reward).sum()
    }

    /// Get challenges by tier
    pub fn by_tier(tier: ChallengeTier) -> Vec<&'static Challenge> {
        CHALLENGES.iter().filter(|c| c.tier == tier).collect()
    }
}

/// Progress state for a challenge
#[derive(Debug, Clone)]
pub struct ChallengeProgress {
    pub challenge: &'static Challenge,
    pub current_value: u64,
    pub target_value: u64,
    pub completed: bool,
    pub completed_at: Option<i64>,
}

impl ChallengeProgress {
    /// Calculate progress percentage (0.0 - 1.0)
    pub fn progress_percent(&self) -> f32 {
        if self.completed || self.target_value == 0 {
            1.0
        } else {
            (self.current_value as f32 / self.target_value as f32).min(1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_count() {
        assert_eq!(CHALLENGES.len(), 50, "Should have exactly 50 challenges");
    }

    #[test]
    fn test_all_ids_unique() {
        let mut ids: Vec<_> = ChallengeId::all().iter().map(|id| id.as_str()).collect();
        ids.sort();
        let unique_count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique_count, "All challenge IDs should be unique");
    }

    #[test]
    fn test_challenge_numbers() {
        for (i, challenge) in CHALLENGES.iter().enumerate() {
            assert_eq!(
                challenge.id.number(),
                (i + 1) as u32,
                "Challenge number should match array index + 1"
            );
        }
    }

    #[test]
    fn test_tier_distribution() {
        assert_eq!(Challenge::by_tier(ChallengeTier::GettingStarted).len(), 10);
        assert_eq!(Challenge::by_tier(ChallengeTier::BuildingMomentum).len(), 10);
        assert_eq!(Challenge::by_tier(ChallengeTier::GettingSerious).len(), 10);
        assert_eq!(Challenge::by_tier(ChallengeTier::PowerUser).len(), 10);
        assert_eq!(Challenge::by_tier(ChallengeTier::Expert).len(), 10);
    }

    #[test]
    fn test_total_xp() {
        let total = Challenge::total_xp();
        println!("Total possible XP from challenges: {}", total);
        assert!(total > 2000, "Total challenge XP should be substantial");
    }
}
