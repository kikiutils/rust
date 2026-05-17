#!/usr/bin/env pwsh
$ErrorActionPreference = "Stop"

$sep = [string][char]0x1f
$flags = @(
    "-C"
    "control-flow-guard=yes"

    # Optional static CRT for single-file deployment. Keep disabled by default
    # because some dependency stacks expect the dynamic MSVC runtime.
    # "-C"
    # "target-feature=+crt-static"
)

$old = [Environment]::GetEnvironmentVariable("CARGO_ENCODED_RUSTFLAGS", "Process")
$code = 0

try {
    [Environment]::SetEnvironmentVariable("CARGO_ENCODED_RUSTFLAGS", [string]::Join($sep, $flags), "Process")
    & cargo b -r --target x86_64-pc-windows-msvc @args
    $code = $LASTEXITCODE
} finally {
    [Environment]::SetEnvironmentVariable("CARGO_ENCODED_RUSTFLAGS", $old, "Process")
}

exit $code
