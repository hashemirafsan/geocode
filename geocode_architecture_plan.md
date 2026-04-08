# GeoCode
## Product Plan, Architecture, Delivery Strategy, and Ticket Backlog

## 1) Executive Summary
GeoCode is a local-first, CLI-first geospatial analysis tool that should grow into an optional agentic workflow product over time. The first release should feel like a trusted scientific utility for CLI-native researchers, not a partially finished AI assistant.

The core delivery principle is:
- architecture wide
- release narrow

That means GeoCode should preserve the long-term seams for sessions, providers, memory, and agent orchestration from day one, while shipping `v0.1` as a deterministic command-line tool with immediate value.

## 2) Locked Product Decisions
### Product identity
- Product name: `GeoCode`
- CLI/repo/module naming: `geocode`

### Initial audience
- Primary audience: CLI-native researchers

### `v0.1` supported formats
- NetCDF (`.nc`)
- GeoTIFF (`.tiff`, `.tif`)

### `v0.1` command scope
- `geocode inspect <file>`
- `geocode mean <file> [--var <name>]`
- `geocode compare <file-a> <file-b> [--var <name>]`

### `v0.1` product stance
- LLM is optional
- direct command mode is the source of truth
- JSON output is first-class from day one
- session support exists, but stays minimal
- memory is deferred
- packaging can wait; a developer-facing binary release is enough

### Semantic decisions
- `compare a b` means `mean_b - mean_a`
- NetCDF variable selection is explicit when needed
- GeoTIFF mean should exclude nodata by default when nodata metadata exists
- `inspect` should show essential metadata only
- `compare` is scalar-summary only in `v0.1`
- local files only in `v0.1`
- JSON exists in `v0.1`, but strong schema stability is a `v0.2` concern

## 3) Product Vision
GeoCode should let users work with geospatial datasets through deterministic CLI commands first, and later through optional natural-language and session-aware workflows.

Positioning:
- OpenCode-style geospatial workflow experience
- local execution for deterministic scientific computation
- Rust engine as executor
- LLM as optional planner later, not executor
- CLI-first with TUI later
- explicit, traceable, and testable behavior

## 4) Product Goals
### Primary goals
- Let users inspect unfamiliar datasets quickly.
- Compute reliable summary statistics on supported data types.
- Compare baseline and scenario datasets through explicit commands.
- Preserve enough session context for later follow-up support.
- Provide structured JSON output for automation and scripting.
- Grow into agent mode without rewriting the core execution model.

### Non-goals for initial releases
- Full code generation for arbitrary scientific workflows
- Cloud-hosted multi-tenant SaaS platform
- Rich desktop GIS replacement
- Spatial alignment, resampling, or reprojection-heavy comparison in `v0.1`
- Rich persistent memory in `v0.1`

## 5) Target Users and Jobs To Be Done
### Primary users
- Climate and flood model researchers
- Geospatial engineers
- Environmental analysts working in terminal-first workflows

### Core user jobs
1. Inspect a new NetCDF or GeoTIFF file.
2. Understand available variables or raster characteristics quickly.
3. Compute a basic mean over a selected dataset target.
4. Compare two datasets through a deterministic summary.
5. Script workflows with machine-readable output.
6. Later, ask follow-up questions without repeating full context.

## 6) Product Modes
GeoCode should support two product modes architecturally, but only one needs to ship first.

### Mode A: Direct command mode
Examples:
- `geocode inspect base.nc`
- `geocode mean base.nc --var depth`
- `geocode compare base.nc scenario.nc --var depth`

Purpose:
- deterministic execution
- easy testing
- clear UX for expert users
- fastest path to a useful MVP

### Mode B: Agent session mode
Examples:
- `geocode ask "show all variables in the base file"`
- `geocode chat`
- follow-up: `now compare that with scenario`

Purpose:
- natural-language entrypoint
- session-aware follow-up
- future tool chaining

Important release rule:
- Mode B must not block `v0.1`

## 7) Core Architectural Principles
1. LLM plans; Rust executes.
2. Deterministic numeric behavior is non-negotiable.
3. CLI commands must be useful without LLM setup.
4. Session state must be explicit and persisted in a narrow, understandable way.
5. Memory must be scoped and should not be introduced before its value is proven.
6. Tools must be typed, composable primitives.
7. Provider integration must be replaceable, but should start minimal.
8. Releases must ship vertical slices with real user value.

