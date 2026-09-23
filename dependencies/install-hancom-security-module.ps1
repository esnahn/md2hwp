# Retired: security-module installation and registration are user-managed.
# Kept as an explicit diagnostic for old documentation/bookmarks. No mutation.
throw @'
Automatic Hancom security-module installation has been retired.
Download the official Automation module and follow the included registration instructions:
https://developer.hancom.com/hwpautomation
Register FilePathCheckerModuleExample as REG_SZ with the DLL's absolute path under
HKCU\Software\HNC\HwpAutomation\Modules, using the account that runs Hancom.
Then run tools/investigation/test-hwp-open.ps1 in the documented interactive context.
'@
