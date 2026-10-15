[CmdletBinding()]
param(
    [string]$TestExecutable,

    [string]$OutputDirectory,

    [switch]$SelfTest,

    [ValidateRange(1, 16)]
    [int]$MaxModules = 8,

    [ValidateRange(1, 32)]
    [int]$MaxDependenciesPerModule = 16,

    [ValidateRange(1, 64)]
    [int]$MaxSymbolsPerModule = 32,

    [ValidateRange(1, 16)]
    [int]$MaxDumpbinCandidates = 8,

    [ValidateRange(100, 4000)]
    [int]$MaxDumpbinOutputLines = 2000,

    [ValidateRange(5, 120)]
    [int]$MaxAuditSeconds = 60
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$auditStopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$script:DumpbinResolverEvidence = [ordered]@{
    status                    = 'not_started'
    vswhere_path              = $null
    vswhere_exit_code         = $null
    vswhere_timed_out         = $false
    vswhere_output_truncated  = $false
    installation_path          = $null
    default_version_file       = $null
    default_tool_version       = $null
    candidate_count_observed   = 0
    candidate_limit            = $MaxDumpbinCandidates
    candidate_count_truncated  = $false
    candidates                 = @()
    selection_reason           = $null
    selected_path              = $null
}

function Assert-AuditBudget {
    if ($auditStopwatch.Elapsed.TotalSeconds -gt $MaxAuditSeconds) {
        throw "PE audit exceeded its $MaxAuditSeconds-second budget; root cause remains unknown."
    }
}

function Get-RemainingAuditSeconds {
    Assert-AuditBudget
    return [Math]::Max(1, [Math]::Floor($MaxAuditSeconds - $auditStopwatch.Elapsed.TotalSeconds))
}

function Initialize-BoundedNativeRunner {
    if ('AntigravityBoundedNativeRunner' -as [type]) {
        return
    }

    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Diagnostics;

public sealed class AntigravityBoundedNativeResult
{
    public int? ExitCode { get; set; }
    public bool TimedOut { get; set; }
    public bool OutputTruncated { get; set; }
    public string[] Lines { get; set; }
}

public static class AntigravityBoundedNativeRunner
{
    public static AntigravityBoundedNativeResult Run(string filePath, string[] arguments, int timeoutSeconds, int maxOutputLines)
    {
        var startInfo = new ProcessStartInfo { FileName = filePath, UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        foreach (var argument in arguments) startInfo.ArgumentList.Add(argument);
        using (var process = new Process { StartInfo = startInfo })
        {
            var gate = new object();
            var lines = new List<string>();
            var truncated = false;
            DataReceivedEventHandler capture = (sender, eventArgs) =>
            {
                if (eventArgs.Data == null) return;
                lock (gate)
                {
                    if (lines.Count < maxOutputLines) lines.Add(eventArgs.Data);
                    else truncated = true;
                }
            };
            process.OutputDataReceived += capture;
            process.ErrorDataReceived += capture;
            if (!process.Start()) throw new InvalidOperationException("Unable to start owned diagnostic process.");
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();
            var timedOut = !process.WaitForExit(Math.Max(1, timeoutSeconds) * 1000);
            if (timedOut) process.Kill(entireProcessTree: true);
            process.WaitForExit();
            lock (gate)
            {
                return new AntigravityBoundedNativeResult { ExitCode = timedOut ? (int?)null : process.ExitCode, TimedOut = timedOut, OutputTruncated = truncated, Lines = lines.ToArray() };
            }
        }
    }
}
'@
}

function Invoke-BoundedNative {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][int]$TimeoutSeconds,
        [Parameter(Mandatory)][int]$MaxOutputLines
    )

    Initialize-BoundedNativeRunner
    return [AntigravityBoundedNativeRunner]::Run($FilePath, $Arguments, $TimeoutSeconds, $MaxOutputLines)
}