## 8) Business and Functional Requirements
### `v0.1` requirements
- Load local NetCDF and GeoTIFF files.
- Inspect essential metadata.
- Compute mean summaries.
- Compare two same-type files by scalar mean summary.
- Return human-readable output.
- Return JSON output via `--json`.
- Persist minimal session context.
- Provide explicit failure modes.
- Be testable with sample fixtures and golden outputs.

### Post-`v0.1` requirements
- Session commands (`show`, `clear`, later `list`)
- Optional agent entrypoints (`ask`, `chat`)
- OpenAI-backed planner integration
- Memory inspection and reset commands
- Richer stats and tool chaining
- TUI
- Exports and visualization

## 9) `v0.1` Command Semantics
### `geocode inspect <file>`
Returns essential metadata only.

#### NetCDF inspect output should include
- file type
- variable names
- dimensions
- shape
- dtype where available

#### GeoTIFF inspect output should include
- file type
- band count
- raster width and height
- dtype
- nodata if available

### `geocode mean <file> [--var <name>]`
#### NetCDF
- require `--var` when needed
- compute mean across the full selected variable
- no dimension filters yet

#### GeoTIFF
- compute mean over valid raster values
- exclude nodata by default when nodata is defined

### `geocode compare <file-a> <file-b> [--var <name>]`
Rules:
- same-type only
- no alignment, reprojection, or resampling in `v0.1`
- no per-cell diff output
- compare independent scalar means only
- `difference = mean_b - mean_a`

This keeps the comparison minimal, deterministic, and scientifically honest for the first release.

## 10) Recommended Architecture Shape
The original wide architecture is directionally correct, but too large for the current stage if implemented literally.

Recommended initial module layout:

```text
geocode/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── app.rs
│   ├── engine.rs
│   ├── tools.rs
│   ├── session.rs
│   ├── output.rs
│   ├── agent.rs
│   └── provider.rs
├── fixtures/
│   ├── netcdf/
│   ├── geotiff/
│   └── golden/
└── tests/
```

Why this shape:
- preserves future seams
- avoids many empty submodules
- keeps implementation minimal
- allows later splitting without rewrite

### Initial module responsibilities
#### `cli`
- argument parsing
- command dispatch into app layer

#### `app`
- command orchestration
- maps parsed command requests into engine/tool execution

#### `engine`
- execution model
- shared validation
- errors
- trace events

#### `tools`
- typed primitive operations for inspect, mean, compare

#### `session`
- minimal session load/save
- workspace path
- aliases
- last variable

#### `output`
- text rendering
- JSON rendering

#### `agent`
- placeholder types and later planner request/response contracts

#### `provider`
- placeholder abstraction for later OpenAI integration

## 11) High-Level Architecture
```text
User CLI Input
      ↓
CLI Parser / Router
      ↓
App Layer
      ↓
Execution Engine
      ↓
Typed Tools
      ↓
Result Renderer
      ↓
Text / JSON Output

Session Store remains sidecar state for command context.
Agent and provider layers are future extensions over the same execution path.
```

## 12) Session Strategy
### `v0.1` session scope
Persist only:
- session id
- workspace path
- alias mapping if internally used
- last variable

### Session rules
- session should help continuity, not create hidden complexity
- no persistent memory subsystem in `v0.1`
- failed commands should not corrupt session state

### Suggested storage paths
- `~/.config/geocode/`
- `~/.local/share/geocode/` or platform equivalent

## 13) Provider and Agent Strategy
### `v0.1`
- no provider dependency for core value
- no full agent loop required

### First provider plan
- OpenAI first
- API key flow first
- graceful disable when unconfigured

### Agent mode rule
- agent mode should reuse the same execution layer as direct commands
- agent mode should start narrow and optional

## 14) Release Strategy
### Public Release `0.1`
- direct command mode only
- NetCDF and GeoTIFF support
- `inspect`, `mean`, `compare`
- text + JSON output
- minimal session foundation
- developer-facing binary release

### Public Release `0.2`
- session-aware agent entry
- stronger JSON contract
- follow-up query support

### Public Release `0.3`
- OpenAI provider integration
- provider status and selection

### Public Release `0.4`
- scoped memory
- richer tool chaining

### Public Release `0.5`
- TUI
- exports and visualization

## 15) Risks and Mitigations
### Risk: scope explosion
Mitigation:
- strict release boundaries
- do not let agent mode block direct command value

