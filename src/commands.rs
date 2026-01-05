//! CLI command definitions for kyco.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Run the GUI (receives selections from IDE extensions via HTTP)
    Gui,

    /// Show the status of all jobs
    Status {
        /// Show only jobs with this status
        #[arg(long)]
        filter: Option<String>,
    },

    /// Initialize a new .kyco/config.toml configuration file
    Init {
        /// Overwrite existing config file
        #[arg(long)]
        force: bool,
    },

    /// Control jobs in a running KYCo GUI (local /ctl API)
    Job {
        #[command(subcommand)]
        command: JobCommands,
    },

    /// Manage modes in `.kyco/config.toml`
    Mode {
        #[command(subcommand)]
        command: ModeCommands,
    },

    /// List/show configured agents
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },

    /// List/show configured chains
    Chain {
        #[command(subcommand)]
        command: ChainCommands,
    },
}

#[derive(Subcommand)]
pub enum JobCommands {
    /// List jobs
    List {
        /// Print JSON instead of human output
        #[arg(long)]
        json: bool,
        /// Filter by status (pending, queued, running, completed, failed, aborted)
        #[arg(long, short = 's')]
        status: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n')]
        limit: Option<usize>,
        /// Search in job description/prompt
        #[arg(long, short = 'q')]
        search: Option<String>,
        /// Filter by mode (e.g., review, implement, fix)
        #[arg(long, short = 'm')]
        mode: Option<String>,
    },
    /// Get a single job by ID
    Get {
        job_id: u64,
        /// Print JSON instead of human output
        #[arg(long)]
        json: bool,
    },
    /// Start a job (creates it in the GUI and optionally queues it)
    Start {
        /// File path (relative to --path, or absolute)
        #[arg(long)]
        file: String,
        /// Start line (1-indexed)
        #[arg(long)]
        line_start: Option<usize>,
        /// End line (1-indexed)
        #[arg(long)]
        line_end: Option<usize>,
        /// Mode or chain name
        #[arg(long)]
        mode: String,
        /// Optional prompt/description text
        #[arg(long)]
        prompt: Option<String>,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Create as pending only (do not queue immediately)
        #[arg(long)]
        pending: bool,
        /// Force running in a git worktree (like Shift+Enter in the UI)
        #[arg(long)]
        force_worktree: bool,
        /// Print JSON response
        #[arg(long)]
        json: bool,
    },
    /// Queue a job (set status=queued)
    Queue { job_id: u64 },
    /// Abort/stop a job (graceful, waits for agent)
    Abort { job_id: u64 },
    /// Kill a job immediately (forceful, does not wait)
    Kill { job_id: u64 },
    /// Delete a job from the GUI list
    Delete {
        job_id: u64,
        /// Also remove the job's git worktree (if any)
        #[arg(long)]
        cleanup_worktree: bool,
    },
    /// Continue a session job with a follow-up prompt (creates a new job)
    Continue {
        job_id: u64,
        /// Follow-up prompt text
        #[arg(long)]
        prompt: String,
        /// Create as pending only (do not queue immediately)
        #[arg(long)]
        pending: bool,
        /// Print JSON response
        #[arg(long)]
        json: bool,
    },
    /// Wait until a job reaches a terminal state
    Wait {
        job_id: u64,
        /// Timeout in seconds
        #[arg(long)]
        timeout_secs: Option<u64>,
        /// Poll interval in milliseconds
        #[arg(long, default_value_t = 500)]
        poll_ms: u64,
        /// Print final job JSON
        #[arg(long)]
        json: bool,
    },
    /// Print a job's output / result
    Output {
        job_id: u64,
        /// Print full job JSON
        #[arg(long)]
        json: bool,
        /// Print parsed `result.summary` (or raw fallback)
        #[arg(long)]
        summary: bool,
        /// Print parsed `result.state`
        #[arg(long)]
        state: bool,
    },
    /// Merge a job's changes into the base branch
    Merge {
        job_id: u64,
        /// Custom commit message (optional)
        #[arg(long, short = 'm')]
        message: Option<String>,
    },
    /// Reject a job's changes and cleanup its worktree
    Reject { job_id: u64 },
    /// Restart a failed or rejected job with the same parameters
    Restart { job_id: u64 },
    /// Show the diff of a job's changes
    Diff {
        job_id: u64,
        /// Print JSON output with metadata
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ModeCommands {
    /// List configured modes
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
    },
    /// Show a mode definition
    Get {
        name: String,
        /// Print JSON instead of TOML
        #[arg(long)]
        json: bool,
    },
    /// Create or update a mode
    Set {
        name: String,
        /// Prompt template
        #[arg(long)]
        prompt: Option<String>,
        /// System prompt
        #[arg(long)]
        system_prompt: Option<String>,
        /// Default agent id
        #[arg(long)]
        agent: Option<String>,
        /// Aliases (comma-separated)
        #[arg(long, value_delimiter = ',')]
        aliases: Vec<String>,
        /// Session mode: oneshot|session
        #[arg(long)]
        session_mode: Option<String>,
        /// Max turns (0 = unlimited)
        #[arg(long)]
        max_turns: Option<u32>,
        /// Model override
        #[arg(long)]
        model: Option<String>,
        /// Disallowed tools (comma-separated)
        #[arg(long, value_delimiter = ',')]
        disallowed_tools: Vec<String>,
        /// Output states (comma-separated)
        #[arg(long, value_delimiter = ',')]
        output_states: Vec<String>,
        /// Custom state prompt
        #[arg(long)]
        state_prompt: Option<String>,
        /// Convenience: mark mode as read-only (disallow Write/Edit)
        #[arg(long)]
        readonly: bool,
        /// Print JSON for the saved mode
        #[arg(long)]
        json: bool,
    },
    /// Delete a mode
    Delete { name: String },
}

#[derive(Subcommand)]
pub enum AgentCommands {
    /// List configured agents
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
    },
    /// Show an agent definition
    Get {
        name: String,
        /// Print JSON instead of TOML
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ChainCommands {
    /// List configured chains
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
    },
    /// Show a chain definition
    Get {
        name: String,
        /// Print JSON instead of TOML
        #[arg(long)]
        json: bool,
    },
    /// Create or update a chain
    Set {
        name: String,
        /// Description of what this chain does
        #[arg(long)]
        description: Option<String>,
        /// Steps (comma-separated mode names, e.g., "review,fix,test")
        #[arg(long, value_delimiter = ',')]
        steps: Vec<String>,
        /// Stop the chain on first failure
        #[arg(long)]
        stop_on_failure: bool,
        /// Continue chain even if a step fails
        #[arg(long)]
        no_stop_on_failure: bool,
        /// Pass full response to next step
        #[arg(long)]
        pass_full_response: bool,
        /// Pass only summary to next step
        #[arg(long)]
        no_pass_full_response: bool,
        /// Maximum loop iterations (for chains with loop_to)
        #[arg(long)]
        max_loops: Option<u32>,
        /// Force running in a git worktree
        #[arg(long)]
        use_worktree: bool,
        /// Disable git worktree for this chain
        #[arg(long)]
        no_use_worktree: bool,
        /// Print JSON for the saved chain
        #[arg(long)]
        json: bool,
    },
    /// Delete a chain
    Delete { name: String },
}