function Select-DumpbinCandidate {
    param(
        [Parameter(Mandatory)][AllowEmptyCollection()][object[]]$Candidates,
        [AllowNull()][string]$DefaultToolVersion
    )

    $valid = @($Candidates | Where-Object { $_.valid -and $_.version })
    if ($valid.Count -eq 0) {
        return [pscustomobject]@{ status = 'no_valid_hostx64_x64_candidate'; selected = $null }
    }

    if ($DefaultToolVersion) {
        try {
            $defaultVersion = [version]$DefaultToolVersion
        } catch {
            return [pscustomobject]@{ status = 'default_tool_version_invalid'; selected = $null }
        }

        $matchingDefault = @($valid | Where-Object { ([version]$_.version) -eq $defaultVersion })
        if ($matchingDefault.Count -eq 1) {
            return [pscustomobject]@{ status = 'selected_default_tool_version'; selected = $matchingDefault[0] }
        }

        return [pscustomobject]@{ status = 'default_tool_version_not_available'; selected = $null }
    }

    if ($valid.Count -eq 1) {
        return [pscustomobject]@{ status = 'selected_only_valid_candidate'; selected = $valid[0] }
    }

    $ordered = @($valid | Sort-Object { [version]$_.version } -Descending)
    $highestVersion = [version]$ordered[0].version
    $highest = @($ordered | Where-Object { ([version]$_.version) -eq $highestVersion })
    if ($highest.Count -eq 1) {
        return [pscustomobject]@{ status = 'selected_highest_valid_version_without_default'; selected = $highest[0] }
    }

    return [pscustomobject]@{ status = 'ambiguous_highest_valid_version'; selected = $null }
}

function Get-DumpbinPath {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $script:DumpbinResolverEvidence.vswhere_path = $vswhere
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        $script:DumpbinResolverEvidence.status = 'vswhere_unavailable'
        throw 'vswhere.exe is unavailable; PE import/export audit is unknown.'
    }

    $vswhereResult = Invoke-BoundedNative -FilePath $vswhere -Arguments @('-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath') -TimeoutSeconds (Get-RemainingAuditSeconds) -MaxOutputLines 16
    $script:DumpbinResolverEvidence.vswhere_exit_code = $vswhereResult.ExitCode
    $script:DumpbinResolverEvidence.vswhere_timed_out = $vswhereResult.TimedOut
    $script:DumpbinResolverEvidence.vswhere_output_truncated = $vswhereResult.OutputTruncated
    if ($vswhereResult.TimedOut -or $vswhereResult.OutputTruncated -or $vswhereResult.ExitCode -ne 0) {
        $script:DumpbinResolverEvidence.status = 'vswhere_unavailable_or_bounded_failure'
        throw 'vswhere did not produce bounded successful installation metadata; PE import/export audit is unknown.'
    }

    $installations = @($vswhereResult.Lines | ForEach-Object { $_.Trim() } | Where-Object { $_ } | Sort-Object -Unique)
    if ($installations.Count -ne 1 -or -not (Test-Path -LiteralPath $installations[0] -PathType Container)) {
        $script:DumpbinResolverEvidence.status = 'vswhere_installation_path_unavailable_or_ambiguous'
        throw 'vswhere did not resolve exactly one valid Visual Studio installation; PE import/export audit is unknown.'
    }

    $installationPath = $installations[0]
    $script:DumpbinResolverEvidence.installation_path = $installationPath
    $toolRoot = Join-Path $installationPath 'VC\Tools\MSVC'
    if (-not (Test-Path -LiteralPath $toolRoot -PathType Container)) {
        $script:DumpbinResolverEvidence.status = 'msvc_tool_root_unavailable'
        throw 'Visual Studio MSVC tool root is unavailable; PE import/export audit is unknown.'
    }

    $versionDirectories = @(
        Get-ChildItem -LiteralPath $toolRoot -Directory | ForEach-Object {
            try {
                [void][version]$_.Name
                $_
            } catch {
                $null
            }
        } | Where-Object { $_ } | Sort-Object { [version]$_.Name } -Descending
    )
    $script:DumpbinResolverEvidence.candidate_count_observed = $versionDirectories.Count
    if ($versionDirectories.Count -gt $MaxDumpbinCandidates) {
        $script:DumpbinResolverEvidence.candidate_count_truncated = $true
        $script:DumpbinResolverEvidence.candidates = @($versionDirectories | Select-Object -First $MaxDumpbinCandidates | ForEach-Object {
            $candidatePath = Join-Path $_.FullName 'bin\Hostx64\x64\dumpbin.exe'
            [ordered]@{ version = $_.Name; path = $candidatePath; valid = (Test-Path -LiteralPath $candidatePath -PathType Leaf) }
        })
        $script:DumpbinResolverEvidence.status = 'too_many_msvc_tool_versions'
        throw "MSVC tool-version candidate count exceeds the $MaxDumpbinCandidates-item bound; PE import/export audit is unknown."
    }

    $candidates = @($versionDirectories | ForEach-Object {
        $candidatePath = Join-Path $_.FullName 'bin\Hostx64\x64\dumpbin.exe'
        [pscustomobject]@{
            version = $_.Name
            path    = $candidatePath
            valid   = Test-Path -LiteralPath $candidatePath -PathType Leaf
        }
    })
    $script:DumpbinResolverEvidence.candidates = @($candidates | ForEach-Object { [ordered]@{ version = $_.version; path = $_.path; valid = $_.valid } })

    $defaultVersionFile = Join-Path $installationPath 'VC\Auxiliary\Build\Microsoft.VCToolsVersion.default.txt'
    $script:DumpbinResolverEvidence.default_version_file = $defaultVersionFile
    $defaultToolVersion = $null
    if (Test-Path -LiteralPath $defaultVersionFile -PathType Leaf) {
        $defaultToolVersion = @(Get-Content -LiteralPath $defaultVersionFile -TotalCount 2 | ForEach-Object { $_.Trim() } | Where-Object { $_ } | Select-Object -First 1)[0]
        $script:DumpbinResolverEvidence.default_tool_version = $defaultToolVersion
    }

    $selection = Select-DumpbinCandidate -Candidates $candidates -DefaultToolVersion $defaultToolVersion
    $script:DumpbinResolverEvidence.selection_reason = $selection.status
    if ($null -eq $selection.selected) {
        $script:DumpbinResolverEvidence.status = $selection.status
        throw "Unable to select a validated VS Hostx64/x64 dumpbin.exe ($($selection.status)); PE import/export audit is unknown."
    }

    $script:DumpbinResolverEvidence.status = 'selected'
    $script:DumpbinResolverEvidence.selected_path = $selection.selected.path
    return $selection.selected.path
}

