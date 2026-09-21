# Flamingo Agent

A Windows service in Rust that, every 5 seconds, collects the UTC time and its
own RSS, launches a C++ child with them as one string argument, and keeps the
child's log readable by Administrators and SYSTEM only.


## Contents

- `agent-rust/` The service.
- `logger-child/`  The child: appends its argument to stdout and the log, exits
- `install.ps1`, `install.bat`, `uninstall.bat`  Build both, install, register, start.

## Install

Needs cargo and Visual Studio Build Tools with the "Desktop development with
C++" workload. Double-click, or run from any prompt:

```
install.bat              # build, install to %ProgramFiles%\FlamingoAgent, start
uninstall.bat            # install.bat -Uninstall
```

The `.bat` asks for elevation if needed and runs `install.ps1` with the
execution policy bypassed, so it works on a machine where scripts are
disabled. Logs go to `%ProgramData%\FlamingoAgent`.

## Check

```powershell
Get-Service FlamingoAgent
Get-Content "$env:ProgramData\FlamingoAgent\agent.log" -Wait -Tail 5   # one metrics line per tick
Get-Content "$env:ProgramData\FlamingoAgent\child.log" -Wait -Tail 5   # elevated prompt only
(Get-Acl "$env:ProgramData\FlamingoAgent\child.log").Access | Format-Table IdentityReference, FileSystemRights
```

The ACL lists exactly `BUILTIN\Administrators` and `NT AUTHORITY\SYSTEM`. The
child inherits the service's LocalSystem token, which is what lets it append to
that file. A child that can't exits 1 and the service logs a warning.

## Run in the foreground

```powershell
& "$env:ProgramFiles\FlamingoAgent\flamingo-agent.exe" --data-dir $env:TEMP\flamingo
```

Ctrl+C stops it. Unelevated, the child can't write the log and the agent logs
that. Options: `--data-dir <dir>` and `--child <exe>` (default: next to the
agent). `--install` accepts the same options and bakes them into the
registration.

## How it works

Each tick the agent creates `child.log` if missing, applies the DACL
`D:P(A;;FA;;;BA)(A;;FA;;;SY)`, then runs `logger-child <metrics> <log-file>`,
which opens the file for append. The agent owns the file so it never exists
with inherited permissions. Failures are logged and the loop carries on; the
SCM registration restarts the service if it exits.
