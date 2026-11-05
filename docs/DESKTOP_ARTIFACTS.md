# Desktop installation artifacts

Run the Release workflow with dry_run=true on the reviewed branch or commit.
This builds manual installers for Linux amd64, Windows amd64, and both macOS
architectures, and uploads workflow artifacts named with the exact source SHA.
The dry run creates no GitHub release or tag. All platforms use the committed
Rust toolchain, Cargo lockfiles, and frontend dependency lockfile.

Download the artifact for your system from the completed workflow run.
Linux artifacts target the Ubuntu 24.04 build baseline and contain a Debian package
and AppImage. Windows artifacts contain
the generated MSI/NSIS installers. macOS artifacts contain the architecture's DMG.
Install the package and connect to your OpenCorde server.

Manual artifacts are unsigned. The dry-run build disables updater artifact
generation and platform code signing. Operating-system trust prompts can apply.
Repository update-signing credentials are currently absent and the updater public
key remains a placeholder. Automatic signed updates require separate key
provisioning and a reviewed public-key configuration before release publication.

The encrypted vault uses the operating system credential store. Keep the
credential-store entry together with the local application data when backing up
a device. Missing vault files or wrapping keys fail closed. Installing a newer
server alone does not install native recovery functionality on users' devices.

Build success establishes packaging and compilation on the selected runner.
Linux native restart tests exercise a real Secret Service keyring. Windows and
macOS credential-store runtime recovery requires tests on those operating systems
before claiming equivalent runtime validation there.