function Invoke-Dumpbin {
    param([string[]]$Arguments)

    Assert-AuditBudget
    $remainingSeconds = Get-RemainingAuditSeconds
    $result = Invoke-BoundedNative -FilePath $script:Dumpbin -Arguments $Arguments -TimeoutSeconds $remainingSeconds -MaxOutputLines $MaxDumpbinOutputLines
    if ($result.TimedOut) {
        throw "dumpbin timed out within the remaining overall audit budget; root cause remains unknown."
    }
    if ($result.ExitCode -ne 0) {
        throw "dumpbin failed: $($Arguments -join ' ')"
    }
    if ($result.OutputTruncated) {
        throw "dumpbin output exceeded the $MaxDumpbinOutputLines-line bound; root cause remains unknown."
    }

    return $result.Lines
}

function ConvertFrom-DumpbinDependentLines {
    param([string[]]$Lines)

    $names = foreach ($line in $Lines) {
        if ($line -match '^\s+([A-Za-z0-9_.-]+\.dll)\s*$') {
            $matches[1].ToLowerInvariant()
        }
    }
    return @($names | Sort-Object -Unique)
}

function Get-DependentNames {
    param([string]$Path)
    return ConvertFrom-DumpbinDependentLines -Lines (Invoke-Dumpbin @('/dependents', $Path))
}

