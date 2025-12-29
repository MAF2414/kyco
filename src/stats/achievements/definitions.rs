//! Achievement definitions and metadata
//!
//! All 100 achievements are defined here with their unlock conditions and rewards.

/// Unique identifier for each achievement
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AchievementId {
    // === MILESTONE (10) - Job completion counts ===
    FirstJob,
    TenJobs,
    FiftyJobs,
    Century,
    TwoFifty,
    FiveHundred,
    Thousand,
    TwoThousandFive,
    FiveThousand,
    TenThousand,

    // === CHAIN (6) - Chain completion counts ===
    FirstChain,
    FiveChains,
    TenChains,
    TwentyFiveChains,
    FiftyChains,
    HundredChains,

    // === MODE (5) - Different modes used ===
    ModeNovice,     // 3 modes
    Polyglot,       // 5 modes
    ModeExplorer,   // 10 modes
    ModeMaster,     // 15 modes
    ModeCollector,  // 20 modes

    // === AGENT (4) - Different agents used ===
    MultiAgent,     // 2 agents
    TripleAgent,    // 3 agents
    QuadAgent,      // 4 agents
    AgentCollector, // 5 agents

    // === SKILL (10) - Performance achievements ===
    Flawless5,
    Flawless10,
    Flawless25,
    Flawless50,
    Flawless100,
    SpeedDemon,     // <30s
    LightningFast,  // <15s
    Instant,        // <10s
    TokenSaver,     // <500 tokens
    Efficient,      // <1000 tokens

    // === TIME (5) - Time-based achievements ===
    NightOwl,       // 0-5 AM
    EarlyBird,      // 5-7 AM
    WeekendWarrior, // Weekend
    LunchCoder,     // 12-13
    LateNight,      // 22-24

    // === STREAK (10) - Daily and success streaks ===
    Streak3,
    Streak7,
    Streak14,
    Streak30,
    Streak60,
    Streak90,
    Streak180,
    Streak365,
    SuccessStreak10,
    SuccessStreak25,

    // === TOKEN (15) - Total tokens processed ===
    Tokens10k,
    Tokens50k,
    Tokens100k,
    Tokens500k,
    Tokens1m,
    Tokens5m,
    Tokens10m,
    Tokens50m,
    Tokens100m,
    Tokens500m,
    Tokens1b,
    Tokens10b,
    Tokens100b,
    Tokens500b,
    Tokens1t,

    // === FILES (16) - File interactions ===
    Files10,
    Files50,
    Files100,
    Files500,
    Files1k,
    Files5k,
    Files10k,
    Files50k,
    Files100k,
    Files500k,
    Files1m,
    UniqueFiles50,
    UniqueFiles100,
    UniqueFiles500,
    UniqueFiles1k,
    UniqueFiles5k,

    // === TOOLS (16) - Tool usage ===
    ToolCalls100,
    ToolCalls500,
    ToolCalls1k,
    ToolCalls5k,
    ToolCalls10k,
    ToolCalls50k,
    ToolCalls100k,
    ToolCalls500k,
    ToolCalls1m,
    ToolCalls5m,
    ToolCalls10m,
    UniqueTools5,
    UniqueTools10,
    UniqueTools20,
    UniqueTools50,
    UniqueTools100,

    // === COST (12) - Money spent ===
    Spent1,
    Spent10,
    Spent50,
    Spent100,
    Spent500,
    Spent1000,
    Spent2500,
    Spent5000,
    Spent10000,
    Spent25000,
    Spent50000,
    Spent100000,

    // === LINES (12) - Lines of code changed ===
    Lines100,
    Lines500,
    Lines1k,
    Lines5k,
    Lines10k,
    Lines50k,
    Lines100k,
    Lines500k,
    Lines1m,
    Lines5m,
    Lines10m,
    Lines50m,

    // === DURATION (8) - Total time spent ===
    Duration1h,
    Duration10h,
    Duration100h,
    Duration1000h,
    Duration2500h,
    Duration5000h,
    Duration10000h,
    Duration25000h,

    // === SPECIAL (11) - Unique achievements ===
    FirstOfDay10,   // First job of day 10 times
    FirstOfDay50,   // First job of day 50 times
    Marathon,       // 10+ jobs in one day
    Prolific,       // 50+ jobs in one day
    Workhorse,      // 100+ jobs in one day
    PairProgrammer, // Use Claude + Codex in same day
    NewYear,        // Job on Jan 1st
    MidnightOil,    // Job at exactly midnight
    LuckySeven,     // 7 successful jobs 7 times in a row
    Dedication,     // Total 365 unique days
    TenKClub,       // 10,000 total successful jobs

    // === HIDDEN (15) - Secret achievements with cynical humor ===
    QueueOverlord,       // 200 jobs in queue at once
    FeetUp,              // 60min of work done in 6min (10x speed)
    SizeDoesntMatter,    // Run 17 modes with 300+ LOC
    StatStarer,          // View stats page for 10+ minutes in a session
    ModeHoarder,         // Create/use 50+ different modes
    ModelDiversity,      // Use 3+ different AI models
    CoffeeBreak,         // No activity for exactly 15 minutes then resume
    OopsAllErrors,       // 10 failed jobs in a row
    CtrlZHero,           // Reset achievements (why would you do this?)
    Overengineered,      // Chain with 10+ steps
    TokenBurner,         // 100k tokens in a single job
    RubberDuck,          // 50 jobs with < 100 LOC changes each
    YakShaving,          // 20 jobs without any commits
    CopyPasta,           // Same prompt used 5 times
    NightShift,          // 10 jobs between 2-4 AM

    // === WHISPER (10) - Subtle achievements, barely noticed ===
    SilentWorker,        // 100 jobs with no toast notifications
    MinimalFootprint,    // 10 jobs under 200 tokens each
    PatientOne,          // Wait 1 hour between jobs
    Methodical,          // Same mode 20 times in a row
    QuietNight,          // Single job after 11 PM, nothing else
    GhostMerge,          // Merge 5 jobs without looking at diffs
    ZenMaster,           // Complete job with 0 file changes
    Lurker,              // Open GUI 50 times without starting a job
    PerfectTiming,       // Job finishes at exactly :00 seconds
    TheWatcher,          // Check job status 100 times

    // === LOYALTY (4) - Agent preference achievements ===
    DarioFan,            // 200 more Claude jobs than Codex
    SamStan,             // 200 more Codex jobs than Claude
    Switzerland,         // Exactly equal Claude and Codex jobs (50+ each)
    Polygamous,          // Used all agents at least 100 times each
}

