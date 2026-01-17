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

    /// Manage modes in `.kyco/config.toml` (DEPRECATED: use skill instead)
    Mode {
        #[command(subcommand)]
        command: ModeCommands,
    },

    /// Manage skills (SKILL.md files in .claude/skills/ or .codex/skills/)
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
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

    /// Manage security findings (BugBounty Kanban)
    Finding {
        #[command(subcommand)]
        command: FindingCommands,
    },

    /// Import findings from external security tools (aliases for `kyco finding import`)
    Import {
        #[command(subcommand)]
        command: ImportCommands,
    },

    /// Manage BugBounty projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    /// Check scope and tool policy for a project
    Scope {
        #[command(subcommand)]
        command: ScopeCommands,
    },
}

#[derive(Subcommand)]
pub enum JobCommands {
    /// List jobs
    List {
        /// Print JSON instead of human output
        #[arg(long)]
        json: bool,
        /// Filter by BugBounty project ID (job.bugbounty_project_id)
        #[arg(long)]
        project: Option<String>,
        /// Filter by linked finding ID (requires BugBounty DB)
        #[arg(long)]
        finding: Option<String>,
        /// Filter by status (pending, queued, running, completed, failed, aborted)
        #[arg(long, short = 's')]
        status: Option<String>,
        /// Filter by agent result state (job.result.state)
        #[arg(long)]
        state: Option<String>,
        /// Limit number of results
        #[arg(long, short = 'n')]
        limit: Option<usize>,
        /// Search in job description/prompt
        #[arg(long, short = 'q')]
        search: Option<String>,
        /// Filter by skill (e.g., review, implement, fix)
        #[arg(long, visible_alias = "mode")]
        skill: Option<String>,
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
        /// File path (relative to --path, or absolute). Optional if --prompt is provided.
        #[arg(long)]
        file: Option<String>,
        /// One or more input files / glob patterns (repeatable, comma-separated)
        #[arg(long, value_delimiter = ',')]
        input: Vec<String>,
        /// Create one job per resolved input (requires --input)
        #[arg(long)]
        batch: bool,
        /// Start line (1-indexed)
        #[arg(long)]
        line_start: Option<usize>,
        /// End line (1-indexed)
        #[arg(long)]
        line_end: Option<usize>,
        /// Skill or chain name
        #[arg(long, visible_alias = "mode")]
        skill: String,
        /// Optional prompt/description text
        #[arg(long)]
        prompt: Option<String>,
        /// BugBounty project ID (optional; overrides inference/active project)
        #[arg(long)]
        project: Option<String>,
        /// Link this job to one or more existing findings (repeatable, comma-separated)
        #[arg(long, value_delimiter = ',')]
        finding: Vec<String>,
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
        /// Print parsed `next_context` (JSON)
        #[arg(long)]
        next_context: bool,
        /// Print parsed `next_context.findings` (JSON)
        #[arg(long)]
        findings: bool,
        /// Print parsed `next_context.flow_edges` (JSON)
        #[arg(long)]
        flow: bool,
        /// Print parsed `next_context.artifacts` (JSON)
        #[arg(long)]
        artifacts: bool,
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
pub enum SkillCommands {
    /// List available skills
    List {
        /// Print JSON instead of plain lines
        #[arg(long)]
        json: bool,
        /// Filter by agent (claude, codex)
        #[arg(long)]
        agent: Option<String>,
    },
    /// Show a skill definition
    Get {
        name: String,
        /// Print JSON instead of SKILL.md format
        #[arg(long)]
        json: bool,
    },
    /// Create a new skill
    Create {
        name: String,
        /// Description of what this skill does
        #[arg(long)]
        description: Option<String>,
        /// Agent type: claude (default) or codex
        #[arg(long)]
        agent: Option<String>,
        /// Create in global ~/.kyco/skills/ instead of project-local
        #[arg(long)]
        global: bool,
        /// Print JSON for the created skill
        #[arg(long)]
        json: bool,
    },
    /// Delete a skill
    Delete {
        name: String,
        /// Agent type: claude (default) or codex
        #[arg(long)]
        agent: Option<String>,
        /// Delete from global ~/.kyco/skills/
        #[arg(long)]
        global: bool,
    },
    /// Show the file path for a skill
    Path {
        name: String,
        /// Agent type: claude (default) or codex
        #[arg(long)]
        agent: Option<String>,
    },
    /// Install a skill template to all agent directories (.claude/skills/ and .codex/skills/)
    Install {
        name: String,
        /// Deprecated: Skills are now installed to all agent directories
        #[arg(long, hide = true)]
        agent: Option<String>,
        /// Also install to global ~/.kyco/skills/ for system-wide access
        #[arg(long)]
        global: bool,
    },

    // =========================================================================
    // Registry commands (search & install from 50,000+ community skills)
    // =========================================================================

    /// Search for skills in the community registry (~50,000 skills)
    Search {
        /// Search query (matches name, description, author)
        query: String,
        /// Maximum number of results (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Print JSON instead of human-readable format
        #[arg(long)]
        json: bool,
    },
    /// Show details about a skill in the registry
    Info {
        /// Skill name (author/name or just name)
        name: String,
        /// Print JSON instead of human-readable format
        #[arg(long)]
        json: bool,
    },
    /// Install a skill from the community registry (downloads from GitHub)
    #[command(name = "install-from-registry")]
    InstallFromRegistry {
        /// Skill to install (author/name or just name)
        name: String,
        /// Agent type: claude, codex, or both (default)
        #[arg(long)]
        agent: Option<String>,
        /// Install to global ~/.kyco/skills/ instead of project-local
        #[arg(long)]
        global: bool,
    },
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

// ============================================
// BUGBOUNTY COMMANDS
// ============================================

#[derive(Subcommand)]
pub enum FindingCommands {
    /// List findings
    List {
        /// Filter by project ID
        #[arg(long)]
        project: Option<String>,
        /// Filter by status (raw, needs_repro, verified, submitted, etc.)
        #[arg(long, short = 's')]
        status: Option<String>,
        /// Filter by severity (critical, high, medium, low, info)
        #[arg(long)]
        severity: Option<String>,
        /// Search query (matches id/title/text/assets)
        #[arg(long, short = 'q')]
        search: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Show a finding by ID
    Show {
        /// Finding ID (e.g., VULN-001)
        id: String,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Create a new finding
    Create {
        /// Finding title
        #[arg(long, short = 't')]
        title: String,
        /// Project ID
        #[arg(long)]
        project: String,
        /// Severity (critical, high, medium, low, info)
        #[arg(long, short = 's')]
        severity: Option<String>,
        /// Attack scenario description
        #[arg(long)]
        attack_scenario: Option<String>,
        /// Preconditions
        #[arg(long)]
        preconditions: Option<String>,
        /// Impact description
        #[arg(long)]
        impact: Option<String>,
        /// Confidence (high, medium, low)
        #[arg(long)]
        confidence: Option<String>,
        /// CWE ID (e.g., CWE-639)
        #[arg(long)]
        cwe: Option<String>,
        /// Affected assets (comma-separated)
        #[arg(long, value_delimiter = ',')]
        assets: Vec<String>,
        /// Also write `notes/findings/<id>.md` under the project's root
        #[arg(long)]
        write_notes: bool,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Update finding status (move in Kanban)
    SetStatus {
        /// Finding ID
        id: String,
        /// New status (raw, needs_repro, verified, report_draft, submitted, triaged, accepted, paid, duplicate, wont_fix, false_positive, out_of_scope)
        status: String,
    },
    /// Link an existing job to a finding
    Link {
        /// Finding ID
        #[arg(long)]
        finding: String,
        /// Job identifier (KYCo job id like "123" or BugBounty job id UUID)
        #[arg(long)]
        job: String,
        /// Link type (e.g. discovered, related, verification)
        #[arg(long, default_value = "related")]
        link_type: String,
    },
    /// Remove a job link from a finding
    Unlink {
        /// Finding ID
        #[arg(long)]
        finding: String,
        /// Job identifier (KYCo job id like "123" or BugBounty job id UUID)
        #[arg(long)]
        job: String,
    },
    /// Mark finding as false positive
    Fp {
        /// Finding ID
        id: String,
        /// Reason for marking as FP
        #[arg(long, short = 'r')]
        reason: String,
    },
    /// Delete a finding
    Delete {
        /// Finding ID
        id: String,
        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Export finding to report format
    Export {
        /// Finding ID
        id: String,
        /// Output format (markdown, intigriti, hackerone)
        #[arg(long, short = 'f', default_value = "markdown")]
        format: String,
        /// Output file path (stdout if not specified)
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
    /// Export a finding to `notes/findings/<id>.md` under the project root
    ExportNotes {
        /// Finding ID
        id: String,
        /// Dry run - show diff without writing
        #[arg(long)]
        dry_run: bool,
        /// Overwrite even if notes file looks newer than the DB
        #[arg(long)]
        force: bool,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Import findings from SARIF or Semgrep output
    Import {
        /// Path to the file to import (SARIF/Semgrep/Snyk/Nuclei JSON)
        file: String,
        /// Target project ID
        #[arg(long)]
        project: String,
        /// Input format (sarif, semgrep, snyk, nuclei, auto)
        #[arg(long, short = 'f', default_value = "auto")]
        format: String,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Import/sync findings from `notes/findings/*.md` in the project root
    ImportNotes {
        /// Target project ID
        #[arg(long)]
        project: String,
        /// Dry run - parse and report without writing to DB
        #[arg(long)]
        dry_run: bool,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Extract findings from a completed job's output
    ExtractFromJob {
        /// Job ID to extract findings from
        job_id: u64,
        /// Target project ID (uses job's bugbounty_project_id if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ImportCommands {
    /// Import Semgrep JSON output
    Semgrep {
        /// Path to Semgrep JSON output
        file: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Create one verify job per imported finding
        #[arg(long)]
        create_jobs: bool,
        /// Queue created jobs immediately (default: create as pending only)
        #[arg(long)]
        queue: bool,
        /// Skill/chain to run for verify jobs (only used with --create-jobs)
        #[arg(long, default_value = "flow-trace")]
        skill: String,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Import CodeQL SARIF output
    Codeql {
        /// Path to SARIF output
        file: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Create one verify job per imported finding
        #[arg(long)]
        create_jobs: bool,
        /// Queue created jobs immediately (default: create as pending only)
        #[arg(long)]
        queue: bool,
        /// Skill/chain to run for verify jobs (only used with --create-jobs)
        #[arg(long, default_value = "flow-trace")]
        skill: String,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Import SARIF output (generic)
    Sarif {
        /// Path to SARIF output
        file: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Create one verify job per imported finding
        #[arg(long)]
        create_jobs: bool,
        /// Queue created jobs immediately (default: create as pending only)
        #[arg(long)]
        queue: bool,
        /// Skill/chain to run for verify jobs (only used with --create-jobs)
        #[arg(long, default_value = "flow-trace")]
        skill: String,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Import Snyk JSON output
    Snyk {
        /// Path to Snyk JSON output
        file: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Create one verify job per imported finding
        #[arg(long)]
        create_jobs: bool,
        /// Queue created jobs immediately (default: create as pending only)
        #[arg(long)]
        queue: bool,
        /// Skill/chain to run for verify jobs (only used with --create-jobs)
        #[arg(long, default_value = "flow-trace")]
        skill: String,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Import Nuclei JSON/JSONL output
    Nuclei {
        /// Path to Nuclei JSON/JSONL output
        file: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Create one verify job per imported finding
        #[arg(long)]
        create_jobs: bool,
        /// Queue created jobs immediately (default: create as pending only)
        #[arg(long)]
        queue: bool,
        /// Skill/chain to run for verify jobs (only used with --create-jobs)
        #[arg(long, default_value = "flow-trace")]
        skill: String,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Auto-detect and import output (SARIF/Semgrep/Snyk/Nuclei)
    Auto {
        /// Path to the file to import
        file: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Create one verify job per imported finding
        #[arg(long)]
        create_jobs: bool,
        /// Queue created jobs immediately (default: create as pending only)
        #[arg(long)]
        queue: bool,
        /// Skill/chain to run for verify jobs (only used with --create-jobs)
        #[arg(long, default_value = "flow-trace")]
        skill: String,
        /// Primary agent id (e.g. "claude")
        #[arg(long)]
        agent: Option<String>,
        /// Optional list of agents for parallel execution (comma-separated)
        #[arg(long, value_delimiter = ',')]
        agents: Vec<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// List all projects
    List {
        /// Filter by platform (hackerone, intigriti, bugcrowd)
        #[arg(long)]
        platform: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Show a project by ID
    Show {
        /// Project ID (e.g., hackerone-nextcloud)
        id: String,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Discover projects from BugBounty/programs/ directory
    Discover {
        /// Path to scan (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
        /// Dry run - show what would be discovered without creating
        #[arg(long)]
        dry_run: bool,
    },
    /// Select a project as the active project
    Select {
        /// Project ID to select
        id: String,
    },
    /// Initialize a new project
    Init {
        /// Project ID (e.g., hackerone-nextcloud)
        #[arg(long)]
        id: String,
        /// Root path for the project
        #[arg(long = "root")]
        root_path: String,
        /// Platform (hackerone, intigriti, bugcrowd)
        #[arg(long)]
        platform: Option<String>,
    },
    /// Delete a project
    Delete {
        /// Project ID
        id: String,
        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Generate project overview (findings summary, status, etc.)
    Overview {
        /// Project ID (optional, generates for all if omitted)
        #[arg(long)]
        project: Option<String>,
        /// Output file path (stdout if not specified)
        #[arg(long, short = 'o')]
        output: Option<String>,
        /// Also update BugBounty/OVERVIEW.md
        #[arg(long)]
        update_global: bool,
        /// Print JSON instead of markdown
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum ScopeCommands {
    /// Show scope for a project
    Show {
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Check if a URL/asset is in scope
    Check {
        /// URL or asset to check
        url: String,
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
    },
    /// Show tool policy for a project
    Policy {
        /// Project ID (uses active project if not specified)
        #[arg(long)]
        project: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
}
