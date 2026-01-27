# Project Workflow

## Guiding Principles

1. **The Plan is the Source of Truth:** All work must be tracked in `plan.md`
2. **The Tech Stack is Deliberate:** Changes to the tech stack must be documented in `tech-stack.md` *before* implementation
3. **Test-Driven Development:** Write unit tests before implementing functionality
4. **High Code Coverage:** Aim for >80% code coverage for all modules
5. **User Experience First:** Every decision should prioritize user experience
6. **Non-Interactive & CI-Aware:** Prefer non-interactive commands. Use `CI=true` for watch-mode tools (tests, linters) to ensure single execution.

## Task Workflow

All tasks follow a strict lifecycle:

### Standard Task Workflow

1. **Select Task:** Choose the next available task from `plan.md` in sequential order

2. **Mark In Progress:** Before beginning work, edit `plan.md` and change the task from `[ ]` to `[~]`

3. **Write Failing Tests (Red Phase):**
   - Create a new test file for the feature or bug fix.
   - Write one or more unit tests that clearly define the expected behavior and acceptance criteria for the task.
   - **CRITICAL:** Run the tests and confirm that they fail as expected. This is the "Red" phase of TDD. Do not proceed until you have failing tests.

4. **Implement to Pass Tests (Green Phase):**
   - Write the minimum amount of application code necessary to make the failing tests pass.
   - Run the test suite again and confirm that all tests now pass. This is the "Green" phase.

5. **Refactor (Optional but Recommended):**
   - With the safety of passing tests, refactor the implementation code and the test code to improve clarity, remove duplication, and enhance performance without changing the external behavior.
   - Rerun tests to ensure they still pass after refactoring.

6. **Verify Coverage:** Run coverage reports using the project's tooling:
   ```bash
   # Terminal summary
   cargo xtask coverage

   # HTML report (opens in browser)
   cargo xtask coverage --html

   # LCOV report for CI integration
   cargo xtask coverage --lcov
   ```
   Target: >80% coverage for new code. Requires `cargo-llvm-cov` (install with `cargo install cargo-llvm-cov`).

7. **Document Deviations:** If implementation differs from tech stack:
   - **STOP** implementation
   - Update `tech-stack.md` with new design
   - Add dated note explaining the change
   - Resume implementation

8. **Commit Code Changes:**
   - Stage all code changes related to the task.
   - Propose a clear, concise commit message e.g, `feat(ui): Create basic HTML structure for calculator`.
   - Perform the commit.

9. **Attach Task Summary with Git Notes:**
   - **Step 9.1: Get Commit Hash:** Obtain the hash of the *just-completed commit* (`git log -1 --format="%H"`).
   - **Step 9.2: Draft Note Content:** Create a detailed summary for the completed task. This should include the task name, a summary of changes, a list of all created/modified files, and the core "why" for the change.
   - **Step 9.3: Attach Note:** Use the `git notes` command to attach the summary to the commit.
     ```bash
     # The note content from the previous step is passed via the -m flag.
     git notes add -m "<note content>" <commit_hash>
     ```

10. **Get and Record Task Commit SHA:**
    - **Step 10.1: Update Plan:** Read `plan.md`, find the line for the completed task, update its status from `[~]` to `[x]`, and append the first 7 characters of the *just-completed commit's* commit hash.
    - **Step 10.2: Write Plan:** Write the updated content back to `plan.md`.

11. **Commit Plan Update:**
    - **Action:** Stage the modified `plan.md` file.
    - **Action:** Commit this change with a descriptive message (e.g., `conductor(plan): Mark task 'Create user model' as complete`).

### Phase Completion Verification and Checkpointing Protocol

**Trigger:** This protocol is executed immediately after a task is completed that also concludes a phase in `plan.md`.

1.  **Announce Protocol Start:** Inform the user that the phase is complete and the verification and checkpointing protocol has begun.