impl AchievementId {
    /// Get the string ID for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            // Milestone
            Self::FirstJob => "first_job",
            Self::TenJobs => "ten_jobs",
            Self::FiftyJobs => "fifty_jobs",
            Self::Century => "century",
            Self::TwoFifty => "two_fifty",
            Self::FiveHundred => "five_hundred",
            Self::Thousand => "thousand",
            Self::TwoThousandFive => "two_thousand_five",
            Self::FiveThousand => "five_thousand",
            Self::TenThousand => "ten_thousand",
            // Chain
            Self::FirstChain => "first_chain",
            Self::FiveChains => "five_chains",
            Self::TenChains => "ten_chains",
            Self::TwentyFiveChains => "twenty_five_chains",
            Self::FiftyChains => "fifty_chains",
            Self::HundredChains => "hundred_chains",
            // Mode
            Self::ModeNovice => "mode_novice",
            Self::Polyglot => "polyglot",
            Self::ModeExplorer => "mode_explorer",
            Self::ModeMaster => "mode_master",
            Self::ModeCollector => "mode_collector",
            // Agent
            Self::MultiAgent => "multi_agent",
            Self::TripleAgent => "triple_agent",
            Self::QuadAgent => "quad_agent",
            Self::AgentCollector => "agent_collector",
            // Skill
            Self::Flawless5 => "flawless_5",
            Self::Flawless10 => "flawless_10",
            Self::Flawless25 => "flawless_25",
            Self::Flawless50 => "flawless_50",
            Self::Flawless100 => "flawless_100",
            Self::SpeedDemon => "speed_demon",
            Self::LightningFast => "lightning_fast",
            Self::Instant => "instant",
            Self::TokenSaver => "token_saver",
            Self::Efficient => "efficient",
            // Time
            Self::NightOwl => "night_owl",
            Self::EarlyBird => "early_bird",
            Self::WeekendWarrior => "weekend_warrior",
            Self::LunchCoder => "lunch_coder",
            Self::LateNight => "late_night",
            // Streak
            Self::Streak3 => "streak_3",
            Self::Streak7 => "streak_7",
            Self::Streak14 => "streak_14",
            Self::Streak30 => "streak_30",
            Self::Streak60 => "streak_60",
            Self::Streak90 => "streak_90",
            Self::Streak180 => "streak_180",
            Self::Streak365 => "streak_365",
            Self::SuccessStreak10 => "success_streak_10",
            Self::SuccessStreak25 => "success_streak_25",
            // Token
            Self::Tokens10k => "tokens_10k",
            Self::Tokens50k => "tokens_50k",
            Self::Tokens100k => "tokens_100k",
            Self::Tokens500k => "tokens_500k",
            Self::Tokens1m => "tokens_1m",
            Self::Tokens5m => "tokens_5m",
            Self::Tokens10m => "tokens_10m",
            Self::Tokens50m => "tokens_50m",
            Self::Tokens100m => "tokens_100m",
            Self::Tokens500m => "tokens_500m",
            Self::Tokens1b => "tokens_1b",
            Self::Tokens10b => "tokens_10b",
            Self::Tokens100b => "tokens_100b",
            Self::Tokens500b => "tokens_500b",
            Self::Tokens1t => "tokens_1t",
            // Files
            Self::Files10 => "files_10",
            Self::Files50 => "files_50",
            Self::Files100 => "files_100",
            Self::Files500 => "files_500",
            Self::Files1k => "files_1k",
            Self::Files5k => "files_5k",
            Self::Files10k => "files_10k",
            Self::Files50k => "files_50k",
            Self::Files100k => "files_100k",
            Self::Files500k => "files_500k",
            Self::Files1m => "files_1m",
            Self::UniqueFiles50 => "unique_files_50",
            Self::UniqueFiles100 => "unique_files_100",
            Self::UniqueFiles500 => "unique_files_500",
            Self::UniqueFiles1k => "unique_files_1k",
            Self::UniqueFiles5k => "unique_files_5k",
            // Tools
            Self::ToolCalls100 => "tool_calls_100",
            Self::ToolCalls500 => "tool_calls_500",
            Self::ToolCalls1k => "tool_calls_1k",
            Self::ToolCalls5k => "tool_calls_5k",
            Self::ToolCalls10k => "tool_calls_10k",
            Self::ToolCalls50k => "tool_calls_50k",
            Self::ToolCalls100k => "tool_calls_100k",
            Self::ToolCalls500k => "tool_calls_500k",
            Self::ToolCalls1m => "tool_calls_1m",
            Self::ToolCalls5m => "tool_calls_5m",
            Self::ToolCalls10m => "tool_calls_10m",
            Self::UniqueTools5 => "unique_tools_5",
            Self::UniqueTools10 => "unique_tools_10",
            Self::UniqueTools20 => "unique_tools_20",
            Self::UniqueTools50 => "unique_tools_50",
            Self::UniqueTools100 => "unique_tools_100",
            // Cost
            Self::Spent1 => "spent_1",
            Self::Spent10 => "spent_10",
            Self::Spent50 => "spent_50",
            Self::Spent100 => "spent_100",
            Self::Spent500 => "spent_500",
            Self::Spent1000 => "spent_1000",
            Self::Spent2500 => "spent_2500",
            Self::Spent5000 => "spent_5000",
            Self::Spent10000 => "spent_10000",
            Self::Spent25000 => "spent_25000",
            Self::Spent50000 => "spent_50000",
            Self::Spent100000 => "spent_100000",
            // Lines
            Self::Lines100 => "lines_100",
            Self::Lines500 => "lines_500",
            Self::Lines1k => "lines_1k",
            Self::Lines5k => "lines_5k",
            Self::Lines10k => "lines_10k",
            Self::Lines50k => "lines_50k",
            Self::Lines100k => "lines_100k",
            Self::Lines500k => "lines_500k",
            Self::Lines1m => "lines_1m",
            Self::Lines5m => "lines_5m",
            Self::Lines10m => "lines_10m",
            Self::Lines50m => "lines_50m",
            // Duration
            Self::Duration1h => "duration_1h",
            Self::Duration10h => "duration_10h",
            Self::Duration100h => "duration_100h",
            Self::Duration1000h => "duration_1000h",
            Self::Duration2500h => "duration_2500h",
            Self::Duration5000h => "duration_5000h",
            Self::Duration10000h => "duration_10000h",
            Self::Duration25000h => "duration_25000h",
            // Special
            Self::FirstOfDay10 => "first_of_day_10",
            Self::FirstOfDay50 => "first_of_day_50",
            Self::Marathon => "marathon",
            Self::Prolific => "prolific",
            Self::Workhorse => "workhorse",
            Self::PairProgrammer => "pair_programmer",
            Self::NewYear => "new_year",
            Self::MidnightOil => "midnight_oil",
            Self::LuckySeven => "lucky_seven",
            Self::Dedication => "dedication",
            Self::TenKClub => "ten_k_club",
            // Hidden
            Self::QueueOverlord => "queue_overlord",
            Self::FeetUp => "feet_up",
            Self::SizeDoesntMatter => "size_doesnt_matter",
            Self::StatStarer => "stat_starer",
            Self::ModeHoarder => "mode_hoarder",
            Self::ModelDiversity => "model_diversity",
            Self::CoffeeBreak => "coffee_break",
            Self::OopsAllErrors => "oops_all_errors",
            Self::CtrlZHero => "ctrl_z_hero",
            Self::Overengineered => "overengineered",
            Self::TokenBurner => "token_burner",
            Self::RubberDuck => "rubber_duck",
            Self::YakShaving => "yak_shaving",
            Self::CopyPasta => "copy_pasta",
            Self::NightShift => "night_shift",
            // Whisper
            Self::SilentWorker => "silent_worker",
            Self::MinimalFootprint => "minimal_footprint",
            Self::PatientOne => "patient_one",
            Self::Methodical => "methodical",
            Self::QuietNight => "quiet_night",
            Self::GhostMerge => "ghost_merge",
            Self::ZenMaster => "zen_master",
            Self::Lurker => "lurker",
            Self::PerfectTiming => "perfect_timing",
            Self::TheWatcher => "the_watcher",
            // Loyalty
            Self::DarioFan => "dario_fan",
            Self::SamStan => "sam_stan",
            Self::Switzerland => "switzerland",
            Self::Polygamous => "polygamous",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // Milestone
            "first_job" => Some(Self::FirstJob),
            "ten_jobs" => Some(Self::TenJobs),
            "fifty_jobs" => Some(Self::FiftyJobs),
            "century" => Some(Self::Century),
            "two_fifty" => Some(Self::TwoFifty),
            "five_hundred" => Some(Self::FiveHundred),
            "thousand" => Some(Self::Thousand),
            "two_thousand_five" => Some(Self::TwoThousandFive),
            "five_thousand" => Some(Self::FiveThousand),
            "ten_thousand" => Some(Self::TenThousand),
            // Chain
            "first_chain" => Some(Self::FirstChain),
            "five_chains" => Some(Self::FiveChains),
            "ten_chains" => Some(Self::TenChains),
            "twenty_five_chains" => Some(Self::TwentyFiveChains),
            "fifty_chains" => Some(Self::FiftyChains),
            "hundred_chains" => Some(Self::HundredChains),
            // Mode
            "mode_novice" => Some(Self::ModeNovice),
            "polyglot" => Some(Self::Polyglot),
            "mode_explorer" => Some(Self::ModeExplorer),
            "mode_master" => Some(Self::ModeMaster),
            "mode_collector" => Some(Self::ModeCollector),
            // Agent
            "multi_agent" => Some(Self::MultiAgent),
            "triple_agent" => Some(Self::TripleAgent),
            "quad_agent" => Some(Self::QuadAgent),
            "agent_collector" => Some(Self::AgentCollector),
            // Skill
            "flawless_5" => Some(Self::Flawless5),
            "flawless_10" => Some(Self::Flawless10),
            "flawless_25" => Some(Self::Flawless25),
            "flawless_50" => Some(Self::Flawless50),
            "flawless_100" => Some(Self::Flawless100),
            "speed_demon" => Some(Self::SpeedDemon),
            "lightning_fast" => Some(Self::LightningFast),
            "instant" => Some(Self::Instant),
            "token_saver" => Some(Self::TokenSaver),
            "efficient" => Some(Self::Efficient),
            // Time
            "night_owl" => Some(Self::NightOwl),
            "early_bird" => Some(Self::EarlyBird),
            "weekend_warrior" => Some(Self::WeekendWarrior),
            "lunch_coder" => Some(Self::LunchCoder),
            "late_night" => Some(Self::LateNight),
            // Streak
            "streak_3" => Some(Self::Streak3),
            "streak_7" => Some(Self::Streak7),
            "streak_14" => Some(Self::Streak14),
            "streak_30" => Some(Self::Streak30),
            "streak_60" => Some(Self::Streak60),
            "streak_90" => Some(Self::Streak90),
            "streak_180" => Some(Self::Streak180),
            "streak_365" => Some(Self::Streak365),
            "success_streak_10" => Some(Self::SuccessStreak10),
            "success_streak_25" => Some(Self::SuccessStreak25),
            // Token
            "tokens_10k" => Some(Self::Tokens10k),
            "tokens_50k" => Some(Self::Tokens50k),
            "tokens_100k" => Some(Self::Tokens100k),
            "tokens_500k" => Some(Self::Tokens500k),
            "tokens_1m" => Some(Self::Tokens1m),
            "tokens_5m" => Some(Self::Tokens5m),
            "tokens_10m" => Some(Self::Tokens10m),
            "tokens_50m" => Some(Self::Tokens50m),
            "tokens_100m" => Some(Self::Tokens100m),
            "tokens_500m" => Some(Self::Tokens500m),
            "tokens_1b" => Some(Self::Tokens1b),
            "tokens_10b" => Some(Self::Tokens10b),
            "tokens_100b" => Some(Self::Tokens100b),
            "tokens_500b" => Some(Self::Tokens500b),
            "tokens_1t" => Some(Self::Tokens1t),
            // Files
            "files_10" => Some(Self::Files10),
            "files_50" => Some(Self::Files50),
            "files_100" => Some(Self::Files100),
            "files_500" => Some(Self::Files500),
            "files_1k" => Some(Self::Files1k),
            "files_5k" => Some(Self::Files5k),
            "files_10k" => Some(Self::Files10k),
            "files_50k" => Some(Self::Files50k),
            "files_100k" => Some(Self::Files100k),
            "files_500k" => Some(Self::Files500k),
            "files_1m" => Some(Self::Files1m),
            "unique_files_50" => Some(Self::UniqueFiles50),
            "unique_files_100" => Some(Self::UniqueFiles100),
            "unique_files_500" => Some(Self::UniqueFiles500),
            "unique_files_1k" => Some(Self::UniqueFiles1k),
            "unique_files_5k" => Some(Self::UniqueFiles5k),
            // Tools
            "tool_calls_100" => Some(Self::ToolCalls100),
            "tool_calls_500" => Some(Self::ToolCalls500),
            "tool_calls_1k" => Some(Self::ToolCalls1k),
            "tool_calls_5k" => Some(Self::ToolCalls5k),
            "tool_calls_10k" => Some(Self::ToolCalls10k),
            "tool_calls_50k" => Some(Self::ToolCalls50k),
            "tool_calls_100k" => Some(Self::ToolCalls100k),
            "tool_calls_500k" => Some(Self::ToolCalls500k),
            "tool_calls_1m" => Some(Self::ToolCalls1m),
            "tool_calls_5m" => Some(Self::ToolCalls5m),
            "tool_calls_10m" => Some(Self::ToolCalls10m),
            "unique_tools_5" => Some(Self::UniqueTools5),
            "unique_tools_10" => Some(Self::UniqueTools10),
            "unique_tools_20" => Some(Self::UniqueTools20),
            "unique_tools_50" => Some(Self::UniqueTools50),
            "unique_tools_100" => Some(Self::UniqueTools100),
            // Cost
            "spent_1" => Some(Self::Spent1),
            "spent_10" => Some(Self::Spent10),
            "spent_50" => Some(Self::Spent50),
            "spent_100" => Some(Self::Spent100),
            "spent_500" => Some(Self::Spent500),
            "spent_1000" => Some(Self::Spent1000),
            "spent_2500" => Some(Self::Spent2500),
            "spent_5000" => Some(Self::Spent5000),
            "spent_10000" => Some(Self::Spent10000),
            "spent_25000" => Some(Self::Spent25000),
            "spent_50000" => Some(Self::Spent50000),
            "spent_100000" => Some(Self::Spent100000),
            // Lines
            "lines_100" => Some(Self::Lines100),
            "lines_500" => Some(Self::Lines500),
            "lines_1k" => Some(Self::Lines1k),
            "lines_5k" => Some(Self::Lines5k),
            "lines_10k" => Some(Self::Lines10k),
            "lines_50k" => Some(Self::Lines50k),
            "lines_100k" => Some(Self::Lines100k),
            "lines_500k" => Some(Self::Lines500k),
            "lines_1m" => Some(Self::Lines1m),
            "lines_5m" => Some(Self::Lines5m),
            "lines_10m" => Some(Self::Lines10m),
            "lines_50m" => Some(Self::Lines50m),
            // Duration
            "duration_1h" => Some(Self::Duration1h),
            "duration_10h" => Some(Self::Duration10h),
            "duration_100h" => Some(Self::Duration100h),
            "duration_1000h" => Some(Self::Duration1000h),
            "duration_2500h" => Some(Self::Duration2500h),
            "duration_5000h" => Some(Self::Duration5000h),
            "duration_10000h" => Some(Self::Duration10000h),
            "duration_25000h" => Some(Self::Duration25000h),
            // Special
            "first_of_day_10" => Some(Self::FirstOfDay10),
            "first_of_day_50" => Some(Self::FirstOfDay50),
            "marathon" => Some(Self::Marathon),
            "prolific" => Some(Self::Prolific),
            "workhorse" => Some(Self::Workhorse),
            "pair_programmer" => Some(Self::PairProgrammer),
            "new_year" => Some(Self::NewYear),
            "midnight_oil" => Some(Self::MidnightOil),
            "lucky_seven" => Some(Self::LuckySeven),
            "dedication" => Some(Self::Dedication),
            "ten_k_club" => Some(Self::TenKClub),
            // Hidden
            "queue_overlord" => Some(Self::QueueOverlord),
            "feet_up" => Some(Self::FeetUp),
            "size_doesnt_matter" => Some(Self::SizeDoesntMatter),
            "stat_starer" => Some(Self::StatStarer),
            "mode_hoarder" => Some(Self::ModeHoarder),
            "model_diversity" => Some(Self::ModelDiversity),
            "coffee_break" => Some(Self::CoffeeBreak),
            "oops_all_errors" => Some(Self::OopsAllErrors),
            "ctrl_z_hero" => Some(Self::CtrlZHero),
            "overengineered" => Some(Self::Overengineered),
            "token_burner" => Some(Self::TokenBurner),
            "rubber_duck" => Some(Self::RubberDuck),
            "yak_shaving" => Some(Self::YakShaving),
            "copy_pasta" => Some(Self::CopyPasta),
            "night_shift" => Some(Self::NightShift),
            // Whisper
            "silent_worker" => Some(Self::SilentWorker),
            "minimal_footprint" => Some(Self::MinimalFootprint),
            "patient_one" => Some(Self::PatientOne),
            "methodical" => Some(Self::Methodical),
            "quiet_night" => Some(Self::QuietNight),
            "ghost_merge" => Some(Self::GhostMerge),
            "zen_master" => Some(Self::ZenMaster),
            "lurker" => Some(Self::Lurker),
            "perfect_timing" => Some(Self::PerfectTiming),
            "the_watcher" => Some(Self::TheWatcher),
            // Loyalty
            "dario_fan" => Some(Self::DarioFan),
            "sam_stan" => Some(Self::SamStan),
            "switzerland" => Some(Self::Switzerland),
            "polygamous" => Some(Self::Polygamous),
            _ => None,
        }
    }

    /// Get all achievement IDs
    pub fn all() -> &'static [AchievementId] {
        &[
            // Milestone
            Self::FirstJob,
            Self::TenJobs,
            Self::FiftyJobs,
            Self::Century,
            Self::TwoFifty,
            Self::FiveHundred,
            Self::Thousand,
            Self::TwoThousandFive,
            Self::FiveThousand,
            Self::TenThousand,
            // Chain
            Self::FirstChain,
            Self::FiveChains,
            Self::TenChains,
            Self::TwentyFiveChains,
            Self::FiftyChains,
            Self::HundredChains,
            // Mode
            Self::ModeNovice,
            Self::Polyglot,
            Self::ModeExplorer,
            Self::ModeMaster,
            Self::ModeCollector,
            // Agent
            Self::MultiAgent,
            Self::TripleAgent,
            Self::QuadAgent,
            Self::AgentCollector,
            // Skill
            Self::Flawless5,
            Self::Flawless10,
            Self::Flawless25,
            Self::Flawless50,
            Self::Flawless100,
            Self::SpeedDemon,
            Self::LightningFast,
            Self::Instant,
            Self::TokenSaver,
            Self::Efficient,
            // Time
            Self::NightOwl,
            Self::EarlyBird,
            Self::WeekendWarrior,
            Self::LunchCoder,
            Self::LateNight,
            // Streak
            Self::Streak3,
            Self::Streak7,
            Self::Streak14,
            Self::Streak30,
            Self::Streak60,
            Self::Streak90,
            Self::Streak180,
            Self::Streak365,
            Self::SuccessStreak10,
            Self::SuccessStreak25,
            // Token
            Self::Tokens10k,
            Self::Tokens50k,
            Self::Tokens100k,
            Self::Tokens500k,
            Self::Tokens1m,
            Self::Tokens5m,
            Self::Tokens10m,
            Self::Tokens50m,
            Self::Tokens100m,
            Self::Tokens500m,
            Self::Tokens1b,
            Self::Tokens10b,
            Self::Tokens100b,
            Self::Tokens500b,
            Self::Tokens1t,
            // Files
            Self::Files10,
            Self::Files50,
            Self::Files100,
            Self::Files500,
            Self::Files1k,
            Self::Files5k,
            Self::Files10k,
            Self::Files50k,
            Self::Files100k,
            Self::Files500k,
            Self::Files1m,
            Self::UniqueFiles50,
            Self::UniqueFiles100,
            Self::UniqueFiles500,
            Self::UniqueFiles1k,
            Self::UniqueFiles5k,
            // Tools
            Self::ToolCalls100,
            Self::ToolCalls500,
            Self::ToolCalls1k,
            Self::ToolCalls5k,
            Self::ToolCalls10k,
            Self::ToolCalls50k,
            Self::ToolCalls100k,
            Self::ToolCalls500k,
            Self::ToolCalls1m,
            Self::ToolCalls5m,
            Self::ToolCalls10m,
            Self::UniqueTools5,
            Self::UniqueTools10,
            Self::UniqueTools20,
            Self::UniqueTools50,
            Self::UniqueTools100,
            // Cost
            Self::Spent1,
            Self::Spent10,
            Self::Spent50,
            Self::Spent100,
            Self::Spent500,
            Self::Spent1000,
            Self::Spent2500,
            Self::Spent5000,
            Self::Spent10000,
            Self::Spent25000,
            Self::Spent50000,
            Self::Spent100000,
            // Lines
            Self::Lines100,
            Self::Lines500,
            Self::Lines1k,
            Self::Lines5k,
            Self::Lines10k,
            Self::Lines50k,
            Self::Lines100k,
            Self::Lines500k,
            Self::Lines1m,
            Self::Lines5m,
            Self::Lines10m,
            Self::Lines50m,
            // Duration
            Self::Duration1h,
            Self::Duration10h,
            Self::Duration100h,
            Self::Duration1000h,
            Self::Duration2500h,
            Self::Duration5000h,
            Self::Duration10000h,
            Self::Duration25000h,
            // Special
            Self::FirstOfDay10,
            Self::FirstOfDay50,
            Self::Marathon,
            Self::Prolific,
            Self::Workhorse,
            Self::PairProgrammer,
            Self::NewYear,
            Self::MidnightOil,
            Self::LuckySeven,
            Self::Dedication,
            Self::TenKClub,
            // Hidden
            Self::QueueOverlord,
            Self::FeetUp,
            Self::SizeDoesntMatter,
            Self::StatStarer,
            Self::ModeHoarder,
            Self::ModelDiversity,
            Self::CoffeeBreak,
            Self::OopsAllErrors,
            Self::CtrlZHero,
            Self::Overengineered,
            Self::TokenBurner,
            Self::RubberDuck,
            Self::YakShaving,
            Self::CopyPasta,
            Self::NightShift,
            // Whisper
            Self::SilentWorker,
            Self::MinimalFootprint,
            Self::PatientOne,
            Self::Methodical,
            Self::QuietNight,
            Self::GhostMerge,
            Self::ZenMaster,
            Self::Lurker,
            Self::PerfectTiming,
            Self::TheWatcher,
            // Loyalty
            Self::DarioFan,
            Self::SamStan,
            Self::Switzerland,
            Self::Polygamous,
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
    Token,
    Files,
    Tools,
    Cost,
    Lines,
    Duration,
    Special,
    Hidden,  // Secret achievements - not shown until unlocked
    Whisper, // Subtle achievements - minimal fanfare
    Loyalty, // Agent preference achievements
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
            Self::Token => "Tokens",
            Self::Files => "Files",
            Self::Tools => "Tools",
            Self::Cost => "Investment",
            Self::Lines => "Lines of Code",
            Self::Duration => "Time Spent",
            Self::Special => "Special",
            Self::Hidden => "???",
            Self::Whisper => "...",
            Self::Loyalty => "Loyalty",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Milestone => "🎯",
            Self::Chain => "🔗",
            Self::Mode => "🎭",
            Self::Agent => "🤖",
            Self::Skill => "⚡",
            Self::Time => "🕐",
            Self::Streak => "🔥",
            Self::Token => "🪙",
            Self::Files => "📁",
            Self::Tools => "🔧",
            Self::Cost => "💰",
            Self::Lines => "📝",
            Self::Duration => "⏱️",
            Self::Special => "⭐",
            Self::Hidden => "🔮",
            Self::Whisper => "🤫",
            Self::Loyalty => "💕",
        }
    }

    /// Whether achievements in this category are hidden until unlocked
    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Hidden)
    }

    /// Whether achievements in this category should have minimal notification
    pub fn is_quiet(&self) -> bool {
        matches!(self, Self::Whisper)
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

/// All 100 achievement definitions
pub static ACHIEVEMENTS: &[Achievement] = &[
    // ============================================================
    // MILESTONE (10) - Job completion milestones
    // ============================================================
    Achievement {
        id: AchievementId::FirstJob,
        name: "Baby's First Job",
        description: "Complete 1 job. Everyone starts somewhere, I guess.",
        icon: "🍼",
        category: AchievementCategory::Milestone,
        xp_reward: 10,
        target: Some(1),
    },
    Achievement {
        id: AchievementId::TenJobs,
        name: "Still Here?",
        description: "10 jobs done. Didn't expect you to stick around.",
        icon: "📈",
        category: AchievementCategory::Milestone,
        xp_reward: 25,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::FiftyJobs,
        name: "Getting Dependent",
        description: "50 jobs. Can you even code without AI anymore?",
        icon: "💉",
        category: AchievementCategory::Milestone,
        xp_reward: 50,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Century,
        name: "Century Club",
        description: "100 jobs. Your keyboard is getting jealous.",
        icon: "💯",
        category: AchievementCategory::Milestone,
        xp_reward: 100,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::TwoFifty,
        name: "Quarter Pounder",
        description: "250 jobs. That's a lot of prompts you could've typed yourself.",
        icon: "🍔",
        category: AchievementCategory::Milestone,
        xp_reward: 175,
        target: Some(250),
    },
    Achievement {
        id: AchievementId::FiveHundred,
        name: "Veteran Delegator",
        description: "500 jobs delegated. Management material.",
        icon: "🏅",
        category: AchievementCategory::Milestone,
        xp_reward: 250,
        target: Some(500),
    },
    Achievement {
        id: AchievementId::Thousand,
        name: "The Outsourcer",
        description: "1,000 jobs. Remember when you wrote code yourself?",
        icon: "🏆",
        category: AchievementCategory::Milestone,
        xp_reward: 500,
        target: Some(1000),
    },
    Achievement {
        id: AchievementId::TwoThousandFive,
        name: "Professional Prompter",
        description: "2,500 jobs. Put it on your LinkedIn.",
        icon: "💼",
        category: AchievementCategory::Milestone,
        xp_reward: 750,
        target: Some(2500),
    },
    Achievement {
        id: AchievementId::FiveThousand,
        name: "Human Supervisor",
        description: "5,000 jobs. You're basically a project manager now.",
        icon: "👔",
        category: AchievementCategory::Milestone,
        xp_reward: 1000,
        target: Some(5000),
    },
    Achievement {
        id: AchievementId::TenThousand,
        name: "AI Whisperer",
        description: "10,000 jobs. The machines obey your commands.",
        icon: "🧙",
        category: AchievementCategory::Milestone,
        xp_reward: 2000,
        target: Some(10000),
    },

    // ============================================================
    // CHAIN (6) - Chain completion milestones
    // ============================================================
    Achievement {
        id: AchievementId::FirstChain,
        name: "Chain Smoker",
        description: "First chain done. One job wasn't enough for you?",
        icon: "🔗",
        category: AchievementCategory::Chain,
        xp_reward: 25,
        target: Some(1),
    },
    Achievement {
        id: AchievementId::FiveChains,
        name: "Supply Chain",
        description: "5 chains. You're building dependencies.",
        icon: "⛓️",
        category: AchievementCategory::Chain,
        xp_reward: 50,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::TenChains,
        name: "Chain of Command",
        description: "10 chains. Who's the boss now?",
        icon: "🔱",
        category: AchievementCategory::Chain,
        xp_reward: 100,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::TwentyFiveChains,
        name: "Ball and Chain",
        description: "25 chains. Married to the workflow.",
        icon: "⚙️",
        category: AchievementCategory::Chain,
        xp_reward: 200,
        target: Some(25),
    },
    Achievement {
        id: AchievementId::FiftyChains,
        name: "Daisy Chain",
        description: "50 chains. It's chains all the way down.",
        icon: "🌼",
        category: AchievementCategory::Chain,
        xp_reward: 350,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::HundredChains,
        name: "Blockchain",
        description: "100 chains. No, not that kind. This one actually works.",
        icon: "💎",
        category: AchievementCategory::Chain,
        xp_reward: 500,
        target: Some(100),
    },

    // ============================================================
    // MODE (5) - Different modes used
    // ============================================================
    Achievement {
        id: AchievementId::ModeNovice,
        name: "Mode Curious",
        description: "3 different modes. Experimenting, are we?",
        icon: "🎪",
        category: AchievementCategory::Mode,
        xp_reward: 25,
        target: Some(3),
    },
    Achievement {
        id: AchievementId::Polyglot,
        name: "Indecisive",
        description: "5 modes. Can't pick a favorite?",
        icon: "🎭",
        category: AchievementCategory::Mode,
        xp_reward: 50,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::ModeExplorer,
        name: "Mode Safari",
        description: "10 modes explored. Some call it exploration, some call it procrastination.",
        icon: "🗺️",
        category: AchievementCategory::Mode,
        xp_reward: 100,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::ModeMaster,
        name: "Mode à la Mode",
        description: "15 modes. You're either thorough or scattered.",
        icon: "🍨",
        category: AchievementCategory::Mode,
        xp_reward: 175,
        target: Some(15),
    },
    Achievement {
        id: AchievementId::ModeCollector,
        name: "Gotta Catch 'Em All",
        description: "20 modes. Mode completionist detected.",
        icon: "🏛️",
        category: AchievementCategory::Mode,
        xp_reward: 250,
        target: Some(20),
    },

    // ============================================================
    // AGENT (4) - Different agents used
    // ============================================================
    Achievement {
        id: AchievementId::MultiAgent,
        name: "Cheater",
        description: "2 agents. Playing the field already?",
        icon: "🤖",
        category: AchievementCategory::Agent,
        xp_reward: 25,
        target: Some(2),
    },
    Achievement {
        id: AchievementId::TripleAgent,
        name: "Triple Agent",
        description: "3 agents. Your loyalty is... flexible.",
        icon: "🕵️",
        category: AchievementCategory::Agent,
        xp_reward: 50,
        target: Some(3),
    },
    Achievement {
        id: AchievementId::QuadAgent,
        name: "Agent Swap Meet",
        description: "4 agents. Keeping your options open?",
        icon: "🦾",
        category: AchievementCategory::Agent,
        xp_reward: 100,
        target: Some(4),
    },
    Achievement {
        id: AchievementId::AgentCollector,
        name: "Harem Protagonist",
        description: "5 agents. They all think they're your favorite.",
        icon: "💕",
        category: AchievementCategory::Agent,
        xp_reward: 200,
        target: Some(5),
    },

    // ============================================================
    // SKILL (10) - Performance achievements
    // ============================================================
    Achievement {
        id: AchievementId::Flawless5,
        name: "Beginner's Luck",
        description: "5 in a row without fails. Don't get cocky.",
        icon: "✨",
        category: AchievementCategory::Skill,
        xp_reward: 25,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::Flawless10,
        name: "Suspiciously Good",
        description: "10 in a row. Either you're good or the tasks are easy.",
        icon: "🌟",
        category: AchievementCategory::Skill,
        xp_reward: 75,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::Flawless25,
        name: "Control Freak",
        description: "25 perfect jobs. Let me guess, you review every line?",
        icon: "💎",
        category: AchievementCategory::Skill,
        xp_reward: 150,
        target: Some(25),
    },
    Achievement {
        id: AchievementId::Flawless50,
        name: "Trust Issues",
        description: "50 flawless. Still checking AI's homework, huh?",
        icon: "🛡️",
        category: AchievementCategory::Skill,
        xp_reward: 300,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Flawless100,
        name: "Impossible Standards",
        description: "100 perfect. Your therapist called, they're worried.",
        icon: "🏰",
        category: AchievementCategory::Skill,
        xp_reward: 500,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::SpeedDemon,
        name: "Impatient",
        description: "Done in 30 seconds. Blink and you'll miss it.",
        icon: "⚡",
        category: AchievementCategory::Skill,
        xp_reward: 50,
        target: None,
    },
    Achievement {
        id: AchievementId::LightningFast,
        name: "ADHD Mode",
        description: "15 seconds. Did you even read the output?",
        icon: "🌩️",
        category: AchievementCategory::Skill,
        xp_reward: 100,
        target: None,
    },
    Achievement {
        id: AchievementId::Instant,
        name: "Blink",
        description: "Under 10 seconds. Was that even a real task?",
        icon: "💨",
        category: AchievementCategory::Skill,
        xp_reward: 150,
        target: None,
    },
    Achievement {
        id: AchievementId::TokenSaver,
        name: "Cheapskate",
        description: "Under 500 tokens. Counting every penny, are we?",
        icon: "🪙",
        category: AchievementCategory::Skill,
        xp_reward: 25,
        target: None,
    },
    Achievement {
        id: AchievementId::Efficient,
        name: "Budget Conscious",
        description: "Under 1000 tokens. Your accountant would be proud.",
        icon: "📊",
        category: AchievementCategory::Skill,
        xp_reward: 15,
        target: None,
    },

    // ============================================================
    // TIME (5) - Time-based achievements
    // ============================================================
    Achievement {
        id: AchievementId::NightOwl,
        name: "Sleep is Overrated",
        description: "Coding at midnight-5 AM. Your circadian rhythm hates you.",
        icon: "🦉",
        category: AchievementCategory::Time,
        xp_reward: 15,
        target: None,
    },
    Achievement {
        id: AchievementId::EarlyBird,
        name: "Disgusting",
        description: "Coding before 7 AM. Are you okay?",
        icon: "🐦",
        category: AchievementCategory::Time,
        xp_reward: 15,
        target: None,
    },
    Achievement {
        id: AchievementId::WeekendWarrior,
        name: "No Life",
        description: "Working on weekends. Your friends miss you.",
        icon: "😢",
        category: AchievementCategory::Time,
        xp_reward: 10,
        target: None,
    },
    Achievement {
        id: AchievementId::LunchCoder,
        name: "Eating Optional",
        description: "Coding during lunch. The sandwich can wait.",
        icon: "🍕",
        category: AchievementCategory::Time,
        xp_reward: 10,
        target: None,
    },
    Achievement {
        id: AchievementId::LateNight,
        name: "One More Thing",
        description: "10 PM - midnight coding. Just one more job...",
        icon: "🌙",
        category: AchievementCategory::Time,
        xp_reward: 10,
        target: None,
    },

    // ============================================================
    // STREAK (10) - Daily and success streaks
    // ============================================================
    Achievement {
        id: AchievementId::Streak3,
        name: "Habit Forming",
        description: "3 days straight. The addiction begins.",
        icon: "🔥",
        category: AchievementCategory::Streak,
        xp_reward: 30,
        target: Some(3),
    },
    Achievement {
        id: AchievementId::Streak7,
        name: "Week of Sin",
        description: "7 days. One full week of outsourcing to AI.",
        icon: "📅",
        category: AchievementCategory::Streak,
        xp_reward: 75,
        target: Some(7),
    },
    Achievement {
        id: AchievementId::Streak14,
        name: "Fortnight Fanatic",
        description: "14 days. It's not a phase, mom.",
        icon: "🗓️",
        category: AchievementCategory::Streak,
        xp_reward: 150,
        target: Some(14),
    },
    Achievement {
        id: AchievementId::Streak30,
        name: "Monthly Subscription",
        description: "30 days. You're now a regular.",
        icon: "💳",
        category: AchievementCategory::Streak,
        xp_reward: 300,
        target: Some(30),
    },
    Achievement {
        id: AchievementId::Streak60,
        name: "Institutionalized",
        description: "60 days. This is your life now.",
        icon: "🔱",
        category: AchievementCategory::Streak,
        xp_reward: 500,
        target: Some(60),
    },
    Achievement {
        id: AchievementId::Streak90,
        name: "Quarterly Obsession",
        description: "90 days. Your family has filed a missing person report.",
        icon: "🏆",
        category: AchievementCategory::Streak,
        xp_reward: 750,
        target: Some(90),
    },
    Achievement {
        id: AchievementId::Streak180,
        name: "Half Year Hermit",
        description: "180 days. We're concerned about you.",
        icon: "🏚️",
        category: AchievementCategory::Streak,
        xp_reward: 1000,
        target: Some(180),
    },
    Achievement {
        id: AchievementId::Streak365,
        name: "Yearly Yikes",
        description: "365 days. One full year. Touch grass. Please.",
        icon: "🌿",
        category: AchievementCategory::Streak,
        xp_reward: 2000,
        target: Some(365),
    },
    Achievement {
        id: AchievementId::SuccessStreak10,
        name: "Getting Lucky",
        description: "10 wins straight. Let's see how long this lasts.",
        icon: "✅",
        category: AchievementCategory::Streak,
        xp_reward: 50,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::SuccessStreak25,
        name: "Statistically Improbable",
        description: "25 perfect jobs. Are you cheating?",
        icon: "🎲",
        category: AchievementCategory::Streak,
        xp_reward: 125,
        target: Some(25),
    },

    // ============================================================
    // TOKEN (7) - Total tokens processed
    // ============================================================
    Achievement {
        id: AchievementId::Tokens10k,
        name: "Baby's First Tokens",
        description: "10k tokens. That's like one medium prompt.",
        icon: "🪙",
        category: AchievementCategory::Token,
        xp_reward: 25,
        target: Some(10000),
    },
    Achievement {
        id: AchievementId::Tokens50k,
        name: "Token Nibbler",
        description: "50k tokens consumed. Getting hungry.",
        icon: "🍪",
        category: AchievementCategory::Token,
        xp_reward: 50,
        target: Some(50000),
    },
    Achievement {
        id: AchievementId::Tokens100k,
        name: "Token Snacker",
        description: "100k tokens. AI APIs are your favorite snack.",
        icon: "🍿",
        category: AchievementCategory::Token,
        xp_reward: 100,
        target: Some(100000),
    },
    Achievement {
        id: AchievementId::Tokens500k,
        name: "Token Glutton",
        description: "500k tokens. Slow down, you'll get indigestion.",
        icon: "🍔",
        category: AchievementCategory::Token,
        xp_reward: 200,
        target: Some(500000),
    },
    Achievement {
        id: AchievementId::Tokens1m,
        name: "Token Millionaire",
        description: "1 million tokens. Congrats, you're rich in all the wrong ways.",
        icon: "🤑",
        category: AchievementCategory::Token,
        xp_reward: 400,
        target: Some(1000000),
    },
    Achievement {
        id: AchievementId::Tokens5m,
        name: "Token Hoarder",
        description: "5 million tokens. That's a small novel's worth of AI thoughts.",
        icon: "📚",
        category: AchievementCategory::Token,
        xp_reward: 750,
        target: Some(5000000),
    },
    Achievement {
        id: AchievementId::Tokens10m,
        name: "Token Dragon",
        description: "10 million tokens. You could've written Lord of the Rings.",
        icon: "🐉",
        category: AchievementCategory::Token,
        xp_reward: 1000,
        target: Some(10000000),
    },
    Achievement {
        id: AchievementId::Tokens50m,
        name: "Token Titan",
        description: "50 million tokens. That's a library's worth of AI babble.",
        icon: "🦣",
        category: AchievementCategory::Token,
        xp_reward: 1500,
        target: Some(50000000),
    },
    Achievement {
        id: AchievementId::Tokens100m,
        name: "Token Tyrant",
        description: "100 million tokens. You're personally funding AI research.",
        icon: "👑",
        category: AchievementCategory::Token,
        xp_reward: 2000,
        target: Some(100000000),
    },
    Achievement {
        id: AchievementId::Tokens500m,
        name: "Token Tsunami",
        description: "500 million tokens. The datacenter knows you by name.",
        icon: "🌊",
        category: AchievementCategory::Token,
        xp_reward: 3000,
        target: Some(500000000),
    },
    Achievement {
        id: AchievementId::Tokens1b,
        name: "Billionaire (of Tokens)",
        description: "1 BILLION tokens. Dr. Evil would be proud.",
        icon: "🤑",
        category: AchievementCategory::Token,
        xp_reward: 5000,
        target: Some(1000000000),
    },
    Achievement {
        id: AchievementId::Tokens10b,
        name: "Token Black Hole",
        description: "10 billion tokens. You're consuming more than some countries.",
        icon: "🕳️",
        category: AchievementCategory::Token,
        xp_reward: 7500,
        target: None, // Exceeds u32, checked in checker with u64
    },
    Achievement {
        id: AchievementId::Tokens100b,
        name: "Token Singularity",
        description: "100 billion tokens. At this point, are YOU the AI?",
        icon: "💫",
        category: AchievementCategory::Token,
        xp_reward: 10000,
        target: None, // Exceeds u32, checked in checker with u64
    },
    Achievement {
        id: AchievementId::Tokens500b,
        name: "Token God",
        description: "500 billion tokens. Sam and Dario named a server room after you.",
        icon: "🛐",
        category: AchievementCategory::Token,
        xp_reward: 15000,
        target: None, // Exceeds u32, checked in checker with u64
    },
    Achievement {
        id: AchievementId::Tokens1t,
        name: "The Trillionaire",
        description: "1 TRILLION tokens. You ARE the training data now.",
        icon: "🌌",
        category: AchievementCategory::Token,
        xp_reward: 25000,
        target: None, // Exceeds u32, checked in checker with u64
    },

    // ============================================================
    // FILES (8) - File interactions
    // ============================================================
    Achievement {
        id: AchievementId::Files10,
        name: "File Toucher",
        description: "10 files touched. Just getting warmed up.",
        icon: "📁",
        category: AchievementCategory::Files,
        xp_reward: 15,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::Files50,
        name: "File Fondler",
        description: "50 files fondled. Getting handsy with the codebase.",
        icon: "📂",
        category: AchievementCategory::Files,
        xp_reward: 30,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Files100,
        name: "File Fiend",
        description: "100 files accessed. You've seen things.",
        icon: "🗂️",
        category: AchievementCategory::Files,
        xp_reward: 50,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::Files500,
        name: "File Stalker",
        description: "500 files. You know this codebase intimately.",
        icon: "🗃️",
        category: AchievementCategory::Files,
        xp_reward: 100,
        target: Some(500),
    },
    Achievement {
        id: AchievementId::Files1k,
        name: "File Creep",
        description: "1,000 files. No file is safe from your gaze.",
        icon: "👁️",
        category: AchievementCategory::Files,
        xp_reward: 200,
        target: Some(1000),
    },
    Achievement {
        id: AchievementId::Files5k,
        name: "File Omniscient",
        description: "5,000 files. You've achieved total codebase awareness.",
        icon: "🔮",
        category: AchievementCategory::Files,
        xp_reward: 400,
        target: Some(5000),
    },
    Achievement {
        id: AchievementId::UniqueFiles50,
        name: "File Sampler",
        description: "50 unique files. Variety is the spice of debugging.",
        icon: "🎰",
        category: AchievementCategory::Files,
        xp_reward: 50,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::UniqueFiles100,
        name: "File Tourist",
        description: "100 unique files visited. Taking the scenic route.",
        icon: "🗺️",
        category: AchievementCategory::Files,
        xp_reward: 100,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::Files10k,
        name: "File Hoarder",
        description: "10,000 file accesses. You touch everything.",
        icon: "📦",
        category: AchievementCategory::Files,
        xp_reward: 300,
        target: Some(10000),
    },
    Achievement {
        id: AchievementId::Files50k,
        name: "File Maniac",
        description: "50,000 files. Your SSD is filing for divorce.",
        icon: "💾",
        category: AchievementCategory::Files,
        xp_reward: 500,
        target: Some(50000),
    },
    Achievement {
        id: AchievementId::Files100k,
        name: "File Overlord",
        description: "100,000 file accesses. The filesystem bows to you.",
        icon: "👁️‍🗨️",
        category: AchievementCategory::Files,
        xp_reward: 750,
        target: Some(100000),
    },
    Achievement {
        id: AchievementId::Files500k,
        name: "File Deity",
        description: "500,000 files. You've read more files than some OSes have.",
        icon: "🏛️",
        category: AchievementCategory::Files,
        xp_reward: 1000,
        target: Some(500000),
    },
    Achievement {
        id: AchievementId::Files1m,
        name: "File Infinity",
        description: "1 MILLION file accesses. Achievement: OCD unlocked.",
        icon: "♾️",
        category: AchievementCategory::Files,
        xp_reward: 2000,
        target: Some(1000000),
    },
    Achievement {
        id: AchievementId::UniqueFiles500,
        name: "File Wanderer",
        description: "500 unique files. Getting lost in the codebase.",
        icon: "🧭",
        category: AchievementCategory::Files,
        xp_reward: 200,
        target: Some(500),
    },
    Achievement {
        id: AchievementId::UniqueFiles1k,
        name: "File Explorer Pro",
        description: "1,000 unique files. Is there a file you haven't seen?",
        icon: "🔭",
        category: AchievementCategory::Files,
        xp_reward: 400,
        target: Some(1000),
    },
    Achievement {
        id: AchievementId::UniqueFiles5k,
        name: "File Archaeologist",
        description: "5,000 unique files. You've excavated the entire repo.",
        icon: "⛏️",
        category: AchievementCategory::Files,
        xp_reward: 750,
        target: Some(5000),
    },

    // ============================================================
    // TOOLS (16) - Tool usage
    // ============================================================
    Achievement {
        id: AchievementId::ToolCalls100,
        name: "Tool Dabbler",
        description: "100 tool calls. Just learning the ropes.",
        icon: "🔧",
        category: AchievementCategory::Tools,
        xp_reward: 25,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::ToolCalls500,
        name: "Tool Tinkerer",
        description: "500 calls. You've found your favorites.",
        icon: "🛠️",
        category: AchievementCategory::Tools,
        xp_reward: 50,
        target: Some(500),
    },
    Achievement {
        id: AchievementId::ToolCalls1k,
        name: "Tool Junkie",
        description: "1,000 tool calls. It's becoming a problem.",
        icon: "⚙️",
        category: AchievementCategory::Tools,
        xp_reward: 100,
        target: Some(1000),
    },
    Achievement {
        id: AchievementId::ToolCalls5k,
        name: "Tool Addict",
        description: "5,000 calls. Intervention may be needed.",
        icon: "💉",
        category: AchievementCategory::Tools,
        xp_reward: 200,
        target: Some(5000),
    },
    Achievement {
        id: AchievementId::ToolCalls10k,
        name: "Tool Maniac",
        description: "10,000 tool calls. This is unhealthy.",
        icon: "🤪",
        category: AchievementCategory::Tools,
        xp_reward: 400,
        target: Some(10000),
    },
    Achievement {
        id: AchievementId::ToolCalls50k,
        name: "Tool Transcendent",
        description: "50,000 calls. You've become one with the tools.",
        icon: "🧘",
        category: AchievementCategory::Tools,
        xp_reward: 750,
        target: Some(50000),
    },
    Achievement {
        id: AchievementId::UniqueTools5,
        name: "Tool Curious",
        description: "5 different tools. Testing the waters.",
        icon: "🧰",
        category: AchievementCategory::Tools,
        xp_reward: 25,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::UniqueTools10,
        name: "Swiss Army Dev",
        description: "10 unique tools. One for every occasion.",
        icon: "🇨🇭",
        category: AchievementCategory::Tools,
        xp_reward: 75,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::ToolCalls100k,
        name: "Tool Obsessed",
        description: "100,000 tool calls. The tools are using YOU now.",
        icon: "🔩",
        category: AchievementCategory::Tools,
        xp_reward: 600,
        target: Some(100000),
    },
    Achievement {
        id: AchievementId::ToolCalls500k,
        name: "Tool Hurricane",
        description: "500,000 tool calls. A storm of productivity... or procrastination.",
        icon: "🌀",
        category: AchievementCategory::Tools,
        xp_reward: 1000,
        target: Some(500000),
    },
    Achievement {
        id: AchievementId::ToolCalls1m,
        name: "Tool Millionaire",
        description: "1 MILLION tool calls. If only each call was worth a penny.",
        icon: "💎",
        category: AchievementCategory::Tools,
        xp_reward: 2000,
        target: Some(1000000),
    },
    Achievement {
        id: AchievementId::ToolCalls5m,
        name: "Tool Black Hole",
        description: "5 million tool calls. Nothing escapes your toolbox.",
        icon: "🕳️",
        category: AchievementCategory::Tools,
        xp_reward: 4000,
        target: Some(5000000),
    },
    Achievement {
        id: AchievementId::ToolCalls10m,
        name: "Tool Singularity",
        description: "10 MILLION tool calls. The AI is the tool now.",
        icon: "🌌",
        category: AchievementCategory::Tools,
        xp_reward: 7500,
        target: Some(10000000),
    },
    Achievement {
        id: AchievementId::UniqueTools20,
        name: "Tool Collector",
        description: "20 unique tools. Starting a museum?",
        icon: "🏺",
        category: AchievementCategory::Tools,
        xp_reward: 150,
        target: Some(20),
    },
    Achievement {
        id: AchievementId::UniqueTools50,
        name: "Tool Hoarder",
        description: "50 unique tools. You have a tool for everything.",
        icon: "🗄️",
        category: AchievementCategory::Tools,
        xp_reward: 300,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::UniqueTools100,
        name: "Tool Sommelier",
        description: "100 unique tools. May I suggest the Edit with a hint of Bash?",
        icon: "🍷",
        category: AchievementCategory::Tools,
        xp_reward: 500,
        target: Some(100),
    },

    // ============================================================
    // COST (6) - Investment achievements
    // ============================================================
    Achievement {
        id: AchievementId::Spent1,
        name: "First Blood",
        description: "$1 spent. Your wallet has been breached.",
        icon: "🩸",
        category: AchievementCategory::Cost,
        xp_reward: 10,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent10,
        name: "Minor Investor",
        description: "$10 down. It's just coffee money, right?",
        icon: "☕",
        category: AchievementCategory::Cost,
        xp_reward: 25,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent50,
        name: "Getting Serious",
        description: "$50 invested. That's a nice dinner for two.",
        icon: "🍷",
        category: AchievementCategory::Cost,
        xp_reward: 75,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent100,
        name: "Benjamin's Gone",
        description: "$100 spent. A hundred bucks to not think.",
        icon: "💸",
        category: AchievementCategory::Cost,
        xp_reward: 150,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent500,
        name: "Rent Money",
        description: "$500 in AI. Hope it's worth it.",
        icon: "🏠",
        category: AchievementCategory::Cost,
        xp_reward: 350,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent1000,
        name: "API Whale",
        description: "$1,000 spent. Sam and Dario thank you for your service.",
        icon: "🐋",
        category: AchievementCategory::Cost,
        xp_reward: 500,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent2500,
        name: "Small Investor",
        description: "$2,500 spent. That's a used car. Or a lot of AI.",
        icon: "🚗",
        category: AchievementCategory::Cost,
        xp_reward: 750,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent5000,
        name: "AI Sugar Daddy",
        description: "$5,000 spent. The AIs fight over who gets your prompts.",
        icon: "💰",
        category: AchievementCategory::Cost,
        xp_reward: 1000,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent10000,
        name: "Five Figures",
        description: "$10,000 spent. You could've bought a motorcycle.",
        icon: "🏍️",
        category: AchievementCategory::Cost,
        xp_reward: 1500,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent25000,
        name: "AI Venture Capitalist",
        description: "$25,000 spent. You're basically a Series A investor now.",
        icon: "📈",
        category: AchievementCategory::Cost,
        xp_reward: 2500,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent50000,
        name: "Down Payment",
        description: "$50,000 spent. That's a house deposit. Hope the code was worth it.",
        icon: "🏠",
        category: AchievementCategory::Cost,
        xp_reward: 5000,
        target: None,
    },
    Achievement {
        id: AchievementId::Spent100000,
        name: "The One Percent",
        description: "$100,000 spent. Anthropic and OpenAI are sending you holiday cards.",
        icon: "💎",
        category: AchievementCategory::Cost,
        xp_reward: 10000,
        target: None,
    },

    // ============================================================
    // LINES (12) - Lines of code changed
    // ============================================================
    Achievement {
        id: AchievementId::Lines100,
        name: "First Scribbles",
        description: "100 lines changed. Baby's first refactor.",
        icon: "📝",
        category: AchievementCategory::Lines,
        xp_reward: 15,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::Lines500,
        name: "Line Cook",
        description: "500 lines. Someone's been busy.",
        icon: "👨‍🍳",
        category: AchievementCategory::Lines,
        xp_reward: 30,
        target: Some(500),
    },
    Achievement {
        id: AchievementId::Lines1k,
        name: "Line Manager",
        description: "1,000 lines changed. Not by you, technically.",
        icon: "📋",
        category: AchievementCategory::Lines,
        xp_reward: 50,
        target: Some(1000),
    },
    Achievement {
        id: AchievementId::Lines5k,
        name: "Line Factory",
        description: "5,000 lines. Quantity over quality?",
        icon: "🏭",
        category: AchievementCategory::Lines,
        xp_reward: 100,
        target: Some(5000),
    },
    Achievement {
        id: AchievementId::Lines10k,
        name: "Line Printer",
        description: "10,000 lines. Brrrrr goes the AI.",
        icon: "🖨️",
        category: AchievementCategory::Lines,
        xp_reward: 200,
        target: Some(10000),
    },
    Achievement {
        id: AchievementId::Lines50k,
        name: "Line Tsunami",
        description: "50,000 lines. That's a lot of generated code to maintain.",
        icon: "🌊",
        category: AchievementCategory::Lines,
        xp_reward: 500,
        target: Some(50000),
    },
    Achievement {
        id: AchievementId::Lines100k,
        name: "Line Hurricane",
        description: "100,000 lines. A category 5 code storm.",
        icon: "🌀",
        category: AchievementCategory::Lines,
        xp_reward: 1000,
        target: Some(100000),
    },
    Achievement {
        id: AchievementId::Lines500k,
        name: "Line Apocalypse",
        description: "500,000 lines changed. Entire codebases tremble.",
        icon: "☄️",
        category: AchievementCategory::Lines,
        xp_reward: 2000,
        target: Some(500000),
    },
    Achievement {
        id: AchievementId::Lines1m,
        name: "Line Millionaire",
        description: "1 MILLION lines. You've rewritten Windows by now.",
        icon: "🏆",
        category: AchievementCategory::Lines,
        xp_reward: 4000,
        target: Some(1000000),
    },
    Achievement {
        id: AchievementId::Lines5m,
        name: "Line Factory",
        description: "5 million lines. That's a small operating system.",
        icon: "🏭",
        category: AchievementCategory::Lines,
        xp_reward: 7500,
        target: Some(5000000),
    },
    Achievement {
        id: AchievementId::Lines10m,
        name: "Line Universe",
        description: "10 million lines. You've created more code than most companies.",
        icon: "🌌",
        category: AchievementCategory::Lines,
        xp_reward: 10000,
        target: Some(10000000),
    },
    Achievement {
        id: AchievementId::Lines50m,
        name: "Line God",
        description: "50 MILLION lines. The code has become self-aware.",
        icon: "🤖",
        category: AchievementCategory::Lines,
        xp_reward: 20000,
        target: Some(50000000),
    },

    // ============================================================
    // DURATION (8) - Total time spent
    // ============================================================
    Achievement {
        id: AchievementId::Duration1h,
        name: "Hour Waster",
        description: "1 hour of job time. Time flies when AI's working.",
        icon: "⏱️",
        category: AchievementCategory::Duration,
        xp_reward: 25,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration10h,
        name: "Day Job",
        description: "10 hours. That's a full workday... of waiting.",
        icon: "⏰",
        category: AchievementCategory::Duration,
        xp_reward: 75,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration100h,
        name: "Time Sink",
        description: "100 hours of AI time. What have you been doing?",
        icon: "🕳️",
        category: AchievementCategory::Duration,
        xp_reward: 250,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration1000h,
        name: "Temporal Void",
        description: "1,000 hours. That's 41 days of AI computation. On your prompts.",
        icon: "🌌",
        category: AchievementCategory::Duration,
        xp_reward: 1000,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration2500h,
        name: "Time Traveler",
        description: "2,500 hours. That's 104 days. You've lost track of reality.",
        icon: "⏳",
        category: AchievementCategory::Duration,
        xp_reward: 2500,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration5000h,
        name: "Time Lord",
        description: "5,000 hours. 208 days of AI time. The TARDIS is jealous.",
        icon: "🕰️",
        category: AchievementCategory::Duration,
        xp_reward: 5000,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration10000h,
        name: "10k Hour Master",
        description: "10,000 hours. Malcolm Gladwell would be proud. Or horrified.",
        icon: "🎓",
        category: AchievementCategory::Duration,
        xp_reward: 10000,
        target: None,
    },
    Achievement {
        id: AchievementId::Duration25000h,
        name: "Eternal Prompter",
        description: "25,000 hours. That's almost 3 years of AI time. On YOUR tasks.",
        icon: "♾️",
        category: AchievementCategory::Duration,
        xp_reward: 25000,
        target: None,
    },

    // ============================================================
    // SPECIAL (11) - Unique achievements
    // ============================================================
    Achievement {
        id: AchievementId::FirstOfDay10,
        name: "Early Worm",
        description: "First job of the day 10 times. The early bird gets the... bugs?",
        icon: "🐛",
        category: AchievementCategory::Special,
        xp_reward: 50,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::FirstOfDay50,
        name: "Morning Ritual",
        description: "50 morning firsts. AI before coffee, impressive.",
        icon: "☀️",
        category: AchievementCategory::Special,
        xp_reward: 150,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Marathon,
        name: "Touch Grass Later",
        description: "10+ jobs in one day. Outside can wait.",
        icon: "🏃",
        category: AchievementCategory::Special,
        xp_reward: 50,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::Prolific,
        name: "Hyperfixation",
        description: "50+ jobs in one day. Are you okay? Seriously.",
        icon: "🚀",
        category: AchievementCategory::Special,
        xp_reward: 200,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Workhorse,
        name: "Masochist",
        description: "100+ jobs in ONE DAY. Please seek help.",
        icon: "🐎",
        category: AchievementCategory::Special,
        xp_reward: 500,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::PairProgrammer,
        name: "Two-Timer",
        description: "Claude AND Codex in one day. Playing both sides?",
        icon: "👥",
        category: AchievementCategory::Special,
        xp_reward: 50,
        target: None,
    },
    Achievement {
        id: AchievementId::NewYear,
        name: "No Resolution",
        description: "Coding on Jan 1st. What happened to 'work-life balance'?",
        icon: "🎆",
        category: AchievementCategory::Special,
        xp_reward: 100,
        target: None,
    },
    Achievement {
        id: AchievementId::MidnightOil,
        name: "Exactly Midnight",
        description: "Job finished at 00:00:00. Spooky timing.",
        icon: "🕛",
        category: AchievementCategory::Special,
        xp_reward: 25,
        target: None,
    },
    Achievement {
        id: AchievementId::LuckySeven,
        name: "Superstitious",
        description: "7 wins, 7 times. You believe in luck?",
        icon: "🎰",
        category: AchievementCategory::Special,
        xp_reward: 77,
        target: Some(49),
    },
    Achievement {
        id: AchievementId::Dedication,
        name: "Full Year Freak",
        description: "365 different days. You literally didn't miss a day.",
        icon: "📆",
        category: AchievementCategory::Special,
        xp_reward: 1000,
        target: Some(365),
    },
    Achievement {
        id: AchievementId::TenKClub,
        name: "Probably a Bot",
        description: "10,000 jobs. Are you sure you're human?",
        icon: "🤖",
        category: AchievementCategory::Special,
        xp_reward: 2500,
        target: Some(10000),
    },

    // ============================================================
    // HIDDEN (15) - Secret achievements with cynical humor
    // These are not shown in the UI until unlocked
    // ============================================================
    Achievement {
        id: AchievementId::QueueOverlord,
        name: "Queue Overlord",
        description: "Have 200 jobs in queue. Someone's feeling ambitious today.",
        icon: "👑",
        category: AchievementCategory::Hidden,
        xp_reward: 200,
        target: Some(200),
    },
    Achievement {
        id: AchievementId::FeetUp,
        name: "Feet Up",
        description: "Let AI do 60min of work in 6min. Time for coffee.",
        icon: "🦥",
        category: AchievementCategory::Hidden,
        xp_reward: 150,
        target: None,
    },
    Achievement {
        id: AchievementId::SizeDoesntMatter,
        name: "Size Doesn't Matter",
        description: "Run 17 modes with 300+ LOC each. Overcompensating much?",
        icon: "📏",
        category: AchievementCategory::Hidden,
        xp_reward: 170,
        target: Some(17),
    },
    Achievement {
        id: AchievementId::StatStarer,
        name: "Stat Starer",
        description: "Stare at the stats page for 10+ minutes. Hypnotizing, isn't it?",
        icon: "👁️",
        category: AchievementCategory::Hidden,
        xp_reward: 42,
        target: None,
    },
    Achievement {
        id: AchievementId::ModeHoarder,
        name: "Mode Hoarder",
        description: "Create 50+ modes. You do know you can reuse them, right?",
        icon: "🐿️",
        category: AchievementCategory::Hidden,
        xp_reward: 250,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::ModelDiversity,
        name: "Model Diversity",
        description: "Use 3+ different AI models. Can't decide on a favorite?",
        icon: "🎰",
        category: AchievementCategory::Hidden,
        xp_reward: 75,
        target: Some(3),
    },
    Achievement {
        id: AchievementId::CoffeeBreak,
        name: "Coffee Break",
        description: "Exactly 15min idle then resume. Perfectly timed procrastination.",
        icon: "☕",
        category: AchievementCategory::Hidden,
        xp_reward: 15,
        target: None,
    },
    Achievement {
        id: AchievementId::OopsAllErrors,
        name: "Oops! All Errors",
        description: "10 failed jobs in a row. AI having a bad day or you?",
        icon: "💥",
        category: AchievementCategory::Hidden,
        xp_reward: 50,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::CtrlZHero,
        name: "Ctrl+Z Hero",
        description: "Reset all achievements. Starting over is always an option.",
        icon: "🔄",
        category: AchievementCategory::Hidden,
        xp_reward: 1,
        target: None,
    },
    Achievement {
        id: AchievementId::Overengineered,
        name: "Overengineered",
        description: "Chain with 10+ steps. Enterprise architecture vibes.",
        icon: "🏗️",
        category: AchievementCategory::Hidden,
        xp_reward: 100,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::TokenBurner,
        name: "Token Burner",
        description: "100k tokens in a single job. Anthropic thanks your wallet.",
        icon: "🔥",
        category: AchievementCategory::Hidden,
        xp_reward: 100,
        target: None,
    },
    Achievement {
        id: AchievementId::RubberDuck,
        name: "Rubber Duck",
        description: "50 jobs with < 100 LOC changes each. Small steps, big journey.",
        icon: "🦆",
        category: AchievementCategory::Hidden,
        xp_reward: 100,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::YakShaving,
        name: "Yak Shaving",
        description: "20 jobs without committing. What exactly are you building?",
        icon: "🦬",
        category: AchievementCategory::Hidden,
        xp_reward: 75,
        target: Some(20),
    },
    Achievement {
        id: AchievementId::CopyPasta,
        name: "Copy Pasta",
        description: "Same prompt 5 times. Insanity: doing the same thing...",
        icon: "🍝",
        category: AchievementCategory::Hidden,
        xp_reward: 25,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::NightShift,
        name: "Night Shift",
        description: "10 jobs between 2-4 AM. Deadlines or insomnia?",
        icon: "🌃",
        category: AchievementCategory::Hidden,
        xp_reward: 100,
        target: Some(10),
    },

    // ============================================================
    // WHISPER (10) - Subtle achievements, minimal fanfare
    // These unlock quietly without much celebration
    // ============================================================
    Achievement {
        id: AchievementId::SilentWorker,
        name: "Silent Worker",
        description: "100 jobs, no celebration needed",
        icon: "🤫",
        category: AchievementCategory::Whisper,
        xp_reward: 50,
        target: Some(100),
    },
    Achievement {
        id: AchievementId::MinimalFootprint,
        name: "Minimal Footprint",
        description: "10 jobs under 200 tokens each",
        icon: "👣",
        category: AchievementCategory::Whisper,
        xp_reward: 25,
        target: Some(10),
    },
    Achievement {
        id: AchievementId::PatientOne,
        name: "Patient One",
        description: "An hour between jobs",
        icon: "⏳",
        category: AchievementCategory::Whisper,
        xp_reward: 10,
        target: None,
    },
    Achievement {
        id: AchievementId::Methodical,
        name: "Methodical",
        description: "Same mode, 20 times straight",
        icon: "📐",
        category: AchievementCategory::Whisper,
        xp_reward: 40,
        target: Some(20),
    },
    Achievement {
        id: AchievementId::QuietNight,
        name: "Quiet Night",
        description: "One late job, then silence",
        icon: "🌙",
        category: AchievementCategory::Whisper,
        xp_reward: 15,
        target: None,
    },
    Achievement {
        id: AchievementId::GhostMerge,
        name: "Ghost Merge",
        description: "5 merges, no questions asked",
        icon: "👻",
        category: AchievementCategory::Whisper,
        xp_reward: 25,
        target: Some(5),
    },
    Achievement {
        id: AchievementId::ZenMaster,
        name: "Zen Master",
        description: "Job complete, nothing changed",
        icon: "🧘",
        category: AchievementCategory::Whisper,
        xp_reward: 20,
        target: None,
    },
    Achievement {
        id: AchievementId::Lurker,
        name: "Lurker",
        description: "50 GUI opens, zero jobs",
        icon: "👀",
        category: AchievementCategory::Whisper,
        xp_reward: 15,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::PerfectTiming,
        name: "Perfect Timing",
        description: "Finished at :00",
        icon: "⏰",
        category: AchievementCategory::Whisper,
        xp_reward: 10,
        target: None,
    },
    Achievement {
        id: AchievementId::TheWatcher,
        name: "The Watcher",
        description: "100 status checks",
        icon: "🔭",
        category: AchievementCategory::Whisper,
        xp_reward: 30,
        target: Some(100),
    },

    // ============================================================
    // LOYALTY (4) - Agent preference achievements
    // ============================================================
    Achievement {
        id: AchievementId::DarioFan,
        name: "Dario's Disciple",
        description: "200 more Claude jobs than Codex. Constitutional AI appreciates your loyalty.",
        icon: "🧠",
        category: AchievementCategory::Loyalty,
        xp_reward: 100,
        target: Some(200),
    },
    Achievement {
        id: AchievementId::SamStan,
        name: "Sam's Soldier",
        description: "200 more Codex jobs than Claude. AGI will remember your service.",
        icon: "🚀",
        category: AchievementCategory::Loyalty,
        xp_reward: 100,
        target: Some(200),
    },
    Achievement {
        id: AchievementId::Switzerland,
        name: "Switzerland",
        description: "Exactly equal Claude & Codex jobs (50+ each). Diplomatic.",
        icon: "🇨🇭",
        category: AchievementCategory::Loyalty,
        xp_reward: 150,
        target: Some(50),
    },
    Achievement {
        id: AchievementId::Polygamous,
        name: "Polygamous",
        description: "100+ jobs with each agent. Commitment issues?",
        icon: "💔",
        category: AchievementCategory::Loyalty,
        xp_reward: 200,
        target: Some(100),
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

    /// Get achievements by category
    pub fn by_category(category: AchievementCategory) -> Vec<&'static Achievement> {
        ACHIEVEMENTS
            .iter()
            .filter(|a| a.category == category)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_count() {
        assert_eq!(ACHIEVEMENTS.len(), 169, "Should have exactly 169 achievements");
    }

    #[test]
    fn test_all_ids_unique() {
        let mut ids: Vec<_> = AchievementId::all().iter().map(|id| id.as_str()).collect();
        ids.sort();
        let unique_count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique_count, "All achievement IDs should be unique");
    }

    #[test]
    fn test_all_ids_have_definitions() {
        for id in AchievementId::all() {
            let _ = Achievement::get(*id);
        }
    }

    #[test]
    fn test_category_counts() {
        assert_eq!(Achievement::by_category(AchievementCategory::Milestone).len(), 10);
        assert_eq!(Achievement::by_category(AchievementCategory::Chain).len(), 6);
        assert_eq!(Achievement::by_category(AchievementCategory::Mode).len(), 5);
        assert_eq!(Achievement::by_category(AchievementCategory::Agent).len(), 4);
        assert_eq!(Achievement::by_category(AchievementCategory::Skill).len(), 10);
        assert_eq!(Achievement::by_category(AchievementCategory::Time).len(), 5);
        assert_eq!(Achievement::by_category(AchievementCategory::Streak).len(), 10);
        assert_eq!(Achievement::by_category(AchievementCategory::Token).len(), 15);
        assert_eq!(Achievement::by_category(AchievementCategory::Files).len(), 16);
        assert_eq!(Achievement::by_category(AchievementCategory::Tools).len(), 16);
        assert_eq!(Achievement::by_category(AchievementCategory::Cost).len(), 12);
        assert_eq!(Achievement::by_category(AchievementCategory::Lines).len(), 12);
        assert_eq!(Achievement::by_category(AchievementCategory::Duration).len(), 8);
        assert_eq!(Achievement::by_category(AchievementCategory::Special).len(), 11);
        assert_eq!(Achievement::by_category(AchievementCategory::Hidden).len(), 15);
        assert_eq!(Achievement::by_category(AchievementCategory::Whisper).len(), 10);
        assert_eq!(Achievement::by_category(AchievementCategory::Loyalty).len(), 4);
    }

    #[test]
    fn test_hidden_achievements_are_secret() {
        for achievement in Achievement::by_category(AchievementCategory::Hidden) {
            assert!(achievement.category.is_secret(), "Hidden achievements should be secret");
        }
    }

    #[test]
    fn test_whisper_achievements_are_quiet() {
        for achievement in Achievement::by_category(AchievementCategory::Whisper) {
            assert!(achievement.category.is_quiet(), "Whisper achievements should be quiet");
        }
    }

    #[test]
    fn test_total_xp() {
        let total = Achievement::total_xp();
        println!("Total possible XP from achievements: {}", total);
        assert!(total > 10000, "Total XP should be substantial");
    }
}
