# GitHub Project access and sync

Project: https://github.com/users/Hoshiko86/projects/2

The managed cloud environment supplies GitHub integration authentication at its
proxy. A diagnostic request with a deliberately invalid caller credential still
authenticated as the connected account. The integration cannot access user
Projects, even though repository issues and milestones work. Passing a PAT as
`GH_TOKEN` in this environment did not select that PAT. Additional publishing,
token scopes, or CLI reinstallations did not fix the routing.

The platform-side fix requires support for selecting a user credential for
GitHub API requests, or user Projects permissions in the integration. Do not
attempt to bypass the managed proxy or disable TLS verification.

## Supported alternative: run on GitHub Actions

The manual `Sync GitHub Project` workflow uses a GitHub-hosted runner. Its Project
requests use a dedicated PAT stored in GitHub, independently of the cloud proxy.

1. In this repository on GitHub, open Settings → Secrets and variables → Actions.
2. Create a **repository secret** named `HOSHI_PROJECTS_TOKEN`. Paste the existing
   classic PAT with the `project` scope into GitHub's secure secret input.
   Do not put it in chat, source files, workflow inputs or ordinary variables.
   The `codespace` scope is unnecessary. Repository issue reads use the separate
   built-in Actions token; a private repository may need additional configuration.
3. Merge the workflow PR into the default branch. Review it before merging: it
   runs only when manually dispatched, has no pull-request secret trigger, and
   does not install third-party Python packages.
4. Open Actions → Sync GitHub Project → Run workflow. Keep **dry_run** enabled
   first. Review the intended additions in the job log.
5. Run again with **dry_run** disabled to add repository issues and mark closed
   issues Done. The Project needs a Status field with a `Done` option.

Repeated runs preserve existing items and open-issue statuses; they add missing
issues, mark closed issues Done, and never remove unrelated items. Pull requests
are excluded. If a closed issue is reopened, its status must be moved manually.
No custom fields or views are created by this workflow. It uses a personal-user
Project; organization Projects require a different owner query.

This workaround is not validated end-to-end until the secret is configured,
the workflow is on the default branch, and an actual dispatch succeeds. An
expired/revoked PAT or disabled GitHub Actions can still prevent it from running.