2.  **Ensure Test Coverage for Phase Changes:**
    -   **Step 2.1: Determine Phase Scope:** To identify the files changed in this phase, you must first find the starting point. Read `plan.md` to find the Git commit SHA of the *previous* phase's checkpoint. If no previous checkpoint exists, the scope is all changes since the first commit.
    -   **Step 2.2: List Changed Files:** Execute `git diff --name-only <previous_checkpoint_sha> HEAD` to get a precise list of all files modified during this phase.
    -   **Step 2.3: Verify and Create Tests:** For each file in the list:
        -   **CRITICAL:** First, check its extension. Exclude non-code files (e.g., `.json`, `.md`, `.yaml`).
        -   For each remaining code file, verify a corresponding test file exists.
        -   If a test file is missing, you **must** create one. Before writing the test, **first, analyze other test files in the repository to determine the correct naming convention and testing style.** The new tests **must** validate the functionality described in this phase's tasks (`plan.md`).

3.  **Execute Automated Tests with Proactive Debugging:**
    -   Before execution, you **must** announce the exact shell command you will use to run the tests.
    -   **Example Announcement:** "I will now run the automated test suite to verify the phase. **Command:** `CI=true npm test`"
    -   Execute the announced command.
    -   If tests fail, you **must** inform the user and begin debugging. You may attempt to propose a fix a **maximum of two times**. If the tests still fail after your second proposed fix, you **must stop**, report the persistent failure, and ask the user for guidance.

4.  **Propose a Detailed, Actionable Manual Verification Plan:**
    -   **CRITICAL:** To generate the plan, first analyze `product.md`, `product-guidelines.md`, and `plan.md` to determine the user-facing goals of the completed phase.
    -   You **must** generate a step-by-step plan that walks the user through the verification process, including any necessary commands and specific, expected outcomes.
    -   The plan you present to the user **must** follow this format:

        **For a Frontend Change:**
        ```
        The automated tests have passed. For manual verification, please follow these steps:

        **Manual Verification Steps:**
        1.  **Start the development server with the command:** `npm run dev`
        2.  **Open your browser to:** `http://localhost:3000`
        3.  **Confirm that you see:** The new user profile page, with the user's name and email displayed correctly.
        ```

        **For a Backend Change:**
        ```
        The automated tests have passed. For manual verification, please follow these steps:

        **Manual Verification Steps:**
        1.  **Ensure the server is running.**
        2.  **Execute the following command in your terminal:** `curl -X POST http://localhost:8080/api/v1/users -d '{"name": "test"}'`
        3.  **Confirm that you receive:** A JSON response with a status of `201 Created`.
        ```

5.  **Await Explicit User Feedback:**
    -   After presenting the detailed plan, ask the user for confirmation: "**Does this meet your expectations? Please confirm with yes or provide feedback on what needs to be changed.**"
    -   **PAUSE** and await the user's response. Do not proceed without an explicit yes or confirmation.

6.  **Create Checkpoint Commit:**
    -   Stage all changes. If no changes occurred in this step, proceed with an empty commit.
    -   Perform the commit with a clear and concise message (e.g., `conductor(checkpoint): Checkpoint end of Phase X`).

7.  **Attach Auditable Verification Report using Git Notes:**
    -   **Step 7.1: Draft Note Content:** Create a detailed verification report including the automated test command, the manual verification steps, and the user's confirmation.
    -   **Step 7.2: Attach Note:** Use the `git notes` command and the full commit hash from the previous step to attach the full report to the checkpoint commit.

8.  **Get and Record Phase Checkpoint SHA:**
    -   **Step 8.1: Get Commit Hash:** Obtain the hash of the *just-created checkpoint commit* (`git log -1 --format="%H"`).
    -   **Step 8.2: Update Plan:** Read `plan.md`, find the heading for the completed phase, and append the first 7 characters of the commit hash in the format `[checkpoint: <sha>]`.
    -   **Step 8.3: Write Plan:** Write the updated content back to `plan.md`.

9. **Commit Plan Update:**
    - **Action:** Stage the modified `plan.md` file.
    - **Action:** Commit this change with a descriptive message following the format `conductor(plan): Mark phase '<PHASE NAME>' as complete`.

10.  **Announce Completion:** Inform the user that the phase is complete and the checkpoint has been created, with the detailed verification report attached as a git note.

### Quality Gates

Before marking any task complete, verify:

- [ ] All tests pass
- [ ] Code coverage meets requirements (>80%)
- [ ] Code follows project's code style guidelines (as defined in `code_styleguides/`)
- [ ] All public functions/methods are documented (e.g., docstrings, JSDoc, GoDoc)
- [ ] Type safety is enforced (e.g., type hints, TypeScript types, Go types)
- [ ] No linting or static analysis errors (using the project's configured tools)
- [ ] Works correctly on mobile (if applicable)
- [ ] Documentation updated if needed
- [ ] No security vulnerabilities introduced

## Development Commands

### Setup
```bash
# Install Rust toolchain (Edition 2024, MSRV 1.88)
rustup update stable
rustup component add clippy rustfmt

