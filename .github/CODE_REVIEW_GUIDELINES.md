# Code Review Guidelines and Contribution Standards

This document outlines the standards and best practices for code contributions and reviews in our project.

## Code Contribution Guidelines

### Code Style and Quality

- Follow the established code style for the project
- Write self-documenting code with clear variable and function names
- Add comments for complex logic, but avoid obvious comments
- Keep functions small and focused on a single responsibility
- Limit line length to 100 characters where possible

### Commit Guidelines

- Use descriptive commit messages that explain WHY a change was made
- Start commit messages with a verb in imperative form (e.g., "Add", "Fix", "Update")
- Reference issue numbers in commit messages when applicable
- Keep commits focused on a single logical change
- Structure commit messages as:
  ```
  [Type]: Short summary (max 50 chars)
  
  More detailed explanation if needed. Wrap at around 72 characters.
  Explain the problem that this commit is solving. Focus on why you
  are making this change as opposed to how.
  
  Refs #123
  ```

  Types:
  - `Feat`: New feature
  - `Fix`: Bug fix
  - `Docs`: Documentation changes
  - `Style`: Code style changes (formatting, missing semicolons, etc)
  - `Refactor`: Code refactoring without changing functionality
  - `Test`: Adding or improving tests
  - `Chore`: Changes to the build process, tooling, etc

### Pull Request Process

1. **Before submitting a PR**:
   - Ensure code passes all existing tests
   - Add new tests for new functionality
   - Run linters to ensure code style compliance
   - Rebase on the latest version of the target branch

2. **When submitting a PR**:
   - Use the provided PR template
   - Link to relevant issues
   - Provide a clear description of the changes
   - Include screenshots or recordings for UI changes
   - Tag relevant reviewers

3. **After submitting a PR**:
   - Be responsive to feedback
   - Make requested changes promptly
   - Discuss any disagreements respectfully
   - Squash fix-up commits before final merge

## Code Review Guidelines

### For Reviewers

1. **Be timely**:
   - Aim to review PRs within one business day
   - If you can't review thoroughly, do a cursory review and note that a more detailed review will follow

2. **Be thorough**:
   - Review the code, not the author
   - Check for both functionality and style
   - Look for edge cases and potential bugs
   - Consider performance implications
   - Verify test coverage

3. **Be respectful**:
   - Use a constructive tone
   - Explain the reasoning behind your suggestions
   - Distinguish between required changes and preferences
   - Offer solutions, not just criticism
   - Acknowledge good work

4. **Be specific**:
   - Point to specific lines of code
   - Provide examples when suggesting changes
   - Link to documentation or resources when appropriate

### For Authors

1. **Be responsive**:
   - Respond to all comments
   - Thank reviewers for their time and feedback
   - Ask for clarification when needed

2. **Be open to feedback**:
   - Don't take criticism personally
   - Be willing to make changes
   - Explain your reasoning when you disagree

3. **Be thorough**:
   - Address all comments before requesting re-review
   - Test your changes after making revisions
   - Update the PR description if changes alter the original scope

## Review Checklist

- [ ] **Functionality**: Does the code work as intended?
- [ ] **Tests**: Are there appropriate tests that cover the changes?
- [ ] **Error handling**: Does the code handle errors gracefully?
- [ ] **Edge cases**: Are edge cases considered and handled?
- [ ] **Performance**: Are there any performance concerns?
- [ ] **Security**: Are there any security implications?
- [ ] **Accessibility**: Do UI changes follow accessibility best practices?
- [ ] **Documentation**: Is the code well-documented?
- [ ] **Dependencies**: Are new dependencies necessary and appropriate?
- [ ] **Backwards compatibility**: Do changes break existing functionality?

## Final Approval

A PR requires at least one approval from a designated reviewer before merging. For critical features or changes to core functionality, two approvals may be required.

All automated checks must pass before merging, including:

- CI pipeline success
- Linting checks
- Test coverage thresholds
- Security scans

## Post-Merge

After merging:
- Verify the feature works in the development environment
- Monitor for any unexpected behavior
- Close related issues
- Delete the feature branch
- Communicate changes to the team if necessary