function ConvertFrom-DumpbinImportLines {
    param([string[]]$Lines)

    $imports = @{}
    $currentDll = $null
    foreach ($line in $Lines) {
        if ($line -match '^\s+([A-Za-z0-9_.-]+\.dll)\s*$') {
            $currentDll = $matches[1].ToLowerInvariant()
            if (-not $imports.ContainsKey($currentDll)) {
                $imports[$currentDll] = [System.Collections.Generic.List[string]]::new()
            }
            continue
        }

        # dumpbin prints the import hint in one to four hexadecimal characters.
        # Ordinal-only imports remain intentionally unknown rather than guessed.
        if ($null -ne $currentDll -and $line -match '^\s+[0-9A-Fa-f]{1,4}\s+([^\s]+)\s*$') {
            $symbol = $matches[1]
            if ($symbol -notin @('Import', 'Hint', 'Time', 'Forwarder') -and $imports[$currentDll].Count -lt $MaxSymbolsPerModule) {
                $imports[$currentDll].Add($symbol)
            }
        }
    }

    return $imports
}

function Get-ImportsByDll {
    param([string]$Path)
    return ConvertFrom-DumpbinImportLines -Lines (Invoke-Dumpbin @('/imports', $Path))
}

function ConvertFrom-DumpbinExportLines {
    param([string[]]$Lines)

    $exports = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($line in $Lines) {
        # A forwarded export keeps its exported name before the forwarding target,
        # so this treats forwarded symbols as available rather than false-missing.
        if ($line -match '^\s*\d+\s+(?:[0-9A-Fa-f]+\s+){1,2}([^\s=]+)(?:\s*=\s*.+)?$') {
            [void]$exports.Add($matches[1])
        }
    }

    return ,$exports
}

function Get-ExportedSymbols {
    param([string]$Path)
    return ConvertFrom-DumpbinExportLines -Lines (Invoke-Dumpbin @('/exports', $Path))
}

function Get-FileVersionSummary {
    param([string]$Path)

    $info = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
    return [ordered]@{
        path            = $Path
        file_version    = $info.FileVersion
        product_version = $info.ProductVersion
        sha256          = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    }
}

function Test-IsApiSet {
    param([string]$Name)
    return $Name -like 'api-ms-win-*' -or $Name -like 'ext-ms-win-*'
}

function Get-MissingExportFindings {
    param([object[]]$Modules)

    return @(
        $Modules | ForEach-Object {
            $modulePath = $_.module.path
            $_.dependencies | Where-Object { $_.missing_exports.Count -gt 0 } | ForEach-Object {
                [pscustomobject]@{ module = $modulePath; dll = $_.name; candidate = $_.selected_candidate; missing_exports = $_.missing_exports }
            }
        } | Select-Object -First 16
    )
}

function Assert-SelfTest {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) {
        throw "Self-test failed: $Message"
    }
}

