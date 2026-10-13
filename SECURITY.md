# Security

Report vulnerabilities privately to 473276@qq.com. Do not include subscription secrets in public issues.

No shell/remote-code execution or broad filesystem plugins are exposed to the WebView. Only a bundled local window receives IPC capabilities. Remote strings are escaped; a restrictive CSP disallows external scripts. Catalog sync is bounded and restricted to GitHub raw HTTPS without redirects.

This is an unsigned development preview, not an audited security product. The same-user local filesystem and chosen GitHub repository are trust boundaries. SHA-256 checksums are not publisher signatures. Do not edit Clash configurations concurrently. Retain backups before upgrades.
