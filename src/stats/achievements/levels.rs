//! XP and Level system
//!
//! Defines level thresholds, titles, and XP calculations.

/// Level definition
#[derive(Debug, Clone)]
pub struct Level {
    pub level: u32,
    pub xp_required: u32,
    pub title: &'static str,
}

/// All level definitions (must be sorted by level)
pub static LEVELS: &[Level] = &[
    Level {
        level: 1,
        xp_required: 0,
        title: "Apprentice",
    },
    Level {
        level: 2,
        xp_required: 50,
        title: "Novice",
    },
    Level {
        level: 3,
        xp_required: 150,
        title: "Junior Dev",
    },
    Level {
        level: 4,
        xp_required: 300,
        title: "Junior Dev",
    },
    Level {
        level: 5,
        xp_required: 500,
        title: "Developer",
    },
    Level {
        level: 6,
        xp_required: 750,
        title: "Developer",
    },
    Level {
        level: 7,
        xp_required: 1000,
        title: "Senior Dev",
    },
    Level {
        level: 8,
        xp_required: 1300,
        title: "Senior Dev",
    },
    Level {
        level: 9,
        xp_required: 1600,
        title: "Senior Dev",
    },
    Level {
        level: 10,
        xp_required: 2000,
        title: "Expert",
    },
    Level {
        level: 11,
        xp_required: 2500,
        title: "Expert",
    },
    Level {
        level: 12,
        xp_required: 3000,
        title: "Expert",
    },
    Level {
        level: 13,
        xp_required: 3500,
        title: "Master",
    },
    Level {
        level: 14,
        xp_required: 4000,
        title: "Master",
    },
    Level {
        level: 15,
        xp_required: 5000,
        title: "Master",
    },
    Level {
        level: 16,
        xp_required: 6000,
        title: "Grandmaster",
    },
    Level {
        level: 17,
        xp_required: 7000,
        title: "Grandmaster",
    },
    Level {
        level: 18,
        xp_required: 8000,
        title: "Grandmaster",
    },
    Level {
        level: 19,
        xp_required: 9500,
        title: "Grandmaster",
    },
    Level {
        level: 20,
        xp_required: 11000,
        title: "Code Wizard",
    },
    Level {
        level: 21,
        xp_required: 13000,
        title: "Code Wizard",
    },
    Level {
        level: 22,
        xp_required: 15000,
        title: "Code Wizard",
    },
    Level {
        level: 23,
        xp_required: 17500,
        title: "Code Wizard",
    },
    Level {
        level: 24,
        xp_required: 20000,
        title: "Code Wizard",
    },
    Level {
        level: 25,
        xp_required: 23000,
        title: "Legendary",
    },
    Level {
        level: 26,
        xp_required: 26500,
        title: "Legendary",
    },
    Level {
        level: 27,
        xp_required: 30000,
        title: "Legendary",
    },
    Level {
        level: 28,
        xp_required: 35000,
        title: "Legendary",
    },
    Level {
        level: 29,
        xp_required: 40000,
        title: "Legendary",
    },
    Level {
        level: 30,
        xp_required: 50000,
        title: "Mythic",
    },
];

impl Level {
    /// Calculate level and title for given XP
    pub fn for_xp(xp: u32) -> &'static Level {
        LEVELS
            .iter()
            .rev()
            .find(|l| xp >= l.xp_required)
            .unwrap_or(&LEVELS[0])
    }

    /// Get XP needed for next level (None if max level)
    pub fn xp_for_next(current_level: u32) -> Option<u32> {
        LEVELS
            .iter()
            .find(|l| l.level == current_level + 1)
            .map(|l| l.xp_required)
    }

    /// Get max level
    pub fn max_level() -> u32 {
        LEVELS.last().map(|l| l.level).unwrap_or(1)
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
        let level_info = Level::for_xp(total_xp);
        let next_xp = Level::xp_for_next(level_info.level);

        Self {
            total_xp,
            level: level_info.level,
            title: level_info.title.to_string(),
            current_level_xp: level_info.xp_required,
            next_level_xp: next_xp,
        }
    }

    /// Calculate progress percentage to next level (0.0 - 1.0)
    pub fn progress_to_next(&self) -> f32 {
        match self.next_level_xp {
            Some(next) => {
                let xp_in_level = self.total_xp - self.current_level_xp;
                let xp_for_level = next - self.current_level_xp;
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
        self.next_level_xp.is_none()
    }
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

    /// Calculate streak bonus XP
    /// Streak day 1 = 2 XP, day 2 = 4 XP, etc. (capped at 20)
    pub fn streak_bonus(streak_days: u32) -> u32 {
        (streak_days * 2).min(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_for_xp() {
        assert_eq!(Level::for_xp(0).level, 1);
        assert_eq!(Level::for_xp(49).level, 1);
        assert_eq!(Level::for_xp(50).level, 2);
        assert_eq!(Level::for_xp(150).level, 3);
        assert_eq!(Level::for_xp(50000).level, 30);
        assert_eq!(Level::for_xp(100000).level, 30); // Beyond max
    }

    #[test]
    fn test_player_stats_progress() {
        let stats = PlayerStats::new(75); // Between level 2 (50) and level 3 (150)
        assert_eq!(stats.level, 2);
        assert!((stats.progress_to_next() - 0.25).abs() < 0.01); // 25/100 = 0.25
    }
}
