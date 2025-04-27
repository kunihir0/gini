# Contributing to Atsa Engine

Thanks for considering contributing to Atsa Engine! This document outlines the process for contributing to our project.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Workflow](#development-workflow)
3. [Issue Reporting](#issue-reporting)
4. [Pull Requests](#pull-requests)
5. [Coding Standards](#coding-standards)
6. [Testing](#testing)
7. [Documentation](#documentation)
8. [Community Guidelines](#community-guidelines)

## Getting Started

### Prerequisites

- [Bun](https://bun.sh/) (latest version)
- [Git](https://git-scm.com/)
- A GitHub account

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/kunihir0/osx
   cd osx
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/kunihir0/osx.git
   ```
4. Install dependencies:
   ```bash
   cargo build
   ```
5. Run the development server:
   ```bash
   cargo run
   ```

## Development Workflow

We follow a structured branching strategy documented in detail at [.github/BRANCHING_STRATEGY.md](.github/BRANCHING_STRATEGY.md).

In summary:

1. Create a branch from `dev`:
   ```bash
   git checkout dev
   git pull upstream dev
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and commit them:
   ```bash
   git add .
   git commit -m "Feat: Add your feature"
   ```

3. Push to your fork:
   ```bash
   git push -u origin feature/your-feature-name
   ```

4. Create a pull request from your branch to the upstream `dev` branch

## Issue Reporting

Before submitting an issue, please:

1. Check if the issue already exists
2. Use the appropriate issue template:
   - [Bug Report](/.github/ISSUE_TEMPLATE/bug_report.md)
   - [Feature Request](/.github/ISSUE_TEMPLATE/feature_request.md)
   - [Documentation Update](/.github/ISSUE_TEMPLATE/documentation_update.md)
   - [Security Vulnerability](/.github/ISSUE_TEMPLATE/security_vulnerability.md)

3. Provide as much detail as possible to help us understand and reproduce the issue

## Pull Requests

When submitting a pull request:

1. Use the provided [PR template](/.github/PULL_REQUEST_TEMPLATE/standard_pr_template.md)
2. Link the related issue(s)
3. Ensure all tests pass
4. Update relevant documentation
5. Follow our [committing standards](#committing-standards)

For urgent fixes, use the [Hotfix PR template](/.github/PULL_REQUEST_TEMPLATE/hotfix_pr_template.md).

See our complete [Code Review Guidelines](.github/CODE_REVIEW_GUIDELINES.md) for details on the review process.

## Committing Standards

We strive for consistent and maintainable code:

1. Include comments for complex logic
2. Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification

## Testing

New features and bug fixes should include appropriate tests:

1. Test both success and failure cases
2. Mock external dependencies
3. Aim for high coverage of critical paths
4. Run tests locally before submitting PR:
   ```bash
   cargo test
   ```

## Documentation

Good documentation is critical:

1. Update relevant documentation when changing behavior
2. Document new features thoroughly
3. Use the doc gen plugin comments for functions and classes
4. Keep README and other markdown files up to date
5. Create examples for complex features

## Community Guidelines

We aim to maintain a welcoming community for all contributors:

1. Be respectful and inclusive
2. Assume good intentions
3. Focus on the issue, not the person
4. Help others learn and grow
5. Give constructive feedback
6. Credit others for their contributions

## Additional Resources

- [GitHub Organization Guide](.github/GITHUB_ORGANIZATION_GUIDE.md) - Comprehensive guide to our GitHub organization
- [Code Review Guidelines](.github/CODE_REVIEW_GUIDELINES.md) - Detailed standards for code reviews
- [Branching Strategy](.github/BRANCHING_STRATEGY.md) - Our Git workflow

Thank you for contributing to Atsa Engine!