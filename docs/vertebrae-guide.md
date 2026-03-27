# Vertebrae (vtb) — Task Management Guide

Vertebrae (`vtb`) is a task management CLI. It provides structured workflows for planning, triaging, implementing, and reviewing work through a terminal interface.

## Setup

```bash
vtb init
```

Creates `.claude/skills/` (skill files).

## Core Concepts

### Task Hierarchy

```
epic       → Large initiative spanning multiple features
  ticket   → Single deliverable feature
    task   → Unit of work (default level)
```

### Task Position: Workflow + Step

Tasks don't have a standalone status. Instead, a task's position is defined by its **workflow** and **step** within that workflow. For example, a task might be in the `implementation` workflow at the `coding` step.

Use `vtb workflow assign` to place a task in a workflow, and `vtb transition-to` to move it between steps within that workflow:
```bash
vtb workflow assign <id> <workflow-id>              # Assign to workflow (starts at first step)
vtb transition-to <id> <step-id>                    # Move to another step within the same workflow
```

Workflows and steps are identified by UUIDs. Use `vtb workflow list` and `vtb step list <workflow-id>` to discover the IDs for your project.

### Priorities

`low`, `medium`, `high`, `critical`

---

## Creating Tickets

### Basic Creation

```bash
# Simple task
vtb add "Task title"

# Ticket with level and description
vtb add "Feature title" -l ticket -d "Detailed description"

# Epic for a large initiative
vtb add "Refactor auth system" -l epic -d "Overhaul the authentication layer"

# Subtask under a parent
vtb add "Create sign() function" --parent <ticket-id>

# With priority and tags
vtb add "Fix login bug" -p critical -t bug -t backend

# Mark as needing human review
vtb add "Sensitive security change" --needs-review

# With a dependency (this task is blocked by another)
vtb add "Write integration tests" --depends-on <blocker-id>
```

### Planning a Feature (Epic → Tickets → Tasks)

```bash
# 1. Create the epic
vtb add "Add module dependency graph visualization" -l epic -d "Visual dependency analysis"

# 2. Break into tickets
vtb add "Add dependency query module" -l ticket --parent <epic-id>
vtb add "Add graph output formatter" -l ticket --parent <epic-id>

# 3. Break tickets into tasks
vtb add "Create DependencyGraph struct" --parent <ticket-id>
vtb add "Implement Outputable for DependencyGraph" --parent <ticket-id>

# 4. Set dependencies
vtb depend <outputable-task> --on <struct-task>

# 5. View the plan
vtb show <epic-id>
vtb blockers <final-task-id>
```

---

## Documenting Tickets with Sections

Sections add structured content to tickets. They are critical for triage.

### Section Types

| Type | Purpose | Cardinality |
|------|---------|-------------|
| `goal` | What this task achieves | Single |
| `context` | Background information | Single |
| `current_behavior` | How it works now (for bugs) | Single |
| `desired_behavior` | How it should work | Single |
| `step` | Ordered implementation steps | Multiple |
| `constraint` | Requirements/limitations | Multiple |
| `testing_criterion` | How to verify success | Multiple |
| `anti_pattern` | What to avoid | Multiple |
| `failure_test` | Expected failure/edge cases | Multiple |

### Adding Sections

```bash
# Define the objective
vtb section <id> goal "Add dependency graph query for module-level analysis"

# Background context
vtb section <id> context "SurrealDB stores call edges between functions; module dependencies are derived from these"

# Implementation steps (ordered)
vtb section <id> step "Create DependencyGraph struct with modules and edges fields"
vtb section <id> step "Add SurrealQL query in db/src/queries/dependency_graph.rs"
vtb section <id> step "Implement Execute trait for the command"
vtb section <id> step "Implement TableFormatter for output"

# Constraints
vtb section <id> constraint "Must use ConditionBuilder for regex support"
vtb section <id> constraint "All tests must pass with cargo test"

# Testing criteria (at least 1 unit + 1 integration)
vtb section <id> testing_criterion "UNIT: DependencyGraph::new returns valid struct"
vtb section <id> testing_criterion "INTEGRATION: Full query cycle with test database"

# Anti-patterns
vtb section <id> anti_pattern "Don't include query-level filters in output result structs"

# Failure tests
vtb section <id> failure_test "Invalid regex pattern returns descriptive error"
```

