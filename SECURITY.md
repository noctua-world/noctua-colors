# Security

## Reporting

Report a vulnerability through GitHub's private advisory form:
<https://github.com/noctua-world/noctua-colors/security/advisories/new>.

Please do not open a public issue for anything exploitable. There is no bug
bounty; there is a reply.

## What the threat model actually is

This project ships **data** — colour values, as CSS, JSON, SCSS and Rust
constants. It has no runtime, no server, no network access, and no dependencies
in anything it publishes. So the realistic risk is not a bug in the colours; it
is **a malicious artifact wearing this project's name.**

Everything below is aimed at that.

## What is done about it

**No dependencies, and no install scripts.** The npm package declares no
`dependencies`, no `peerDependencies` and no `scripts` — `cargo xtask check`
fails if any of them appear. npm 12 blocks lifecycle scripts by default, so a
`postinstall` here would be broken *and* suspicious.

**No long-lived publishing credentials.** Publishing runs from GitHub Actions
using OIDC trusted publishing — npm and crates.io both exchange the workflow's
identity for a credential that lives minutes. There is no token in a secret store
to steal. crates.io's is revoked automatically when the job ends.

**Provenance on everything.**

| Artifact | Attestation |
|---|---|
| npm tarball | npm provenance, Sigstore-signed, tying the tarball to the commit and workflow |
| GitHub Release assets | SLSA build provenance via `actions/attest` |
| Every generated file | BLAKE3 hash in `system/MANIFEST.json`, alongside the spec's own hash |

**Reproducible output.** The compiler is deterministic: the same spec and version
produce byte-identical artifacts, and CI proves it by building twice and diffing.
So a tampered artifact does not merely fail a signature — it fails to reproduce.

**A minimal tarball.** The npm package ships an explicit allowlist, not the
repository. Small tarballs are auditable tarballs.

## Verifying what you got

```sh
# npm: registry signatures and the provenance attestation
npm audit signatures

# A release asset
gh attestation verify noctua-colors-v0.2.0-system.tar.gz \
  --repo noctua-world/noctua-colors
sha256sum -c SHA256SUMS

# Any generated file, against the manifest it shipped with
b3sum system/css/index.css   # compare with system/MANIFEST.json
```

## Supported versions

Pre-1.0, only the latest release. A security fix will be a new patch version;
`cargo yank` and npm deprecation are used to steer people off a bad one, though
neither removes it — nothing published to either registry can be unpublished, and
you should assume a bad version stays reachable.

## What this project will not do

- **No `postinstall`, ever.** If a future release grows one, treat it as
  compromised.
- **No network access at build or install time.** The artifacts are committed;
  nothing is fetched.
- **No telemetry.**
