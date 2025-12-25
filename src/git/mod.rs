//! Git operations and worktree management

mod manager;

pub use manager::CommitMessage;
pub use manager::{find_git_root, GitManager, WorktreeInfo};
