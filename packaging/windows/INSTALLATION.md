# Windows installation

`packaging/windows/package.ps1` builds two MSIs from `localsheets.wxs`:

- `localsheets-<version>-windows-<arch>.msi` installs for all users, into
  `Program Files\Local Sheets` (`Program Files (x86)` for the 32-bit build), and needs administrator
  rights. It is `Scope="perMachine"`.
- `localsheets-<version>-windows-user-<arch>.msi` installs for the current account only, into
  `%LOCALAPPDATA%\Programs\Local Sheets`, with no UAC prompt. It is `Scope="perUserOrMachine"`:
  Windows redirects Program Files, the Start Menu and HKMU to the user's own locations, so the same
  components work in both packages. Both share the UpgradeCode, and Windows Installer only pairs
  products installed in the same context.

Choose one: installing both gives two Local Sheets entries in Settings > Apps and two in the Start
menu. Silent installs pass `ALLUSERS=1` for the per-machine package
(`msiexec /i localsheets-<version>-windows-<arch>.msi /qn ALLUSERS=1`); the per-user package needs
no property.

The package registers, in the context it installs into (`HKMU` follows the package's scope):

- `Software\Anon5T4R\LocalSheets\InstallDir`, which the app's updater reads to tell an MSI install
  (it updates with the next MSI) from a portable copy (it updates with the portable zip);
- App Paths for `localsheets.exe`, so Win+R `localsheets` works;
- Local Sheets as an "Open with" choice for `.xlsx`, `.xlsm`, `.xlsb`, `.ods`, `.csv` and `.tsv`
  (ProgIds `LocalSheets.Workbook` and `LocalSheets.TextData`, also listed in `Capabilities` +
  `RegisteredApplications` for Settings > Default apps). Local Sheets has no format of its own, so
  the installer never takes an extension over from Excel or LibreOffice;
- a Start Menu shortcut, a `Sheets` alias for Start Menu search, and a desktop shortcut. All three
  are plain links to `localsheets.exe` (not advertised MSI shortcuts), each in its own component
  with a registry key path so Windows Installer removes it on uninstall.

`package.ps1` runs `wix msi validate -ice ICE50` before signing, for both MSIs. The slower check to
run by hand on a packaging change is the same command against the built files:

```powershell
wix msi validate dist/release/localsheets-<version>-windows-x64.msi -ice ICE50
wix msi validate dist/release/localsheets-<version>-windows-user-x64.msi -ice ICE50
```

Passing these checks does not prove a real installation, elevation, upgrades, registry writes,
shortcut behaviour, cancellation, rollback or uninstall: those still need the real payload in a
clean Windows test environment.
