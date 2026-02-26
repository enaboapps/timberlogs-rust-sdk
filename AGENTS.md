# Timberlogs Rust SDK Development Guidelines for AI Agents

## Project Overview
Rust SDK for Timberlogs structured logging service. Provides async log ingestion, flow tracking, and raw format support via the Timberlogs ingest API.

## Project Structure
```
timberlogs-rust-sdk/
├── src/
│   ├── lib.rs         # Public exports
│   ├── client.rs      # TimberlogsClient, Flow, flush logic
│   ├── types.rs       # LogEntry, LogLevel, Environment, RawFormat, etc.
│   └── error.rs       # TimberlogsError enum
├── tests/
│   └── client_test.rs # Integration tests (mockito-based)
├── examples/
│   └── basic.rs       # Usage example
├── Cargo.toml
└── .github/workflows/
    ├── ci.yml         # Build, test, clippy, fmt on PR/push
    └── release.yml    # Trusted publishing to crates.io on tags
```

## Code Style & Standards

### Rust Conventions
- Follow existing code patterns and naming conventions
- NO comments unless explicitly requested
- Use meaningful variable and function names that are self-documenting
- Keep `pub` exports minimal; use `pub(crate)` for internal types
- All public methods on `TimberlogsClient` are `async`
- Use `impl Into<String>` for string parameters

### Testing
- Always run `cargo test` before committing
- Run `cargo clippy -- -D warnings` for lint checks
- Run `cargo fmt --check` for formatting
- Use `mockito` for HTTP mock tests
- Use `mock_config()` helper for tests needing a mock server

### Security Best Practices
- Never introduce code that exposes or logs secrets/keys
- Never commit secrets or keys to repository
- Never commit `.env` files or API tokens

## Development Workflow

### Required Workflow Process
**MANDATORY**: Follow this exact workflow for ALL development tasks:

1. **Issue Creation**: Create GitHub issue (with user consent)
2. **Branch Creation**: Create branch `feature/description-#` or `fix/description-#` from `dev`
3. **Development**: Make changes following code standards
4. **Testing**: Test with `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
5. **Commit & Push**: Commit with proper format and push branch
6. **Pull Request**: Create PR to `dev` with detailed description
7. **User Approval**: Ask user if they want to merge (yes/no)
8. **Proactive Communication**: Always ask yes/no for perceived next steps

### Branch & Issue Management
- Create descriptive branch names: `feature/description-1234` or `fix/description-1234`
- Always create GitHub issues before starting work
- Use GitHub CLI (`gh`) for all GitHub operations
- Default branch is `dev`; all normal work targets `dev`
- Never push directly to `main`; use PRs only
- Treat `main` as release-only and merge into `main` only when explicitly requested by the repository owner

### Commit Standards
Follow this exact format:
```
Brief descriptive title

- Bullet points describing changes
- Focus on what and why, not just what
- Include technical details relevant to reviewers

🤖 Auto-generated
```

### Pull Request Process
1. Create GitHub issue (with user consent)
2. Create feature branch from dev
3. Make changes following code standards
4. Test with `cargo test`
5. Commit with proper format
6. Push branch and create PR to `dev` with detailed description
7. Include test plan in PR description
8. Ask user if they want to merge (yes/no)

### Protected Branch Strategy
- `dev` is the default integration branch
- `dev` and `main` are protected branches
- Require PRs and dismiss stale reviews
- Enforce admin restrictions
- Require conversation resolution before merge
- Never bypass branch protection settings

### Testing Requirements
- Always run `cargo test` before committing
- All 41+ tests must pass before merging
- Build must be successful before merging

## Tool Usage Patterns

### Preferred Tools
- Use `Bash` tool for git operations and builds
- Use `Grep` for searching code (never bash grep/rg)
- Use `Glob` for file pattern matching
- Use `Read` tool for examining files
- Use `Edit` for code changes

### File Operations
- ALWAYS prefer editing existing files over creating new ones
- NEVER create documentation files unless explicitly requested
- Use absolute paths, not relative paths
- Read files before editing to understand context

## Communication Style
- Be concise and direct (under 4 lines typically)
- Don't add unnecessary preamble or explanations
- Focus on answering the specific question asked
- One-word answers are fine when appropriate
- Avoid "Here is what I will do" or similar verbose intros

## Version & Release Management

### Release Process
1. Bump version in `Cargo.toml`
2. Update README install snippet if major version changes
3. Create release branch from `dev`, commit version changes, PR to `main`
4. Merge with merge commit (never squash release PRs)
5. Tag `v*.*.*` and push tags
6. GitHub Action publishes to crates.io via trusted publishing

### Release Merge Strategy
- Never squash-merge release PRs into `main`
- Prefer merge commits for `dev` -> `main` releases
- For `main` -> `dev` sync PRs, use a merge commit

## Deployment
- **crates.io**: Auto-published via trusted publishing on version tags (`v*`)
- **CI**: Build, test, clippy, fmt on all PRs and pushes to `main`/`dev`
