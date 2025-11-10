# Project Rules & Processes
## SANKEY Copier Development Standards

**Created**: 2025-11-06
**Status**: Active
**Applies to**: All development work on this project

---

## 1. Documentation Requirements

### 1.1 Before Starting Any Work

**MUST DO**:
- ✅ Create detailed implementation plan in `.claude/plans/`
- ✅ Document current state and desired end state
- ✅ Break down work into small, trackable tasks
- ✅ Estimate time for each task
- ✅ Identify dependencies between tasks
- ✅ List all files that will be modified/created
- ✅ Define success criteria

**Format**: Markdown files with clear structure

**Naming Convention**: `phase-N-{feature-name}-plan.md`

**Example**: `.claude/plans/phase1-config-message-extension-plan.md`

---

### 1.2 During Work

**MUST DO**:
- ✅ Update progress in the plan document after each task
- ✅ Mark tasks as `[ ]` (pending), `[→]` (in progress), `[✓]` (completed)
- ✅ Add notes about issues encountered
- ✅ Document decisions made
- ✅ Commit progress updates regularly

**Update Frequency**: After completing each task or at least daily

**Example Progress Update**:
```markdown
## Task 3: Extend ConfigMessage struct
Status: [✓] Completed
Started: 2025-11-06 10:00
Completed: 2025-11-06 11:30
Notes:
- Added all CopySettings fields
- Added config_version for future compatibility
- Updated tests
Issues: None
```

---

### 1.3 After Completing Work

**MUST DO**:
- ✅ Mark phase as COMPLETED in plan document
- ✅ Add "Lessons Learned" section
- ✅ Document what worked well and what didn't
- ✅ Update overall project status in `.claude/PROJECT_STATUS.md`
- ✅ Archive plan to `.claude/plans/archive/` if fully complete

---

## 2. Code Quality Standards

### 2.1 Rust Code

**Requirements**:
- ✅ All code must compile without warnings
- ✅ Use `cargo fmt` before committing
- ✅ Use `cargo clippy` and fix all warnings
- ✅ Add comments for non-obvious logic
- ✅ Write unit tests for new functions
- ✅ Use meaningful variable names

**Error Handling**:
- Use `Result<T, E>` for fallible operations
- Use `?` operator for error propagation
- Log errors with `tracing::error!`
- Never use `.unwrap()` in production code (use `.expect()` with message)

---

### 2.2 MQL4/MQL5 Code

**Requirements**:
- ✅ Add header comment with function purpose
- ✅ Use consistent indentation (3 spaces)
- ✅ Add `Print()` statements for important events
- ✅ Handle errors gracefully
- ✅ Test on both MT4 and MT5 platforms

**Naming Convention**:
- Global variables: `g_variable_name`
- Functions: `FunctionName()` (PascalCase)
- Constants: `CONSTANT_NAME` (UPPER_SNAKE_CASE)

---

### 2.3 TypeScript/React Code

**Requirements**:
- ✅ Use TypeScript strict mode
- ✅ Define interfaces for all props
- ✅ Use functional components with hooks
- ✅ Add JSDoc comments for exported functions
- ✅ Use meaningful component names

---

## 3. Testing Requirements

### 3.1 Before Committing Code

**MUST TEST**:
- ✅ Compile and run locally
- ✅ Test happy path (normal operation)
- ✅ Test error cases
- ✅ Test edge cases (null, empty, large values)
- ✅ Test on target platform (MT4/MT5, browser)

---

### 3.2 Integration Testing

**For ZeroMQ Changes**:
- ✅ Start Rust server
- ✅ Start at least one Master EA and one Slave EA
- ✅ Create copy settings via Web UI
- ✅ Verify EA receives CONFIG message
- ✅ Verify trades are copied correctly
- ✅ Check logs for errors

**For Web UI Changes**:
- ✅ Test on Chrome, Firefox, Edge
- ✅ Test on mobile viewport
- ✅ Test all CRUD operations
- ✅ Test error states
- ✅ Check console for errors

---

## 4. Git Workflow

### 4.1 Branching Strategy

**Branch Naming**:
- Feature: `feature/phase1-config-extension`
- Bugfix: `bugfix/fix-zmq-crash`
- Hotfix: `hotfix/critical-security-fix`

**Branch Lifecycle**:
1. Create branch from `master`
2. Work on branch
3. Test thoroughly
4. Merge to `master`
5. Delete branch

---

### 4.2 Commit Messages

**Format**:
```
[Component] Brief description

- Detailed change 1
- Detailed change 2
- Detailed change 3

Refs: #issue-number (if applicable)
```

**Examples**:
```
[Rust] Extend ConfigMessage with full CopySettings

- Added lot_multiplier, reverse_trade, filters fields
- Added config_version for compatibility
- Updated send_config_to_ea() to populate all fields
- Added unit tests

[MQL5] Add CONFIG message parsing for new fields

- Parse lot_multiplier and reverse_trade
- Parse allowed/blocked symbols and magic numbers
- Apply filters before executing trades
- Add logging for config updates

[Web UI] Implement sidebar filter for master accounts

- Created MasterAccountSidebar component
- Added filter state management
- Implemented responsive mobile drawer
- Added animations for filter changes
```