function Invoke-SelfTest {
    $dependents = ConvertFrom-DumpbinDependentLines -Lines @('    KERNEL32.dll', '    local.dll', 'ignored')
    Assert-SelfTest ($dependents.Count -eq 2 -and $dependents[1] -eq 'local.dll') 'dependent parser'

    $imports = ConvertFrom-DumpbinImportLines -Lines @('    local.dll', '       0012 RequiredExport', '       0013 AnotherExport')
    Assert-SelfTest ($imports['local.dll'].Count -eq 2 -and $imports['local.dll'][0] -eq 'RequiredExport') 'import parser'

    $exports = ConvertFrom-DumpbinExportLines -Lines @('      1    0 00001000 RequiredExport', '      2    1 ForwardedExport = KERNEL32.ForwardedExport')
    Assert-SelfTest ($exports -is [System.Collections.Generic.HashSet[string]] -and $exports.Contains('RequiredExport') -and $exports.Contains('ForwardedExport')) 'export parser and HashSet preservation'

    $zeroSelection = Select-DumpbinCandidate -Candidates @() -DefaultToolVersion $null
    Assert-SelfTest ($zeroSelection.status -eq 'no_valid_hostx64_x64_candidate' -and $null -eq $zeroSelection.selected) 'zero dumpbin candidates'

    $oneSelection = Select-DumpbinCandidate -Candidates @([pscustomobject]@{ version = '14.40.33807'; path = 'C:\\vs\\VC\\Tools\\MSVC\\14.40.33807\\bin\\Hostx64\\x64\\dumpbin.exe'; valid = $true }) -DefaultToolVersion $null
    Assert-SelfTest ($oneSelection.status -eq 'selected_only_valid_candidate' -and $oneSelection.selected.version -eq '14.40.33807') 'one validated dumpbin candidate'

    $multipleCandidates = @(
        [pscustomobject]@{ version = '14.39.33519'; path = 'C:\\vs\\VC\\Tools\\MSVC\\14.39.33519\\bin\\Hostx64\\x64\\dumpbin.exe'; valid = $true },
        [pscustomobject]@{ version = '14.40.33807'; path = 'C:\\vs\\VC\\Tools\\MSVC\\14.40.33807\\bin\\Hostx64\\x64\\dumpbin.exe'; valid = $true }
    )
    $defaultSelection = Select-DumpbinCandidate -Candidates $multipleCandidates -DefaultToolVersion '14.39.33519'
    Assert-SelfTest ($defaultSelection.status -eq 'selected_default_tool_version' -and $defaultSelection.selected.version -eq '14.39.33519') 'current tool-version metadata selection'

    $versionSelection = Select-DumpbinCandidate -Candidates $multipleCandidates -DefaultToolVersion $null
    Assert-SelfTest ($versionSelection.status -eq 'selected_highest_valid_version_without_default' -and $versionSelection.selected.version -eq '14.40.33807') 'bounded version-aware fallback selection'

    $invalidSelection = Select-DumpbinCandidate -Candidates @(
        [pscustomobject]@{ version = '14.41.0'; path = 'C:\\invalid\\dumpbin.exe'; valid = $false },
        [pscustomobject]@{ version = '14.40.0'; path = 'C:\\vs\\VC\\Tools\\MSVC\\14.40.0\\bin\\Hostx64\\x64\\dumpbin.exe'; valid = $true }
    ) -DefaultToolVersion '14.41.0'
    Assert-SelfTest ($invalidSelection.status -eq 'default_tool_version_not_available' -and $null -eq $invalidSelection.selected) 'invalid path never selected or executed'

    $cargoMessages = @(
        [pscustomobject]@{ reason = 'compiler-artifact'; profile = [pscustomobject]@{ test = $true }; executable = 'target\\debug\\deps\\antigravity_tools_lib-test.exe'; target = [pscustomobject]@{ name = 'antigravity_tools_lib'; kind = @('rlib') } },
        [pscustomobject]@{ reason = 'compiler-artifact'; profile = [pscustomobject]@{ test = $false }; executable = 'target\\debug\\deps\\antigravity_tools_lib.dll'; target = [pscustomobject]@{ name = 'antigravity_tools_lib'; kind = @('rlib') } },
        [pscustomobject]@{ reason = 'compiler-artifact'; profile = [pscustomobject]@{ test = $true }; executable = 'target\\debug\\deps\\other-test.exe'; target = [pscustomobject]@{ name = 'other'; kind = @('lib') } }
    )
    $selectedArtifacts = @($cargoMessages | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.profile.test -and $_.executable -and $_.target.name -eq 'antigravity_tools_lib' -and $_.target.kind -contains 'rlib' })
    Assert-SelfTest ($selectedArtifacts.Count -eq 1 -and $selectedArtifacts[0].executable -like '*antigravity_tools_lib-test.exe') 'Cargo rlib test-artifact selection'

    $mixedModules = @([pscustomobject]@{
        module = [pscustomobject]@{ path = 'test.exe' }
        dependencies = @(
            [pscustomobject]@{ name = 'kernel32.dll'; selected_candidate = 'C:\\Windows\\System32\\kernel32.dll'; missing_exports = @() },
            [pscustomobject]@{ name = 'local.dll'; selected_candidate = 'C:\\test\\local.dll'; missing_exports = @('RequiredExport') },
            [pscustomobject]@{ name = 'api-ms-win-core.dll'; selected_candidate = $null; missing_exports = @() }
        )
    })
    $findings = Get-MissingExportFindings -Modules $mixedModules
    Assert-SelfTest ($findings.Count -eq 1 -and $findings[0].dll -eq 'local.dll') 'mixed dependency schema'

    $powershell = Join-Path $PSHOME 'pwsh.exe'
    if (-not (Test-Path -LiteralPath $powershell)) {
        $powershell = Join-Path $PSHOME 'pwsh'
    }
    $overflow = Invoke-BoundedNative -FilePath $powershell -Arguments @('-NoProfile', '-Command', '1..8 | ForEach-Object { "line$_" }') -TimeoutSeconds 5 -MaxOutputLines 3
    Assert-SelfTest ($overflow.ExitCode -eq 0 -and $overflow.OutputTruncated -and $overflow.Lines.Count -eq 3) 'native output cap'

    $timeout = Invoke-BoundedNative -FilePath $powershell -Arguments @('-NoProfile', '-Command', 'Start-Sleep -Seconds 3') -TimeoutSeconds 1 -MaxOutputLines 3
    Assert-SelfTest $timeout.TimedOut 'native timeout'

    $nonzero = Invoke-BoundedNative -FilePath $powershell -Arguments @('-NoProfile', '-Command', 'exit 23') -TimeoutSeconds 5 -MaxOutputLines 3
    Assert-SelfTest ($nonzero.ExitCode -eq 23 -and -not $nonzero.TimedOut) 'native exit preservation'

    Write-Output 'diagnose_windows_loader self-test passed'
}

