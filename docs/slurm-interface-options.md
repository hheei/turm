# Slurm interface and prebuilt-adapter options

> Research notes only. The chosen scope for this phase is CLI-only; native
> adapters, helpers, and REST integration are deferred.

## Native adapters can be prebuilt elsewhere

Using site headers requires a matching build input, not compilation on the
destination machine. The headers can be used in a compatible external build
environment to produce a site-specific adapter in advance. The resulting
architecture, glibc requirements, and Slurm ABI must match the deployment.
For ileqm that means x86_64, a glibc baseline no newer than 2.28, and its
Gridview/Sugon headers rather than the official API-36 layout.

This follows from the measured header/layout differences documented in
[the site audit](slurm-ileqm-audit.md). A site-specific prebuilt bundle does
not establish compatibility with every installation reporting API major 36.

## Official interfaces

SchedMD describes CLI, C APIs, and REST, and explicitly encourages developers
to target REST for broader cross-version compatibility:
https://slurm.schedmd.com/api.html

The Slurm 20.11 documentation already includes:

- Versioned REST/OpenAPI endpoints and controller/accounting plugins.
- Local UNIX socket authentication.
- Running slurmrestd as a normal user for that user's own requests, within
  the Munge security domain; root/SlurmUser is needed for serving multiple
  identities, not inherently for a single-user service.
- Synchronous controller/accounting requests and no built-in response cache.

Source:
https://slurm.schedmd.com/archive/slurm-20.11.9/rest.html

REST avoids exposing C layouts to the client but requires a compatible server
deployment and structured wire encoding. It does not inherently remove RPC
latency or guarantee unchanged schemas across endpoint versions.

SchedMD announced JSON/YAML output for sacct, squeue, and sinfo in Slurm 21.08:
https://lists.schedmd.com/pipermail/slurm-announce/2021/000059.html

Structured CLI output removes reliance on formatted text layouts but retains
CLI startup and RPC work.

## ileqm availability checks

Read-only SSH checks found:

- No `slurmrestd` in PATH or under the checked installation's `bin`/`sbin`.
- No REST/OpenAPI/JSON/JWT-named plugin files under the checked Slurm library
  directory.
- No Slurm-named local listening UNIX socket in `ss -xl`.
- No `--json` or `--yaml` advertised by the installed squeue/sacct help.

These checks do not establish whether the site operates a REST endpoint on a
different machine. A site endpoint URL/authentication method would need to be
provided or confirmed by the administrators.

## Decision

- Prefer an existing site REST endpoint when it is available and suitable.
- For no additional service and no IPC encoding, a prebuilt site-specific
  in-process adapter is viable, with native memory-safety and ABI constraints.
- For a generally distributed binary on older/vendor installations, the
  installed CLI remains the compatibility fallback.

No production backend changes were made as part of this interface research.