### Viewing Sections

```bash
vtb sections <id>                     # List all sections
vtb sections <id> --type step         # Filter by type
```

### Removing Sections

```bash
# Single-instance types (no index needed)
vtb unsection <id> goal
vtb unsection <id> context

# Multi-instance types (index required)
vtb unsection <id> step --index 2
vtb unsection <id> testing_criterion --index 1
```

### Editing Sections

```bash
vtb update <id> --edit-section step 0 "Updated step content"
vtb update <id> --remove-section step 0
```

---

## Triage: Making Tickets Ready for Work

Triage validates that a ticket is properly documented before it can be transitioned into an actionable workflow.

### Required Sections (blocks triage without them)

| Section | Minimum | Details |
|---------|---------|---------|
| `testing_criterion` | **2** | At least 1 unit + 1 integration criterion |
| `step` | **1** | Implementation steps |
| `constraint` | **2** | Architectural/quality guidelines |
| `goal` or `desired_behavior` | **1** | Clear objective |

### Strongly Encouraged (warns but allows with `--force`)

| Section | Minimum | Purpose |
|---------|---------|---------|
| `anti_pattern` | **1** | Pitfalls to avoid |
| `failure_test` | **1** | Error scenarios/edge cases |

### Recommended (informational only)

| Section | Purpose |
|---------|---------|
| `context` | Background information |
| `current_behavior` | Current state (for bugs/changes) |

### Triage Example

```bash
# 1. Create ticket
vtb add "Fix search bug" -l ticket -d "Search returns no results"

# 2. Add required sections
vtb section <id> goal "Enable searching tasks by ID and content"
vtb section <id> testing_criterion "UNIT: Search matches task IDs correctly"
vtb section <id> testing_criterion "INTEGRATION: Search filters display in real-time"
vtb section <id> step "Debug search query in backend"
vtb section <id> step "Fix event handler"
vtb section <id> constraint "Must validate search input"
vtb section <id> constraint "All tests must pass"

# 3. Add encouraged sections
vtb section <id> anti_pattern "Don't use raw search strings in queries"
vtb section <id> failure_test "Empty search returns all tasks"

# 4. Add optional context
vtb section <id> current_behavior "Search returns no results for task IDs"
vtb section <id> context "Users cannot navigate by task ID"

# 5. Verify the ticket is fully documented
vtb show <id>
```

---

## Workflows and Steps

Workflows define the stages a task progresses through.

### Creating Workflows

```bash
# Basic workflow with steps (format: name:model)
vtb workflow add "Implementation" --step Coding:sonnet --step Testing:haiku --step Docs:haiku

# With description and auto-advance
vtb workflow add "Code Review" \
  -d "Review and approval process" \
  --step Review:sonnet \
  --step Approved:haiku \
  --auto-advance
```

### Managing Workflows

```bash
vtb workflow list                       # List all workflows
vtb workflow show <workflow-id>         # See steps and details
vtb workflow update <id> --name "Dev"   # Rename
vtb workflow update <id> --auto-advance # Enable auto-advance
vtb workflow delete <workflow-id>       # Delete (no assigned tasks allowed)
```

### Assigning Workflows to Tasks

```bash
vtb workflow assign <task-id> <workflow-id>    # Assign (starts at first step)
vtb workflow unassign <task-id>                # Remove workflow
```

### Managing Steps

All step commands use UUIDs. Use `vtb workflow list` and `vtb step list <workflow-id>` to discover IDs.

```bash
# Add a step to an existing workflow
vtb step add "Testing" -w <workflow-id> \
  --goal "Verify implementation" \
  --model sonnet \
  --order 1

# Add a final step (marks workflow complete)
vtb step add "Approved" -w <workflow-id> --final

# Add step with transition restrictions (--transition-to takes a step UUID)
vtb step add "Needs Work" -w <workflow-id> --transition-to <step-id>

# List, show, update, delete steps
vtb step list <workflow-id>
vtb step show <step-id>
vtb step update <step-id> --goal "New goal" --model opus
vtb step delete <step-id>
```

### Step Properties