# Build all workspace crates
cargo build --workspace

# For client development (Tauri 2.0 apps)
cd clients/wraith-chat && npm install && npm run tauri dev
cd clients/wraith-transfer && npm install && npm run tauri dev
cd clients/wraith-sync && npm install && cd frontend && npm install && cd .. && npm run tauri:dev
```

### Daily Development
```bash
# Build and test
cargo build --workspace
cargo test --workspace
cargo test -p wraith-core           # Test a specific crate
cargo test -p wraith-crypto -- --nocapture  # With output

# Run the CLI
cargo run -p wraith-cli -- --help

# Generate documentation
cargo doc --workspace --no-deps --open
```

### Before Committing
```bash
# Full CI pipeline (format check + clippy + tests)
cargo xtask ci

# Or run individually:
cargo fmt --all -- --check           # Verify formatting
cargo clippy --workspace -- -D warnings  # Zero-warning policy
cargo test --all-features --workspace    # All tests with all features
```

## Testing Requirements

### Unit Testing (Rust)
- Every module must have a corresponding `#[cfg(test)] mod tests` block.
- Use `#[test]` for synchronous tests, `#[tokio::test]` for async tests.
- Test both success and error paths for all `Result`-returning functions.
- Use `proptest` for property-based testing of protocol invariants (frame encoding/decoding, crypto operations).
- Use `criterion` for benchmarking performance-critical paths (frame parsing, hashing, encryption).
- Test naming convention: `test_<function_name>_<scenario>` (e.g., `test_bbr_initial_state`, `test_ct_eq_different_lengths`).
- Mock external dependencies using trait objects or conditional compilation.

### Integration Testing
- Integration tests reside in the `tests/` workspace crate.
- Test complete protocol flows: handshake, session establishment, file transfer, rekeying.
- Verify cryptographic operations end-to-end (encrypt-then-decrypt round-trip).
- Test concurrent stream multiplexing under load.
- Verify BBR congestion control behavior under simulated network conditions.

### Client Testing
- **Desktop (Tauri):** Test IPC command handlers in Rust backend; test React components with TypeScript test frameworks.
- **Android:** JNI binding tests, Keystore integration tests (require device or emulator).
- **iOS:** UniFFI binding tests, Keychain integration tests (require device or simulator).
- **Mobile Network:** Test DHT discovery and NAT traversal under mobile network conditions (3G/4G/5G).

### Coverage Targets
- Protocol crates: >80% line coverage (measured via `cargo xtask coverage`).
- New code: >80% coverage required before merge.
- Cryptographic code: 100% branch coverage for all security-critical paths.

## Code Review Process

### Self-Review Checklist
Before requesting review:

1. **Functionality**
   - Feature works as specified
   - Edge cases handled (malformed packets, oversized payloads, invalid state transitions)
   - Error messages are descriptive with context (using `thiserror` structured variants)

2. **Code Quality**
   - Follows the [Rust style guide](./code_styleguides/rust.md) and [general principles](./code_styleguides/general.md)
   - Zero clippy warnings (`cargo clippy --workspace -- -D warnings`)
   - Consistent formatting (`cargo fmt --all -- --check`)
   - All public APIs documented with `///` doc comments
   - `#[must_use]` on pure functions returning values

3. **Testing**
   - Unit tests in `#[cfg(test)] mod tests` block
   - Property-based tests for protocol invariants (`proptest`)
   - Integration tests for cross-crate interactions
   - Coverage adequate (>80% for new code)