### Risk: NetCDF metadata and variable complexity
Mitigation:
- require explicit `--var`
- keep inspect output narrow

### Risk: GeoTIFF nodata inconsistencies
Mitigation:
- define nodata exclusion behavior explicitly
- use controlled fixtures with known expectations

### Risk: compare expands into alignment work too early
Mitigation:
- keep compare scalar-summary only in `v0.1`
- reject out-of-scope semantics instead of guessing

### Risk: JSON output becomes unstable
Mitigation:
- keep JSON fields minimal in `v0.1`
- defer schema guarantees to `v0.2`

## 16) Recommended Milestone Plan
### Milestone 0: Foundation
Goal:
- create the minimum architecture for deterministic command execution

Deliverables:
- Rust crate bootstrap
- top-level modules
- shared domain types
- app routing path
- output contract
- session contract
- fixture strategy

### Milestone 1: `v0.1` Command MVP
Goal:
- ship a narrow, trustworthy CLI

Deliverables:
- file type detection
- `inspect`
- `mean`
- `compare`
- text and JSON output
- minimal session persistence
- sample datasets
- golden tests

### Milestone 2: Session Utilities
Goal:
- improve continuity for command workflows

Deliverables:
- `session show`
- `session clear`

### Milestone 3: Optional Agent Entry
Goal:
- add bounded natural-language support without harming command mode

Deliverables:
- planner request/response schema
- OpenAI config path
- `ask` or `chat`
- graceful disable when unconfigured

### Milestone 4: Scoped Memory
Goal:
- improve follow-up accuracy through explicit, inspectable memory

Deliverables:
- memory categories
- memory inspect/reset commands
- stable preference storage only

### Milestone 5: Richer Analysis and Tool Chaining
Goal:
- add more statistics and transparent multi-step execution

Deliverables:
- richer stats
- visible traces
- better query coverage

### Milestone 6: TUI and Exports
Goal:
- improve usability after command and agent foundations are solid

Deliverables:
- TUI
- export paths
- later visualization helpers

## 17) Acceptance Criteria for `v0.1`
1. Users can run `inspect`, `mean`, and `compare` on local NetCDF and GeoTIFF files.
2. `compare a b` consistently reports `mean_b - mean_a`.
3. Error messages are explicit for unsupported or ambiguous cases.
4. `--json` is available on all core commands.
5. Session persistence exists but remains minimal and unobtrusive.
6. Sample fixtures and golden outputs cover the core command paths.
7. No LLM or provider setup is required to get real value from the tool.

## 18) Ticket Backlog
### Epic 0: Project Bootstrap
#### `BOOT-001` Initialize Rust crate for `geocode`
Definition of done:
- Cargo project exists
- binary name is `geocode`
- project builds successfully

#### `BOOT-002` Create minimal top-level module layout
Definition of done:
- modules exist for `cli`, `app`, `engine`, `tools`, `session`, `output`, `agent`, `provider`
- no deep submodule tree yet
- build still passes

#### `BOOT-003` Define core domain types
Definition of done:
- types exist for dataset kind, dataset ref, variable ref, command request/response, execution error, trace event, session state
- types are shared from a stable location
- no duplicate ad hoc request/response structs across modules

#### `BOOT-004` Define CLI command surface
Definition of done:
- CLI parses `inspect`, `mean`, `compare`
- required and optional flags are defined
- help output is coherent

#### `BOOT-005` Add app-layer request routing
Definition of done:
- CLI parsing is separated from execution
- parsed requests are routed through a single app or service path
- command handlers are not embedded in `main`

### Epic 1: Foundation Contracts
#### `CORE-001` Define dataset type detection contract
Definition of done:
- clear internal API exists for identifying NetCDF vs GeoTIFF
- unsupported file handling path is defined

#### `CORE-002` Define tool execution contract
Definition of done:
- minimal tool interface exists with id, typed input/output, validation, execution
- future tools can plug into the same structure

#### `CORE-003` Define output rendering contract
Definition of done:
- text and JSON output paths are defined
- command handlers return structured results, not preformatted strings only

#### `CORE-004` Define session persistence contract
Definition of done:
- session schema includes only session id, workspace path, aliases, last variable
- read/write lifecycle is defined conceptually in code structure
- no memory subsystem included