| Property | Description |
|----------|-------------|
| `order` | Execution order (lower = first) |
| `final` | Marks workflow as complete when reached |
| `goal` | What this step accomplishes |
| `model` | AI model to use (sonnet, haiku, opus) |
| `agents` | Agent file paths for AI-assisted execution |
| `skills` | Slash commands available during this step |
| `transition-to` | Restrict which steps can follow this one |

---

## Moving Tickets Between Workflows and Steps

### Assigning a Workflow (`workflow assign`)

Use `workflow assign` to assign a task to a workflow. This places the task at the workflow's first step.

```bash
vtb workflow assign <task-id> <workflow-id>
```

### Moving Between Steps (`transition-to`)

Use `transition-to` to move a task to a different step **within its current workflow**. The target is a **step UUID**.

```bash
vtb transition-to <task-id> <target-step-id>
```

**Important:** A transition from the current step to the target step must be configured for the transition to succeed. Use `vtb step show <step-id>` to see which steps are valid targets from a given step (listed under `Transitions`).

```bash
# Discover valid transitions from the current step
vtb step show <current-step-id>       # Transitions field shows valid target step UUIDs

# List all steps in a workflow to understand the flow
vtb step list <workflow-id>

# Move to a specific step
vtb transition-to <task-id> <target-step-id>

# Override warnings (but not errors)
vtb transition-to <task-id> <target-step-id> --force

# Bypass validation entirely (escape hatch)
vtb transition-to <task-id> <target-step-id> --skip-validation
```

### Step Lifecycle (within a workflow)

Steps exist within a workflow. Before working on a task, determine which workflow it's in and which step it's currently at:

```bash
vtb step list <workflow-id>       # List all steps in the workflow (with order)
vtb show <task-id>                # See the task's current workflow and step
vtb step show <step-id>           # See step details including valid transitions
```

The step lifecycle commands manage a task's progression through these steps:

| Command | Purpose |
|---------|---------|
| `start-step` | Marks the current step as actively being worked on |
| `complete-step` | Marks the current step as done |
| `reject-step` | Rejects the current step and moves to a target step with optional feedback |

After completing a step, use `transition-to` to move the task to the next step.

#### `start-step` — Begin work on the current step

Signals that work has actively begun on a task's current workflow step. Call this before doing any work on the step.

```bash
vtb start-step <id>
```

**Arguments:**
- `<id>` — Task ID (case-insensitive)

**Behavior:**
- Marks the task's current step as "in progress"
- The task must already be assigned to a workflow and positioned at a step
- Idempotent — calling it again on an already-started step is a no-op

#### `complete-step` — Mark the current step as done

Marks the current step as completed. This does **not** automatically advance to the next step — use `transition-to` afterwards to move the task forward.

```bash
vtb complete-step <id>
```

**Arguments:**
- `<id>` — Task ID (case-insensitive)

**Behavior:**
- Marks the current step as completed
- The task should have been started with `start-step` first
- After completing, check which steps the task can transition to and use `transition-to` to advance

#### `reject-step` — Send a task back to a different step

Rejects the current step and transitions the task to a target step. Typically used during review to send work back for revision. Supports an optional feedback message explaining what needs to change.

```bash
vtb reject-step <id> <target-step-id>
vtb reject-step <id> <target-step-id> -f "Feedback message"
```

**Arguments:**
- `<id>` — Task ID (case-insensitive)
- `<target-step-id>` — The step ID to transition to (e.g., a previous step for rework)

**Options:**
- `-f`, `--feedback <message>` — Explanation of why the step was rejected and what needs to change

**Behavior:**
- Marks the current step as rejected
- Moves the task to the specified target step
- The feedback message is recorded and visible when viewing the task, giving the next worker context on what to fix
- The target step does not need to be a previous step — it can be any valid step in the workflow

#### Working Through a Workflow

Given a workflow (e.g. Implementation `<wf-uuid>`) with steps: Coding `<coding-step-uuid>` (order 0) → Testing `<testing-step-uuid>` (order 1) → Review `<review-step-uuid>` (order 2, final):

