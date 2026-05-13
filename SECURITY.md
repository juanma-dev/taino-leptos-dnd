# Security Policy

## Reporting a vulnerability

**Please do not open a public issue for security reports.**

Use GitHub's private vulnerability reporting instead:

<https://github.com/juanma-dev/taino-leptos-dnd/security/advisories/new>

That endpoint creates a private advisory only the maintainers can see. Include:

- The affected crate (`taino-dnd-core` / `taino-dnd-leptos`) and version.
- A description of the issue and the impact you observed.
- Steps to reproduce, ideally as a minimal `cargo new` project.
- Any proof-of-concept code or rendered output, if relevant.

We aim to acknowledge reports within 72 hours and to land a fix within 14
days for high-severity issues. Coordinated disclosure timelines are
negotiable for complex cases.

## Supported versions

This project is pre-1.0. Only the latest released version receives security
fixes. Once `v1.0.0` ships, this section will be updated with a proper
support matrix.

| Version | Supported |
| ------- | --------- |
| latest 0.x | Yes |
| any earlier 0.x | No |

## Out of scope

- Issues that only affect non-released `main` commits (report as a regular
  issue or PR).
- Denial-of-service via maliciously crafted CSS or layouts in user code —
  this library does not validate or sandbox user-provided styles.
- Vulnerabilities in upstream dependencies (Leptos, web-sys, etc.). Please
  report those to the respective projects. We track advisories via
  `cargo audit` in CI and bump pinned versions when fixes are available.

## Hall of fame

Reporters who request credit will be listed here after their report is
resolved. Anonymous reports are equally welcome.