if ($SelfTest) {
    Invoke-SelfTest
    exit 0
}

if (-not $TestExecutable -or -not (Test-Path -LiteralPath $TestExecutable -PathType Leaf) -or -not $OutputDirectory) {
    throw 'TestExecutable and OutputDirectory are required unless -SelfTest is specified.'
}

$reportPath = $null
try {
    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    $script:Dumpbin = Get-DumpbinPath
    $testPath = (Resolve-Path -LiteralPath $TestExecutable).Path
    $exeDirectory = Split-Path -Parent $testPath
    $targetDebugDirectory = Split-Path -Parent $exeDirectory
    $windowsDirectory = $env:WINDIR
    $systemDirectory = Join-Path $windowsDirectory 'System32'

    # Do not print PATH. Keep only existing directories and report a path only when
    # it contains an imported DLL candidate.
    $searchDirectories = [System.Collections.Generic.List[string]]::new()
    foreach ($directory in @($exeDirectory, $targetDebugDirectory, $systemDirectory, $windowsDirectory, (Get-Location).Path) + ($env:PATH -split ';')) {
        if ($directory -and (Test-Path -LiteralPath $directory -PathType Container) -and -not $searchDirectories.Contains($directory)) {
            $searchDirectories.Add($directory)
        }
    }

    $queue = [System.Collections.Generic.Queue[object]]::new()
    $queue.Enqueue([pscustomobject]@{ path = $testPath; depth = 0 })
    $visited = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    $modules = [System.Collections.Generic.List[object]]::new()

    while ($queue.Count -gt 0 -and $modules.Count -lt $MaxModules) {
        Assert-AuditBudget
        $module = $queue.Dequeue()
        if (-not $visited.Add($module.path)) {
            continue
        }

        $imports = Get-ImportsByDll -Path $module.path
        $dependencies = [System.Collections.Generic.List[object]]::new()
        foreach ($name in @(Get-DependentNames -Path $module.path | Select-Object -First $MaxDependenciesPerModule)) {
            $requiredSymbols = @($imports[$name] | Sort-Object -Unique | Select-Object -First $MaxSymbolsPerModule)
            $candidates = [System.Collections.Generic.List[string]]::new()
            foreach ($directory in $searchDirectories) {
                $candidate = Join-Path $directory $name
                if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                    $candidates.Add($candidate)
                }
            }

            $selected = if ($candidates.Count -gt 0) { $candidates[0] } else { $null }
            $status = if (Test-IsApiSet $name) {
                'api_set_not_locally_audited'
            } elseif ($null -eq $selected) {
                'no_candidate_in_bounded_search'
            } elseif ($selected.StartsWith($systemDirectory, [System.StringComparison]::OrdinalIgnoreCase) -or $selected.StartsWith($windowsDirectory, [System.StringComparison]::OrdinalIgnoreCase)) {
                'system_candidate_not_export_audited'
            } else {
                'local_candidate_audited'
            }

            $dependency = [ordered]@{
                name                       = $name
                status                     = $status
                selected_candidate         = $selected
                candidate_paths            = @($candidates | Select-Object -First 3)
                required_exports           = $requiredSymbols
                export_check               = 'not_locally_audited'
                missing_exports            = @()
                exported_symbol_count      = 0
                exported_symbols_observed  = @()
                selected_candidate_version = $null
            }

            if ($status -eq 'local_candidate_audited') {
                $exports = Get-ExportedSymbols -Path $selected
                $missing = @($requiredSymbols | Where-Object { -not $exports.Contains($_) })
                $dependency.export_check = if ($missing.Count -eq 0) { 'all_parsed_imports_exported_or_forwarded' } else { 'parsed_imports_missing_from_local_candidate' }
                $dependency.missing_exports = $missing
                $dependency.exported_symbol_count = $exports.Count
                $dependency.exported_symbols_observed = @($exports | Sort-Object | Select-Object -First $MaxSymbolsPerModule)
                $dependency.selected_candidate_version = Get-FileVersionSummary -Path $selected
                if ($module.depth -lt 2 -and $modules.Count + $queue.Count -lt $MaxModules) {
                    $queue.Enqueue([pscustomobject]@{ path = $selected; depth = $module.depth + 1 })
                }
            }

            $dependencies.Add([pscustomobject]$dependency)
        }

        $modules.Add([pscustomobject]@{
            module = Get-FileVersionSummary -Path $module.path
            depth = $module.depth
            dependencies = $dependencies
        })
    }

    $report = [ordered]@{
        test_executable = Get-FileVersionSummary -Path $testPath
        dumpbin = $script:Dumpbin
        dumpbin_resolver = $script:DumpbinResolverEvidence
        bounds = [ordered]@{ max_modules = $MaxModules; max_dependencies_per_module = $MaxDependenciesPerModule; max_symbols_per_module = $MaxSymbolsPerModule; max_dumpbin_output_lines = $MaxDumpbinOutputLines; max_seconds = $MaxAuditSeconds; max_candidate_paths_per_dll = 3; max_recursion_depth = 2 }
        modules = $modules
        interpretation = 'Only local DLL candidates are export-audited. API sets, system DLLs, and unresolved candidates are recorded as unknown, not as a proven root cause.'
    }
    $reportPath = Join-Path $OutputDirectory 'windows-loader-pe-audit.json'
    $report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $reportPath -Encoding utf8
    $missingFindings = Get-MissingExportFindings -Modules $modules
    $summary = [ordered]@{
        report_path = $reportPath
        elapsed_seconds = [Math]::Round($auditStopwatch.Elapsed.TotalSeconds, 2)
        audited_modules = $modules.Count
        parsed_missing_local_exports = $missingFindings
        interpretation = 'Empty parsed_missing_local_exports does not prove a loader root cause; API-set, system-DLL, ordinal-only, and unresolved cases remain unknown.'
    }
    Write-Host "::notice title=Windows loader PE audit::$reportPath"
    $summary | ConvertTo-Json -Depth 8
}
catch {
    $failure = [ordered]@{
        test_executable = $TestExecutable
        dumpbin_resolver = $script:DumpbinResolverEvidence
        error = $_.Exception.Message
        interpretation = 'PE audit was unavailable; this is diagnostic unknown and does not replace the original test failure.'
    }
    if (-not $reportPath) {
        New-Item -ItemType Directory -Path $OutputDirectory -Force -ErrorAction SilentlyContinue | Out-Null
        $reportPath = Join-Path $OutputDirectory 'windows-loader-pe-audit.json'
    }
    $failure | ConvertTo-Json | Set-Content -LiteralPath $reportPath -Encoding utf8
    Write-Warning "Windows loader PE audit unavailable: $($_.Exception.Message)"
    $failure | ConvertTo-Json
}

# The workflow caller must preserve the focused cargo test exit code.
exit 0
