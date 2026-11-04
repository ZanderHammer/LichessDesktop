---
status: accepted
date: 2026-11-04
---

# Watch lila weekly with a failing check and an import pull request

## Context and Problem Statement

The app depends on two things in [lila](https://github.com/lichess-org/lila) that can change without notice: its translations (ADR 0020) and the format of the page title in `ui/round/src/title.ts`, which turn detection (#21) will read. We want to notice both changes (#22) without having to remember to look.

* `title.ts` sets ``document.title = `${text} - ${initialTitle}` ``, checked at git blob [`929a8fc788`](https://github.com/lichess-org/lila/blob/fbc48d85d63cde254598f4af1d829f016920e19f/ui/round/src/title.ts). Most of the file is about the favicon, so an edit doesn't have to mean a new format. Only a person can tell.
* [Scheduled workflows](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule) only run on the default branch, notify the user who last modified the cron syntax, and are disabled in public repositories after 60 days without repository activity.
* [Pull requests opened with `GITHUB_TOKEN` don't start other workflows](https://github.com/peter-evans/create-pull-request/blob/v8.1.1/docs/concepts-guidelines.md#triggering-further-workflow-runs). Closing and reopening one by hand does. The `main` ruleset requires the CI checks, so an unchecked pull request can't be merged.

## Considered Options

* A scheduled workflow with a failing job for the title and `GITHUB_TOKEN` for the import pull request
* The same workflow, opening an issue when the title changes
* The same workflow, with a personal access token or GitHub App so that CI starts by itself

## Decision Outcome

Chosen option: "A scheduled workflow with a failing job for the title and `GITHUB_TOKEN`", because it needs no stored secret and no code to manage issues.

* **Title.** `.github/workflows/lila-watcher.yml` compares the git blob of `title.ts` with `LILA_TITLE_BLOB` in the same file. A different blob or a missing file fails the job, and it keeps failing every week until someone checks the format and updates the value. Comparing the blob needs no copy of lila's code in this repository.
* **Translations.** A second job runs the import from ADR 0020 and opens or updates a pull request from the `lila-import` branch, using [`peter-evans/create-pull-request`](https://github.com/peter-evans/create-pull-request). The commit names the lila commit. The jobs are separate, so a changed title doesn't hold up translations.
* **Repository setting.** The import job needs "Allow GitHub Actions to create and approve pull requests" to be on.
* **Turn-detection table.** It doesn't exist yet. #21 adds its generator to the import job and its path to `add-paths`.

### Consequences

* Good, because a change in lila reaches us within a week without any manual step.
* Good, because the only credential is the short-lived `GITHUB_TOKEN`.
* Bad, because someone has to close and reopen every import pull request before CI runs on it.
* Bad, because a favicon edit in `title.ts` also fails the job.
* Bad, because the repository setting also lets workflows approve pull requests. No workflow here does, and the `main` ruleset requires no approvals.
* Bad, because the schedule stops after 60 days without repository activity, until someone enables it again.

<!-- Based on the MADR minimal template: https://adr.github.io/madr/ -->
