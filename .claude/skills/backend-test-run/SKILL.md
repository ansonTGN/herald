---
name: backend-test-run
description: Backend test runner and fix orchestrator. Use this skill for runner items, backend test execution, failure diagnosis, "uv run scripts/backend-test.py", and fix/retest loops. It runs targeted tests, classifies failures, delegates production-code fixes to backend-dev, and protects scenario-test semantics.
tools:
  - Bash
  - Read
  - Write
  - Edit
  - Task
  - Agent
---

# Backend Test Run

## Priority

`AGENTS.md` is the highest-priority rule source. This skill defines the backend test runner workflow; if it conflicts with the current user request, User Story/PRD, spec, or code facts, stop and report the conflict instead of blending rules.

This skill is the only backend test execution, diagnosis, and fix orchestration entry point.

It must not write new scenario tests or change business acceptance semantics. Scenario authoring belongs to `backend-test` authoring items.

## Core Workflow

### 1. Analyze Scope

```bash
git status
git diff --name-only
```

Map changes to the narrowest reliable command:
- Single test/helper impact -> `uv run scripts/backend-test.py -- <test_name>`
- Single crate/module impact -> `uv run scripts/backend-test.py -- -E 'package(<crate>)'`
- API-layer impact -> `uv run scripts/backend-test.py -- -E 'package(api)'`
- Multiple local impacts -> `uv run scripts/backend-test.py -- -E 'package(<crate>) and test(<pattern>)'`
- Cross-crate or unclear impact -> document why targeted scope is insufficient, then escalate to full `uv run scripts/backend-test.py`

Full-suite execution is escalation, not the default.

### 2. Run Targeted Tests

```bash
uv run scripts/backend-test.py -- <targeted filter>
```

Common filter forms:

```bash
uv run scripts/backend-test.py -- <test_name>
uv run scripts/backend-test.py -- -E 'package(<crate>)'
uv run scripts/backend-test.py -- -E 'test(<pattern>)'
uv run scripts/backend-test.py -- -E 'package(<crate>) and test(<pattern>)'
```

### 3. Parse Failures

Classify failures as:
- Compilation errors: syntax, imports, module registration, type mismatch.
- Runtime errors: panic, unwrap, index error, infrastructure issue.
- Assertion failures: behavior differs from expectation.
- Environment failures: database, Redis, Docker, network, timeout.

Record: command, test name, file/line, failure type, key message, consulted User Story/PRD.

### 4. Resolve Ownership

Conflict priority:

```text
User Story > PRD > Existing Tests > Current Implementation
```

Rules:
- Implementation contradicts User Story/PRD -> delegate production-code fix to `backend-dev`.
- Test has mechanical breakage -> runner may fix imports, module registration, helper call signatures, or obvious path mistakes.
- Test semantics may be wrong -> stop and output a diagnostic report. Do not change assertions, status-code expectations, permission expectations, or business-rule expectations.
- User Story/PRD is unclear -> stop and request clarification.

The runner may update tests only for mechanical issues. It must not weaken or rewrite acceptance semantics.

### 5. Delegate Production Fixes

When delegating to `backend-dev`, use `Agent(subagent_type="backend-dev")` and the prompt must include:

```markdown
Task: Fix this backend test failure in production code.

Test command: `<command>`
Failing test: `<test_name>`
Failure: `<message>`
Relevant docs: `<User Story/PRD paths>`
Reason implementation appears wrong: `<diagnosis>`

Hard constraints:
- Do not modify `backend/**/tests/scenarios/**`.
- Do not modify any `*_scenarios.rs`.
- Do not change scenario-test assertions, status-code expectations, permission expectations, or business-rule expectations.
- If a test semantics change seems required, return `requires_test_semantics_change` with evidence instead of editing tests.
```

After `backend-dev` returns, rerun the targeted command.

### 6. Verify and Report

Rerun targeted tests first:

```bash
uv run scripts/backend-test.py -- <targeted filter>
```

Escalate to full suite only when:
- User explicitly asks for full coverage.
- Targeted scope can no longer bound the impact.
- A shared infrastructure or cross-crate change makes narrow verification unreliable.

Report:
- Commands executed.
- Pass/fail counts.
- Fixes applied and files changed.
- Any stopped semantic conflict or unclear spec.
- Whether full-suite escalation was used and why.

## Stop Report for Test Semantics Conflict

Use this when the test expectation itself may be wrong:

```markdown
# Backend Test Run Blocked: Test Semantics Need Decision

## Failure
- Command: `<command>`
- Test: `<test_name>`
- Assertion: `<expected vs actual>`

## Evidence
- User Story: `<path and section>`
- PRD: `<path and section>`
- Existing tests: `<paths>`

## Diagnosis
The runner cannot safely decide whether to change test semantics or implementation.

## Required Decision
Choose whether to return this to `backend-test` for scenario correction or send it to `backend-dev` for implementation correction.
```

## Success Criteria

- Targeted tests pass, or a blocking diagnostic report is produced.
- No compilation errors remain in targeted scope.
- Production-code fixes were delegated with scenario-test write restrictions.
- Test semantics were not changed by the runner.