#### `CORE-005` Define error model and user-facing failures
Definition of done:
- major failure categories are modeled
- CLI can return explicit errors for unsupported type, missing var, invalid variable, invalid compare request

### Epic 2: Fixtures and Test Infrastructure
#### `TEST-001` Create fixture strategy for NetCDF and GeoTIFF
Definition of done:
- fixture locations are defined
- at least one sample NetCDF and one sample GeoTIFF are selected or planned
- fixture purpose is documented internally

#### `TEST-002` Create golden-output strategy
Definition of done:
- approach is defined for text golden outputs
- approach is defined for JSON verification
- numeric formatting normalization strategy is decided

#### `TEST-003` Add integration test harness for CLI commands
Definition of done:
- commands can be tested end to end
- stdout, stderr, and exit behavior can be asserted

### Epic 3: Inspect Command
#### `INSPECT-001` Implement file type detection for local files
Definition of done:
- local file path validation exists
- NetCDF and GeoTIFF are identified correctly
- unsupported formats fail explicitly

#### `INSPECT-002` Implement NetCDF essential metadata extraction
Definition of done:
- returns file type, variable names, dimensions, shape, dtype where available
- output stays intentionally minimal

#### `INSPECT-003` Implement GeoTIFF essential metadata extraction
Definition of done:
- returns file type, band count, width, height, dtype, nodata if available
- output stays intentionally minimal

#### `INSPECT-004` Implement `inspect` text renderer
Definition of done:
- text output is compact and stable
- output avoids noisy metadata

#### `INSPECT-005` Implement `inspect --json`
Definition of done:
- JSON output exists for both supported file types
- fields are minimal and explicit

#### `INSPECT-006` Add inspect integration and golden tests
Definition of done:
- text golden tests exist
- JSON assertions exist
- unsupported file test exists

### Epic 4: Mean Command
#### `MEAN-001` Implement NetCDF variable resolution rules
Definition of done:
- explicit `--var` path is supported
- missing or invalid variable fails explicitly
- no heuristic variable inference

#### `MEAN-002` Implement NetCDF mean computation
Definition of done:
- mean is computed across the full selected variable
- no dimension filtering yet
- result is deterministic

#### `MEAN-003` Implement GeoTIFF mean computation
Definition of done:
- mean is computed over valid pixels
- nodata is excluded by default when defined
- behavior is documented in output contract and tests

#### `MEAN-004` Implement `mean` text renderer
Definition of done:
- output includes file, type, variable if applicable, mean value
- numeric formatting is stable

#### `MEAN-005` Implement `mean --json`
Definition of done:
- JSON output exists for NetCDF and GeoTIFF means
- fields are consistent with inspect and compare style

#### `MEAN-006` Add mean correctness and failure tests
Definition of done:
- positive tests exist for NetCDF and GeoTIFF
- tests exist for missing `--var`, invalid var, unsupported file

### Epic 5: Compare Command
#### `COMPARE-001` Define compare validation rules
Definition of done:
- compare supports same-type only
- mixed-type compare fails explicitly
- NetCDF compare requires valid `--var`

#### `COMPARE-002` Implement NetCDF compare
Definition of done:
- computes mean for file A and file B
- computes `difference = mean_b - mean_a`
- output is scalar-summary only

#### `COMPARE-003` Implement GeoTIFF compare
Definition of done:
- computes mean for file A and file B independently
- computes `difference = mean_b - mean_a`
- no alignment, reprojection, or resampling logic exists

#### `COMPARE-004` Implement `compare` text renderer
Definition of done:
- output shows `mean_a`, `mean_b`, and `difference`
- direction is clearly labeled

#### `COMPARE-005` Implement `compare --json`
Definition of done:
- JSON output includes both summary values and difference
- direction is unambiguous

#### `COMPARE-006` Add compare correctness and failure tests
Definition of done:
- positive tests for NetCDF and GeoTIFF
- failure tests for mixed types, missing var, invalid var

### Epic 6: Session Foundation
#### `SESSION-001` Implement minimal session store
Definition of done:
- session can persist local state
- schema includes only agreed fields
- no memory concepts added

#### `SESSION-002` Persist command context on successful runs
Definition of done:
- workspace path is stored
- last variable is stored where applicable
- alias storage path is present if used internally

#### `SESSION-003` Add session-focused tests
Definition of done:
- successful command runs update session as expected
- invalid command runs do not corrupt session state