---

### 4.3 When to Commit

**Commit Frequency**: Small, logical units

**Good Reasons to Commit**:
- ✅ Completed one task from plan
- ✅ Fixed one bug
- ✅ Added one feature
- ✅ Refactored one component

**Bad Reasons**:
- ❌ "End of day" (commit incomplete work)
- ❌ "Lots of changes" (too big)
- ❌ "WIP" (work in progress - use branches instead)

---

## 5. Progress Tracking System

### 5.1 Task Status Markers

**In Plan Documents**:
- `[ ]` - Not started
- `[→]` - In progress (only ONE task should be in progress at a time)
- `[✓]` - Completed
- `[✗]` - Cancelled/Skipped (with reason)
- `[?]` - Blocked (with blocker description)

**Example**:
```markdown
## Phase 1 Tasks

### Backend (Rust)
- [✓] Task 1: Extend ConfigMessage struct
- [→] Task 2: Update send_config_to_ea() function
- [ ] Task 3: Add unit tests
- [?] Task 4: Integration test (blocked: need EA setup)
```

---

### 5.2 Progress Document Structure

**Every plan MUST have**:

```markdown
# Phase N: {Feature Name}

## Overview
- **Phase**: N
- **Priority**: High/Medium/Low
- **Estimated Duration**: X days
- **Started**: YYYY-MM-DD
- **Target Completion**: YYYY-MM-DD
- **Actual Completion**: YYYY-MM-DD (when done)
- **Status**: Planning / In Progress / Testing / Completed / Blocked

## Progress Summary
Current: Task X of Y (Z% complete)
Last Updated: YYYY-MM-DD HH:MM

## Tasks
[List with status markers]

## Daily Updates
### 2025-11-06
- Completed: Task 1, Task 2
- In Progress: Task 3
- Blocked: None
- Notes: Encountered issue with X, resolved by Y

### 2025-11-07
- Completed: Task 3
- In Progress: Task 4
- Issues: Z needs investigation

## Completion Checklist
Before marking phase as complete:
- [ ] All tasks completed
- [ ] All tests passing
- [ ] Documentation updated
- [ ] Code reviewed
- [ ] No known bugs
- [ ] Performance acceptable

## Lessons Learned
(Added after completion)
```

---

### 5.3 Progress Updates - WHO, WHEN, HOW

**WHO**:
- Primary developer working on the task
- Anyone who makes changes to code in this phase

**WHEN**:
- After completing each task
- Before taking a break (end of work session)
- When switching to different work
- At least once per day

**HOW**:
1. Open the plan document
2. Update task status markers
3. Add entry to "Daily Updates" section
4. Update "Progress Summary" percentage
5. Save and commit document

---

## 6. File Organization

### 6.1 Directory Structure

```
.claude/
├── PROJECT_RULES.md           ← This file
├── PROJECT_STATUS.md          ← Overall project status
├── design/                    ← Design documents (read-only reference)
│   ├── sidebar-filter-ux.md
│   ├── config-distribution-architecture.md
│   └── zeromq-current-implementation-analysis.md
├── plans/                     ← Active implementation plans
│   ├── phase1-config-message-extension-plan.md
│   ├── phase2-registration-config-plan.md
│   └── phase3-sidebar-filter-ux-plan.md
└── plans/archive/             ← Completed plans
    └── phase1-config-message-extension-plan.md
```

---

### 6.2 Document Naming

**Design Docs**: `{topic}-{type}.md`
- Example: `sidebar-filter-ux.md`, `zeromq-current-implementation-analysis.md`

**Plan Docs**: `phase{N}-{feature-name}-plan.md`
- Example: `phase1-config-message-extension-plan.md`

**Status Docs**: `PROJECT_STATUS.md`, `CHANGELOG.md`

---

## 7. Handoff Process

### 7.1 When Stopping Work

**MUST DO before stopping**:
1. ✅ Update plan document with current status
2. ✅ Commit all changes (even if incomplete)
3. ✅ Push to remote repository
4. ✅ Add "HANDOFF" section to plan with:
   - Current state
   - Next steps
   - Known issues
   - How to resume

**Example HANDOFF Section**:
```markdown
## HANDOFF (2025-11-06 18:00)

### Current State
- Completed: Tasks 1-3
- In Progress: Task 4 (50% done)
- Files modified: connection.rs, zeromq/mod.rs

### Next Steps
1. Finish implementing send_config_to_ea() function
2. Add error handling for missing fields
3. Write unit tests

### Known Issues
- Compilation error in line 145 of api/mod.rs (missing import)
- Need to verify JSON serialization format

### How to Resume
1. Run: cargo build
2. Fix compilation error (add `use crate::models::TradeFilters;`)
3. Continue with Task 4
```