```bash
# 1. Determine the task's current position and valid transitions
vtb show <id>                              # Check current workflow and step
vtb step list <wf-uuid>                    # List all steps in the workflow
vtb step show <coding-step-uuid>           # See valid transitions from Coding

# 2. Work on the current step (Coding)
vtb start-step <id>                        # Mark Coding as in progress
# ... do the coding work ...
vtb complete-step <id>                     # Mark Coding as done

# 3. Transition to the next step
vtb transition-to <id> <testing-step-uuid>  # Move to Testing

# 4. Work on the next step (Testing)
vtb start-step <id>                        # Mark Testing as in progress
# ... write and run tests ...
vtb complete-step <id>                     # Mark Testing as done

# 5. Transition to the final step
vtb transition-to <id> <review-step-uuid>   # Move to Review

# 6. Work on the final step (Review)
vtb start-step <id>                        # Mark Review as in progress
# ... review the work ...
vtb complete-step <id>                     # Mark Review as done (final step → workflow complete)
```

#### Handling Rejections

When a step fails review, use `reject-step` to send it back with feedback. The `<target-step-id>` is the UUID of the step to return to:

```bash
# Reviewer finds issues during the Review step
vtb reject-step <id> <coding-step-uuid> -f "Missing error handling for invalid contracts"

# Task is now back at Coding step with feedback attached
vtb start-step <id>                        # Resume work on Coding
# ... fix the issues ...
vtb complete-step <id>                     # Mark Coding as done again
vtb transition-to <id> <testing-step-uuid>  # Re-advance through the workflow
```

#### Step Lifecycle Summary

```
For each step in the workflow:
  1. vtb show <id>                              — confirm current step
  2. vtb step show <current-step-id>            — check valid transitions
  3. vtb start-step <id>                        — mark step as in progress
  4. (do the work for this step)
  5. vtb complete-step <id>                     — mark step as done
  6. vtb transition-to <id> <next-step-id>      — move to the next step

All step arguments are UUIDs.
Use vtb step list <wf-id> to list steps, vtb step show <step-id> to see transitions.

Repeat until the final step is completed.

On rejection:
  vtb reject-step <id> <target-step-id> -f "..."  — send back to a previous step
  (restart the cycle from that step)
```

### Workflow Transitions (between workflows)

Define allowed transitions between workflows:

```bash
# Create transition rule
vtb workflow transition add <from-workflow> <to-workflow> --label "approve"

# With target step in destination
vtb workflow transition add <from-workflow> <to-workflow> \
  --label "escalate" --target-step <step-id>

# List and delete transitions
vtb workflow transition list
vtb workflow transition list --workflow-id <id>
vtb workflow transition delete <from-workflow> <to-workflow>
```

### Key Rules

- **`workflow assign`** assigns a task to a workflow (places it at the first step)
- **`transition-to`** moves a task between steps within its current workflow
- **`start-step` / `complete-step` / `reject-step`** manage the step lifecycle
- **Never use `vtb update`** for workflow/step changes — use `workflow assign` or `transition-to`
- Transitions require a configured path from source step to target step — use `vtb step show` to verify
- Use `--skip-validation` only as an escape hatch

---

## Marking Implementation Steps Done

Track progress on a task's implementation steps:

```bash
# Mark step 1 as done (1-based index)
vtb step-done <task-id> 1

# View step completion status
vtb show <task-id>
```

Steps display with checkboxes:
```
Steps:
  1. [x] Create database schema
  2. [ ] Implement API endpoint
  3. [ ] Write tests
```

---

## Dependencies

### Creating Dependencies

```bash
# Task A depends on task B (B must finish before A can start)
vtb depend <task-a> --on <task-b>
```

### Removing Dependencies

```bash
vtb undepend <task-a> --on <task-b>
```

### Viewing Dependencies

```bash
# Full blocker tree for a task
vtb blockers <task-id>
vtb blockers <task-id> --depth 2        # Limit depth
vtb blockers <task-id> --all            # Include completed blockers

# Shortest path between two tasks
vtb path <from-task> <to-task>
```

---

## Code References

Link tasks to specific code locations:

