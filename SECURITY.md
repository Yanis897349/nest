# Security policy

## Supported versions

Security fixes are applied to the latest release and the default branch.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private vulnerability
reporting feature from the repository's **Security** tab. Include the affected version,
reproduction steps, impact, and any suggested mitigation. If private reporting is unavailable,
ask the maintainer for a private contact channel without posting sensitive details.

An acknowledgement should arrive within seven days. Please allow time to validate and prepare a
fix before public disclosure.

## Security and privacy boundaries

- `nest` drives a temporary Chromium profile and does not store logins or credentials.
- Navigation and downloaded price sprites are restricted to HTTPS Ziroom domains. Network
  requests have time and size limits, and untrusted page data is treated as data rather than code.
- Optional diagnostics exclude commute addresses, listing content, and URL queries. Review a log
  before sharing it because operating-system and browser errors can still contain local paths.
- The application never solves CAPTCHAs, rotates proxies, logs in, or bypasses access controls.
- Exported rental data is stored wherever the user chooses and is not encrypted by `nest`.

The project is independent of Ziroom. Users are responsible for complying with the site's terms
and applicable law.