---

### 7.2 When Resuming Work

**MUST DO before starting**:
1. ✅ Read plan document from start
2. ✅ Review "HANDOFF" section (if exists)
3. ✅ Pull latest changes from repository
4. ✅ Verify local environment (compile, run tests)
5. ✅ Update plan with "Resumed: YYYY-MM-DD HH:MM"

---

## 8. Communication Standards

### 8.1 Asking for Help

**When Blocked**:
1. Mark task as `[?]` in plan
2. Add "Blocker" note with detailed description
3. Document what you've tried
4. Ask specific question with context

**Example**:
```markdown
- [?] Task 5: Implement symbol mapping logic
  Blocker: Unclear how symbol_mappings array should be serialized to JSON
  Tried: Used Vec<SymbolMapping> but EA can't parse nested objects
  Question: Should we flatten to key-value pairs or use different format?
```

---

### 8.2 Reporting Issues

**Bug Report Template**:
```markdown
## Bug Report

**Component**: Rust Server / MT5 EA / Web UI
**Severity**: Critical / High / Medium / Low
**Discovered**: YYYY-MM-DD
**Status**: Open / In Progress / Fixed

**Description**:
Brief description of the bug

**Steps to Reproduce**:
1. Step 1
2. Step 2
3. Step 3

**Expected Behavior**:
What should happen

**Actual Behavior**:
What actually happens

**Logs/Screenshots**:
[Attach relevant logs]

**Potential Cause**:
(If known)

**Potential Solution**:
(If known)
```

---

## 9. Quality Checklist

### 9.1 Before Marking Task as Complete

**Every Task MUST**:
- [ ] Code compiles without errors
- [ ] Code compiles without warnings
- [ ] Functionality tested manually
- [ ] Edge cases considered
- [ ] Error handling implemented
- [ ] Logging added
- [ ] Comments added for complex logic
- [ ] No debugging code left (console.log, Print() for testing)
- [ ] Performance acceptable
- [ ] Plan document updated

---

### 9.2 Before Marking Phase as Complete

**Every Phase MUST**:
- [ ] All tasks completed
- [ ] All tests passing (unit + integration)
- [ ] No known bugs
- [ ] Documentation updated
- [ ] Code formatted and linted
- [ ] Git history clean (meaningful commits)
- [ ] "Lessons Learned" section filled
- [ ] Handoff not required (or completed)
- [ ] Next phase ready to start

---

## 10. Emergency Procedures

### 10.1 Critical Bug in Production

**Steps**:
1. Create `hotfix/critical-{bug-name}` branch
2. Fix bug with minimal changes
3. Test thoroughly
4. Commit with `[HOTFIX]` prefix
5. Merge to master immediately
6. Document in `CHANGELOG.md`
7. Create follow-up task for proper fix

---

### 10.2 Lost Work / Corrupted Files

**Prevention**:
- Commit often
- Push to remote daily
- Keep backup of `.claude/` directory

**Recovery**:
1. Check git history: `git log --all --full-history`
2. Restore from previous commit: `git checkout <commit> -- <file>`
3. If not in git, check editor auto-save
4. Worst case: Refer to design docs and rebuild

---

## 11. Review & Approval

### 11.1 Code Review

**Before Merging**:
- Self-review all changes
- Run full test suite
- Check plan completion checklist
- Verify documentation updated

**Optional** (for major changes):
- Ask for peer review
- Address feedback
- Re-test

---

### 11.2 Phase Approval

**Phase is approved when**:
- ✅ All tasks completed
- ✅ All quality checks passed
- ✅ Documentation complete
- ✅ Plan marked as COMPLETED
- ✅ Project status updated

---

## 12. Continuous Improvement

### 12.1 After Each Phase

**Update Rules**:
- Add new lessons learned
- Update estimates based on actual time
- Improve templates
- Add common pitfalls section

---

### 12.2 Rule Violations

**If Rules Not Followed**:
- Document why (technical blocker vs oversight)
- Update rules if justified
- Retrofit documentation if missed

**Common Valid Reasons**:
- Emergency hotfix
- Exploratory spike (create plan afterward)
- External dependency blocked

---

## Summary

**Core Principles**:
1. 📝 **Document Everything** - Plans, progress, decisions
2. 🔄 **Update Frequently** - After every task, daily minimum
3. 🎯 **One Task at a Time** - Focus, complete, move on
4. ✅ **Quality First** - Test thoroughly, handle errors
5. 🤝 **Enable Handoff** - Anyone can resume anytime
6. 📊 **Track Progress** - Clear status, clear next steps

**Violation = Technical Debt**

Following these rules ensures:
- Project continuity
- Knowledge preservation
- Quality assurance
- Efficient handoffs
- Clear accountability

---

**Last Updated**: 2025-11-06
**Next Review**: After Phase 1 completion