4. **Security**
   - No hardcoded secrets, keys, or connection parameters
   - All cryptographic operations use constant-time comparisons (`subtle` crate)
   - Key material zeroized on drop (`zeroize` derive macro)
   - Nonces never reused (monotonic counters or random generation)
   - Input validation on all untrusted data (packet parsing, frame headers)
   - No information leakage in error messages (no secret data in `Display` impls)

5. **Performance**
   - Zero-copy parsing where possible (reference into buffers, not clone)
   - No allocations in hot paths (frame processing, encryption)
   - NUMA-aware allocation for transport workers
   - Benchmark results for performance-critical changes (`criterion`)

6. **Unsafe Code**
   - Minimal `unsafe` blocks with clear `// SAFETY:` comments
   - `# Safety` section in doc comments for `unsafe fn`
   - `#![deny(unsafe_op_in_unsafe_fn)]` in all crate roots

## Commit Guidelines

### Message Format
```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `test`: Adding missing tests
- `chore`: Maintenance tasks

### Examples
```bash
git commit -m "feat(auth): Add remember me functionality"
git commit -m "fix(posts): Correct excerpt generation for short posts"
git commit -m "test(comments): Add tests for emoji reaction limits"
git commit -m "style(mobile): Improve button touch targets"
```

## Definition of Done

A task is complete when:

1. All code implemented to specification
2. Unit tests written and passing
3. Code coverage meets project requirements
4. Documentation complete (if applicable)
5. Code passes all configured linting and static analysis checks
6. Works beautifully on mobile (if applicable)
7. Implementation notes added to `plan.md`
8. Changes committed with proper message
9. Git note with task summary attached to the commit

## Emergency Procedures

### Critical Bug in Production
1. Create hotfix branch from main
2. Write failing test reproducing the bug
3. Implement minimal fix with full test coverage
4. Run `cargo xtask ci` to verify no regressions
5. Build release binaries and verify
6. Document in plan.md with root cause analysis

### Cryptographic Vulnerability
1. Assess impact scope (which crate, which protocol layer)
2. Determine if key material may have been compromised
3. If keys compromised: force ratchet on all active sessions
4. Patch vulnerability with constant-time fix
5. Verify fix does not introduce timing side-channels
6. Run full cryptographic test suite
7. Document in `docs/security/` with CVE reference if applicable

### Security Breach
1. Rotate all secrets and key material immediately
2. Trigger kill switch for affected RedOps deployments (Ed25519-signed halt)
3. Review audit logs (tamper-evident Merkle chain in WRAITH-Recon)
4. Patch vulnerability
5. Notify affected users
6. Document and update security procedures in `SECURITY.md`

## Deployment Workflow

### Pre-Deployment Checklist
- [ ] All 2,140+ tests passing (`cargo test --workspace`)
- [ ] Zero clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] Formatting verified (`cargo fmt --all -- --check`)
- [ ] Coverage >80% for new code
- [ ] No `TODO`, `FIXME`, or `unimplemented!()` in production paths
- [ ] All `unsafe` blocks documented with `// SAFETY:` comments
- [ ] Version bumped in workspace `Cargo.toml`
- [ ] CHANGELOG.md updated with release notes
- [ ] Release build succeeds (`cargo build --release`)
- [ ] Security audit passes (zero known vulnerabilities)

### Release Build Steps
1. Run full CI pipeline: `cargo xtask ci`
2. Build release binaries: `cargo build --release`
3. Tag release with semantic version: `git tag -a v2.2.5 -m "Release v2.2.5"`
4. Build Tauri desktop clients: `npm run tauri build` (per client)
5. Verify release artifacts (binary size, feature flags)
6. Test critical paths with release binaries
7. Push tag to remote: `git push origin v2.2.5`

### Post-Release
1. Verify CI/CD pipeline completes successfully
2. Monitor for regression reports
3. Update documentation if needed
4. Plan next development phase

## Continuous Improvement

- Review workflow weekly
- Update based on pain points
- Document lessons learned
- Optimize for user happiness
- Keep things simple and maintainable
