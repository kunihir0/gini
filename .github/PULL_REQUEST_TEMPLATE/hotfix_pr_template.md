---
name: Hotfix Pull Request
about: Submit an urgent fix for a critical issue
title: '[HOTFIX] '
labels: 'hotfix'
assignees: ''

---

## Hotfix Description
Provide a clear and concise description of the urgent fix.

## Issue Being Fixed
- Fixes #(issue number)

## Severity
- [ ] Critical - System down or security vulnerability
- [ ] Major - Significant feature unusable
- [ ] Moderate - Feature partially unusable or degraded performance

## Risk Assessment
Describe the potential risks of this hotfix and why it's safe to apply directly to the main branch.

## Testing Performed
Describe in detail the testing that has been performed on this hotfix.
- [ ] Unit tests
- [ ] Integration tests
- [ ] Manual testing
- [ ] Production environment simulation

### Test Results
Provide a summary of test results.

## Verification Steps
Clear steps for the reviewer to verify this fix:
1. 
2. 
3. 

## Rollback Plan
Describe how to roll back this change if it causes unexpected issues.

## Follow-up Actions
List any additional actions required after merging this hotfix:
- [ ] Backport fix to development branch
- [ ] Create documentation update
- [ ] Monitor system metrics