### Epic 7: Release Hardening
#### `REL-001` Normalize output formatting for stable golden tests
Definition of done:
- numeric precision and formatting are consistent
- text output is stable across runs

#### `REL-002` Document `v0.1` command semantics and limitations
Definition of done:
- supported formats are documented
- compare semantics are documented as scalar summary only
- nodata handling is documented
- JSON is noted as available but not yet strongly versioned

#### `REL-003` Validate developer-facing binary release workflow
Definition of done:
- binary can be built and run locally
- release expectations are documented for developers

### Epic 8: Session Commands
#### `SESSION-004` Add `session show` CLI command
Definition of done:
- CLI supports `geocode session show`
- current session state is rendered in text and JSON
- output includes workspace path, aliases, last variable, and session id if present

#### `SESSION-005` Add `session clear` CLI command
Definition of done:
- CLI supports `geocode session clear`
- persisted session state is reset safely
- command is idempotent

#### `SESSION-006` Decide or implement `session list`
Definition of done:
- `session list` is either implemented or explicitly deferred with rationale
- no ambiguous half-state remains in the roadmap

#### `SESSION-007` Move session storage to user-scoped app paths
Definition of done:
- session storage no longer depends on repo-local files
- storage uses `~/.config/geocode/` and app data path conventions
- migration or compatibility behavior is documented

#### `SESSION-008` Add session command integration tests
Definition of done:
- `session show` and `session clear` are test-covered
- session persistence survives command boundaries
- invalid states are handled explicitly

### Epic 9: Agent Contracts
#### `AGENT-001` Define planner request schema
Definition of done:
- structured planner request includes user input, session context, and available tools
- schema is narrow and command-oriented

#### `AGENT-002` Define planner response schema
Definition of done:
- planner output can represent inspect, mean, and compare intents
- arguments and target files or variables are explicit
- no multi-step chaining yet

#### `AGENT-003` Add agent CLI entrypoint
Definition of done:
- CLI supports `geocode ask <query>` or `geocode chat`
- agent entrypoint routes through the agent layer, not directly into command handlers

### Epic 10: OpenAI Provider
#### `PROVIDER-001` Add provider config model
Definition of done:
- config model exists for provider name, model, API key presence, and optional base URL

#### `PROVIDER-002` Add OpenAI planner client
Definition of done:
- minimal OpenAI integration exists for planner requests
- numeric execution remains outside the LLM

#### `PROVIDER-003` Add provider status command
Definition of done:
- CLI supports provider or auth status inspection
- user can determine whether OpenAI is configured

#### `PROVIDER-004` Add graceful unconfigured behavior
Definition of done:
- agent commands fail cleanly when OpenAI is not configured
- direct command mode remains unaffected
- output includes setup guidance

### Epic 11: First Agent Behavior
#### `AGENT-004` Map inspect-style natural language to inspect execution
Definition of done:
- simple inspect requests resolve into the existing inspect path

#### `AGENT-005` Map mean-style natural language to mean execution
Definition of done:
- simple mean requests resolve into the existing mean path
- explicit variable handling remains safe

#### `AGENT-006` Map compare-style natural language to compare execution
Definition of done:
- simple compare requests resolve into the existing compare path
- same-type and explicit-variable rules are preserved

#### `AGENT-007` Add agent integration tests
Definition of done:
- planner output is validated before execution
- agent requests run through the existing deterministic command or tool path

### Epic 12: Memory Model
#### `MEMORY-001` Define memory categories
Definition of done:
- session state, persistent preferences, and workspace facts are clearly separated
- raw chat logs are explicitly excluded

#### `MEMORY-002` Define memory storage schema
Definition of done:
- memory file format and storage path are defined
- stable facts only are persisted

#### `MEMORY-003` Define memory update policy
Definition of done:
- explicit rules define what is written after agent interactions
- low-signal noise is excluded

### Epic 13: Memory Commands
#### `MEMORY-004` Add `memory show`
Definition of done:
- CLI supports memory inspection in text and JSON

#### `MEMORY-005` Add `memory clear-session`
Definition of done:
- session-scoped memory can be reset independently

#### `MEMORY-006` Add `memory clear-all`
Definition of done:
- persistent memory can be fully reset explicitly

### Epic 14: Memory Integration
#### `MEMORY-007` Persist stable preferences
Definition of done:
- preferred output style, preferred provider or model, and stable alias conventions can be persisted
- no unstable conversational noise is stored

