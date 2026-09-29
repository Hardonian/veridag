# GitHub Repository Settings Runbook

These settings cannot be expressed in code — they must be applied manually in
the GitHub repository settings UI or via the GitHub API after the repo is
live. Apply all of them before accepting external contributions.

---

## 1. Branch Protection: `main`

**Settings → Branches → Add branch protection rule → `main`**

| Setting | Value |
|---------|-------|
| Require a pull request before merging | ✅ |
| Required approving reviews | **1** minimum (set to 2 for protocol changes) |
| Dismiss stale pull request approvals when new commits are pushed | ✅ |
| Require review from Code Owners | ✅ (CODEOWNERS is configured) |
| Require status checks to pass before merging | ✅ |
| Required status checks | `rust`, `cross-edge`, `conformance`, `security` |
| Require branches to be up to date before merging | ✅ |
| Require conversation resolution before merging | ✅ |
| Do not allow bypassing the above settings | ✅ |
| Restrict who can push to matching branches | ✅ → `@Hardonian` (org admins only) |

---

## 2. Tag Protection: `v*`

**Settings → Tags → Add rule → `v*`**

Prevent tag deletion or re-tagging by non-admins. Release tags are permanent.

---

## 3. Private Vulnerability Reporting

**Settings → Security → Private vulnerability reporting → Enable**

This activates the Security Advisory intake linked from `SECURITY.md`.
Maintainers get notified via email when a report is filed.

---

## 4. Repository Secrets

Required for the `release-packages.yml` workflow to publish artifacts.

| Secret name | What it is |
|-------------|-----------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token with `publish-update` scope for the `veridag` org |
| `NPM_TOKEN` | npm automation token with publish access to `@veridag` org |
| `PYPI_TOKEN` | PyPI API token for the `veridag` project (use environment token, not account token) |

Set via: **Settings → Secrets and variables → Actions → New repository secret**

Without these secrets, `release-packages.yml` runs in dry-run mode (safe — it
will build and validate but not publish).

---

## 5. Environments

Create two environments for progressive deployment gating:

**Settings → Environments → New environment**

| Environment | Protection rules |
|-------------|-----------------|
| `staging` | No reviewers required; deploys automatically on tag push |
| `production` | Required reviewer: `@Hardonian`; 10 min wait before deployment |

---

## 6. Dependabot Alerts

**Settings → Security → Dependabot alerts → Enable**  
**Settings → Security → Dependabot security updates → Enable**

Dependabot PRs from `dependabot.yml` will auto-open for Cargo, npm, and pip.

---

## 7. Code Scanning (Optional but Recommended)

**Security → Code scanning → Set up → GitHub Actions**

Adds SARIF-format static analysis results visible inline on PRs. The existing
`security.yml` with `cargo deny` + `cargo audit` covers Rust supply chain;
CodeQL covers general code quality.
