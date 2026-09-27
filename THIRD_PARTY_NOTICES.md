# Third-party components

The application's Rust source is distributed under the repository MIT license.
Cargo dependency licensing and advisories are checked by `cargo deny` using
`Cargo.lock` and `deny.toml`.

The self-contained Windows package also redistributes Microsoft runtime files
from these version-pinned NuGet packages, outside Cargo's dependency graph:

- [Microsoft.WindowsAppSDK.Runtime 2.4.0](https://www.nuget.org/packages/Microsoft.WindowsAppSDK.Runtime/2.4.0)
- [Microsoft.Web.WebView2 1.0.4078.44](https://www.nuget.org/packages/Microsoft.Web.WebView2/1.0.4078.44)

Their applicable Microsoft license terms are supplied by the original packages;
the distribution includes their package license/notices files when present.
`runtime-lock.json` records package versions and archive SHA-256 hashes.
`SHA256SUMS` records the exact shipped payload. The application MIT license
does not relicense these Microsoft binaries.
