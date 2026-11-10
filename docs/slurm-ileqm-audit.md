# ileqm native Slurm compatibility audit

> Archived investigation: the current implementation is CLI-only. The native
> prototype and its API-only dispatcher were removed; no native deployment is
> planned for this phase. The observations below describe the earlier audit.

## Observed environment

- Linux x86_64, glibc 2.28, GCC 8.4.1.
- CLI: `/opt/gridview/slurm/bin/squeue`.
- Release string: `slurm 20.11.8-1.6.0-20230414`.
- Library used by squeue/sacct:
  `/opt/gridview/slurm/lib/slurm/libslurmfull.so`.
- `slurm_api_version()` returns `0x240000`: API major 36.
- Readable site headers: `/opt/gridview/slurm/include/slurm/`.
- `libslurm.so` is also discoverable by the default loader and exports the
  active/history functions used by the current adapter. The full library is
  found by the CLI's RPATH, not by the default loader's bare-name search.
- Installed turm is static with musl libc symbols (`__init_libc`,
  `__libc_start_init`), and no ELF interpreter. In-process loading of the
  site's glibc Slurm requires a glibc-linked replacement turm.
- No `rustc` or `cargo` was found in the SSH session's PATH. A compiler is
  available for building the thin C adapter without administrator access.

## Correction: API major is not a complete vendor ABI identity

The site headers contain enabled `__SUGON_*` extensions. Comparing their
compiled layouts against the pinned official 20.11.9 headers gives:

| Type/field | Official headers | Site headers |
| --- | ---: | ---: |
| `sizeof(slurm_job_info_t)` | 872 | 872 |
| `sizeof(slurmdb_job_rec_t)` | 528 | 592 |
| `offsetof(slurmdb_job_rec_t, uid)` | 472 | 504 |
| `offsetof(slurmdb_job_rec_t, user)` | 488 | 520 |
| `offsetof(slurmdb_job_rec_t, tres_alloc_str)` | 456 | 488 |
| `offsetof(slurmdb_job_rec_t, work_dir)` | 520 | 552 |

The same API major therefore does **not** establish compatibility with the
official accounting adapter. Misreading `user` can dereference an invalid
pointer, not merely display the wrong value. The removed prototype's API-only dispatcher
did not recognize this vendor variant; do not enable its official API-36
adapter on this site as a complete native backend.

Site header SHA256:

```text
slurm.h   195129ef943410c4fb36c4524f6e260e120fc1e86798fc637a0f6c22ddf0df0c
slurmdb.h a964f34f63d0f962f053068f42251e994fb0c4aaf556dac38be21c88b3e0c436
```

Prefer compiling the common thin adapter against these installed headers.
Adapter identity must distinguish vendor/header variants in addition to the
library API major. The vendor also supplies historical `command`, `stdout`,
and `stderr` fields under different names from modern upstream fields; mapping
these belongs inside the adapter.

## Real-library query checks

The prototype C adapter and dispatcher were compiled on ileqm with the
site headers and GCC, in an automatically removed temporary directory.
The adapter loaded the absolute `libslurmfull.so` path and used normal user
credentials, without administrator access.

- Active query (`--me --states=all`): successful, zero current jobs.
- History query (own user, two days, selected terminal states): successful,
  one completed job with a nonempty working directory.
- Observed total subprocess times were 0.010 s and 0.096 s respectively.
  These are single diagnostic samples, not performance benchmarks.

This validates library loading, authentication, and the sampled queries using
the site-compiled C layout. It does not validate concurrent API calls, nonempty
live-array iteration, or the proposed Rust in-process backend.

## Native deployment option evaluated (deferred)

Use a glibc-linked turm built against a baseline no newer than glibc 2.28,
with the common thin C adapter compiled against the site headers. Keep the
Rust-facing C ABI stable and pass typed data directly in-process, without
JSON or IPC. Preserve CLI fallback for unverified ABI variants. Audit Slurm
initialization and shared state before allowing concurrent calls.

The installed turm and Slurm files were not modified by this audit.