```bash
# File reference
vtb ref <id> "cli/src/commands/search/execute.rs"

# Specific line
vtb ref <id> "cli/src/commands/search/execute.rs:L42"

# Line range with name
vtb ref <id> "db/src/queries/search.rs:L42-60" --name "find_search_results" --desc "Main search query"

# Link test to testing criterion
vtb criterion-ref <id> 1 "cli/src/commands/search/execute.rs:L100-125" \
  --name "test_search_finds_modules"

# View and remove references
vtb refs <id>
vtb unref <id> "db/src/queries/search.rs"
vtb unref <id> --all
```

---

## Querying Tasks

### Listing

```bash
vtb list                              # All tasks (tree view)
vtb list --flat                       # Flat table view
vtb list --workflow <workflow-id>     # By workflow
vtb list --step <step-id>             # By current step
vtb list -w <wf-id> --step <step-id>  # Combine workflow and step
vtb list --level ticket               # By level
vtb list --priority high              # By priority
vtb list --tag backend                # By tag
vtb list --parent <id>                # Children of a task
vtb list --root                       # Only root items
vtb list --search "auth"              # Search title/description
vtb list --all                        # Include completed items
```

### Viewing Details

```bash
vtb show <id>                         # Full task details with sections, refs, relationships
```

### Finding Actionable Work

```bash
vtb ready                             # Highest-level items ready for work or triage
```

### Checking Current Work

```bash
vtb list --workflow <workflow-id>     # What's in a workflow
vtb blockers <id>                     # What's blocking a task
```

---

## Typical Workflow (End to End)

```bash
# 1. Plan
vtb add "Add cycle detection command" -l epic -d "Detect circular dependencies"
vtb add "Add cycle query module" -l ticket --parent <epic-id>
vtb add "Add cycle output formatting" -l ticket --parent <epic-id>

# 2. Document and triage tickets
vtb section <ticket-id> goal "..."
vtb section <ticket-id> step "..."
vtb section <ticket-id> testing_criterion "UNIT: ..."
vtb section <ticket-id> testing_criterion "INTEGRATION: ..."
vtb section <ticket-id> constraint "..."
vtb section <ticket-id> constraint "..."
vtb workflow assign <ticket-id> <backlog-wf-id>         # Assign to backlog workflow

# 3. Discover workflow and step UUIDs, then assign to implementation
vtb workflow list                                       # Find workflow UUIDs
vtb step list <impl-wf-id>                              # Find step UUIDs within it
vtb workflow assign <ticket-id> <impl-wf-id>            # Assign (starts at first step)

# 4. Work through steps (Coding → Testing → ...)
vtb start-step <ticket-id>
vtb step-done <ticket-id> 1
vtb step-done <ticket-id> 2
vtb complete-step <ticket-id>
vtb transition-to <ticket-id> <testing-step-id>         # Move to Testing step

vtb start-step <ticket-id>
# ... run tests ...
vtb complete-step <ticket-id>
vtb transition-to <ticket-id> <review-step-id>          # Move to Review step

# 5. Review and complete
vtb start-step <ticket-id>
vtb complete-step <ticket-id>            # or reject-step if rework needed

# 6. Move to next
vtb ready
vtb workflow assign <next-id> <impl-wf-id>
```

---

## Human Review

```bash
vtb review <id>                       # Toggle needs_human_review flag
vtb review <id> --set true            # Explicitly set
vtb review <id> --set false           # Clear
```

Tasks with `needs_human_review: true` pause automated workflow advancement.

## Execution Tracking

Record workflow execution history for auditing:

```bash
vtb execution create <task-id>                                    # Start execution record
vtb execution log <execution-id> "Processing..." --level info     # Add log entry
vtb execution update <execution-id> --status completed            # Mark complete
vtb execution list <task-id>                                      # List executions
vtb execution show <execution-id>                                 # Show details
```

## Updating Tasks

```bash
vtb update <id> --title "New title"
vtb update <id> --description "New description"
vtb update <id> --priority high
vtb update <id> --add-tag urgent --add-tag backend
vtb update <id> --remove-tag old-tag
vtb update <id> --level ticket
vtb update <id> --parent <parent-id>
vtb update <id> --parent ""              # Remove parent
```

**Never use `vtb update` for workflow/step changes** — use `vtb transition-to` instead.

## Deleting Tasks

```bash
vtb delete <id>                          # Delete single task
vtb delete <id> --cascade                # Delete task and all children
```
