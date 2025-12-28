//! Git operations and worktree management

mod manager;

pub use manager::CommitMessage;
pub use manager::{DiffReport, DiffSettings, FileDiff, FileStatus};
pub use manager::{GitManager, WorktreeInfo, find_git_root};