#### `MEMORY-008` Add memory-focused tests
Definition of done:
- writes are scoped correctly
- reset commands behave safely
- no raw chat dump behavior appears

### Epic 15: Additional Primitive Tools
#### `TOOLS-001` Add `min` summary command or tool
Definition of done:
- NetCDF and GeoTIFF support matches current mean scope where reasonable

#### `TOOLS-002` Add `max` summary command or tool
Definition of done:
- behavior is deterministic and documented

#### `TOOLS-003` Add `std` summary command or tool
Definition of done:
- scope is explicit and test-covered

### Epic 16: Structured Planning
#### `PLAN-001` Extend planner response for multi-step plans
Definition of done:
- planner schema supports ordered tool steps
- intermediate objects are explicit

#### `PLAN-002` Add plan validation layer
Definition of done:
- invalid or unsafe plans are rejected before execution

#### `PLAN-003` Add intermediate result references
Definition of done:
- tool outputs can be reused within a single plan execution
- references are explicit and typed

### Epic 17: Traceability
#### `TRACE-001` Add user-visible trace output
Definition of done:
- trace can be requested from CLI or agent mode
- each tool step is visible to the user

#### `TRACE-002` Add structured trace records
Definition of done:
- trace output is machine-readable
- errors are attributable to a specific step

### Epic 18: Richer Query Coverage
#### `AGENT-008` Support two-step agent flows
Definition of done:
- basic chained requests work
- command mode remains the baseline path

#### `AGENT-009` Add tool-chaining integration tests
Definition of done:
- multi-step execution is deterministic
- planner and executor contracts are tested together

### Epic 19: TUI Foundation
#### `TUI-001` Define TUI app shell
Definition of done:
- TUI module boundary is established
- it uses the existing app or service layer instead of duplicating execution logic

#### `TUI-002` Add query input and result panel
Definition of done:
- users can submit commands or queries interactively
- results reuse existing output models

#### `TUI-003` Add session and provider status widgets
Definition of done:
- current session and provider information is visible in the interface

### Epic 20: Exports
#### `EXPORT-001` Add CSV export path
Definition of done:
- scalar and tabular outputs can be exported where meaningful

#### `EXPORT-002` Add JSON export path
Definition of done:
- result objects can be written to JSON explicitly

#### `EXPORT-003` Add GeoJSON export decision point
Definition of done:
- a narrow GeoJSON export is either implemented or explicitly deferred with scope limits

### Epic 21: Visualization
#### `VIZ-001` Define browser visualization boundary
Definition of done:
- visualization is treated as a consumer of existing result types
- it is not a separate execution path

#### `VIZ-002` Add minimal map-opening workflow
Definition of done:
- implemented only when outputs are already well-defined
- no GIS-style UI scope creep is introduced

## 19) Recommended Sprint Order
### Sprint 1
- `BOOT-001` to `BOOT-005`
- `CORE-001` to `CORE-005`
- `TEST-001` to `TEST-003`

### Sprint 2
- `INSPECT-001` to `INSPECT-006`

### Sprint 3
- `MEAN-001` to `MEAN-006`

### Sprint 4
- `COMPARE-001` to `COMPARE-006`
- `SESSION-001` to `SESSION-003`

### Sprint 5
- `REL-001` to `REL-003`

### Sprint 6
- `SESSION-004` to `SESSION-008`

### Sprint 7
- `AGENT-001` to `AGENT-007`
- `PROVIDER-001` to `PROVIDER-004`

### Sprint 8
- `MEMORY-001` to `MEMORY-008`

### Sprint 9
- `TOOLS-001` to `TOOLS-003`
- `PLAN-001` to `PLAN-003`
- `TRACE-001` to `TRACE-002`
- `AGENT-008` to `AGENT-009`

### Sprint 10+
- `TUI-001` to `TUI-003`
- `EXPORT-001` to `EXPORT-003`
- `VIZ-001` to `VIZ-002`

## 20) Final Recommendation
Start with `inspect` before `mean` and `compare`. That will force the right file handling, metadata model, and output shape early, which reduces churn later.

The healthiest path remains:
- direct command mode first
- agent mode later
- memory later than session
- one provider later, not many providers now

GeoCode should earn trust through deterministic command behavior first, then expand into a richer agentic system on top of that foundation.
