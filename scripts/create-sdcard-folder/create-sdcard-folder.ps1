# Download mapped files from latest releases for a list of GitHub repos.
#
# Repo entry format:
# @{
#   Repo = "owner/repo"
#   Files = @(
#     @{ Source = "some-file.nro"; Destination = "mods/" }
#     @{ Source = "?plugin.nro"; Destination = "plugins/" }
#     @{ Source = "switch/subdir/plugin.nro"; Destination = "plugins/custom-name.nro" }
#   )
# }
#
# Only listed Source files are copied.
# If Source starts with '?', it becomes a search match against zip-relative path
# and file name (case-insensitive).
# Destination rules (relative to $OutputFolder):
# - Ends with '/' or '\\': treated as folder, output name comes from Source.
# - Otherwise: treated as full path, last component is output file name.
#
# Pass -Emulator to skip console-only files (Horizon-OC).

param(
    [switch]$Emulator
)

# --- Repos to process ---
$SSBUSkylineExefsFolder = "atmosphere/contents/01006A800016E000/exefs/"
$SSBUSkylinePluginsFolder = "atmosphere/contents/01006A800016E000/romfs/skyline/plugins/"
$SsbuSyncServiceFolder = "atmosphere/contents/00FF0000BEEFB001/"
$Repos = @(
    @{
        Repo = "WuBoytH/ozone"
        Files = @(
            @{ Source = "?exefs/main.npdm"; Destination = $SSBUSkylineExefsFolder }
            @{ Source = "?exefs/subsdk9"; Destination = $SSBUSkylineExefsFolder }
        )
    }
    @{
        Repo = "raytwo/arcropolis"
        Files = @(
            @{ Source = "?libarcropolis.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
    @{
        Repo = "ultimate-research/nro-hook-plugin"
        Files = @(
            @{ Source = "?libnro_hook.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
    @{
        Repo = "HDR-Development/smashline"
        Files = @(
            @{ Source = "?libsmashline_plugin.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
    @{
        Repo = "Coolsonickirby/imgui-smash"
        Files = @(
            @{ Source = "?libimgui_smash.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
    @{
        Repo = "project-ultelier/ssbu-pia-interface"
        Files = @(
            @{ Source = "?libssbu_pia_manager.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
    @{
        Repo = "project-ultelier/smash-ultelier"
        Files = @(
            @{ Source = "?00FF0000BEEFB001/exefs.nsp"; Destination = $SsbuSyncServiceFolder }
            @{ Source = "?00FF0000BEEFB001/flags/boot2.flag"; Destination = "$($SsbuSyncServiceFolder)flags/boot2.flag" }
            @{ Source = "?libssbusync.nro"; Destination = $SSBUSkylinePluginsFolder }
            @{ Source = "?libssbu_oc_bridge.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
    @{
        Repo = "saad-script/ssbu-online-deluxe"
        Files = @(
            @{ Source = "?libssbu_online_deluxe.nro"; Destination = $SSBUSkylinePluginsFolder }
        )
    }
)

# --- Console only ---
$HorizonOCServiceFolder = "atmosphere/contents/00FF0000636C6BFF/"
if (-not $Emulator)
{
    $Repos += @{
        Repo = "Horizon-OC/Horizon-OC"
        Files = @(
            @{ Source = "?atmosphere/exosphere.bin"; Destination = "atmosphere/" }
            @{ Source = "?atmosphere/kips/hoc.kip"; Destination = "atmosphere/kips/" }
            @{ Source = "?00FF0000636C6BFF/exefs.nsp"; Destination = $HorizonOCServiceFolder }
            @{ Source = "?00FF0000636C6BFF/toolbox.json"; Destination = $HorizonOCServiceFolder }
            @{ Source = "?00FF0000636C6BFF/flags/boot2.flag"; Destination = "$($HorizonOCServiceFolder)flags/boot2.flag" }
        )
    }
}

# --- Output folder ---
$OutputFolder = Join-Path $PSScriptRoot "sdcard"
if (Test-Path $OutputFolder)
{
    Write-Host "Cleaning output folder: $OutputFolder" -ForegroundColor Green
    Remove-Item $OutputFolder -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputFolder -Force | Out-Null

# Optional: set GITHUB_TOKEN env var to avoid API rate limits
# $env:GITHUB_TOKEN = "ghp_..."

$Headers = @{
    "User-Agent" = "PowerShell-Release-Downloader"
    "Accept"     = "application/vnd.github+json"
}
if ($env:GITHUB_TOKEN)
{
    $Headers["Authorization"] = "Bearer $($env:GITHUB_TOKEN)"
}

function Normalize-MatchPath
{
    param([string]$PathText)

    if ([string]::IsNullOrWhiteSpace($PathText))
    {
        return ""
    }

    return ($PathText -replace "\\", "/").TrimStart(".", "/").ToLowerInvariant()
}

function Get-RelativePathCompat
{
    param(
        [Parameter(Mandatory = $true)]
        [string]$BasePath,
        [Parameter(Mandatory = $true)]
        [string]$TargetPath
    )

    $baseFull = [System.IO.Path]::GetFullPath($BasePath)
    $targetFull = [System.IO.Path]::GetFullPath($TargetPath)

    try
    {
        return [System.IO.Path]::GetRelativePath($baseFull, $targetFull)
    } catch
    {
        # Windows PowerShell 5.1 runs on .NET Framework where Path.GetRelativePath is unavailable.
        $baseUri = [System.Uri]::new(($baseFull.TrimEnd([char[]]@([char]'\', [char]'/')) + [System.IO.Path]::DirectorySeparatorChar))
        $targetUri = [System.Uri]::new($targetFull)
        $relativeUri = $baseUri.MakeRelativeUri($targetUri)
        $relativePath = [System.Uri]::UnescapeDataString($relativeUri.ToString())
        return $relativePath -replace '/', [System.IO.Path]::DirectorySeparatorChar
    }
}

function Resolve-OutputDirectory
{
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,
        [Parameter(Mandatory = $true)]
        [string]$RelativeFolder
    )

    if ([System.IO.Path]::IsPathRooted($RelativeFolder))
    {
        throw "Destination path must be relative: $RelativeFolder"
    }

    $rootFull = [System.IO.Path]::GetFullPath($Root)
    $combinedFull = [System.IO.Path]::GetFullPath((Join-Path $rootFull $RelativeFolder))

    if (-not $combinedFull.StartsWith($rootFull, [System.StringComparison]::OrdinalIgnoreCase))
    {
        throw "Destination escapes output folder: $RelativeFolder"
    }

    return $combinedFull
}

function Build-RepoConfig
{
    param([Parameter(Mandatory = $true)]$Entry)

    if (-not $Entry.Repo)
    {
        throw "Repo entry object is missing 'Repo'."
    }

    if (-not $Entry.Files)
    {
        throw "Repo '$($Entry.Repo)' is missing 'Files'."
    }

    $rules = @()
    foreach ($pair in $Entry.Files)
    {
        if (-not $pair.Source -or -not $pair.Destination)
        {
            throw "Each Files item needs Source and Destination."
        }

        $sourceRaw = [string]$pair.Source
        $searchMode = $sourceRaw.StartsWith("?")
        $matchSource = if ($searchMode)
        { $sourceRaw.Substring(1) 
        } else
        { $sourceRaw 
        }
        $sourceFileName = [System.IO.Path]::GetFileName($matchSource)
        $destinationRaw = [string]$pair.Destination

        if ([string]::IsNullOrWhiteSpace($matchSource))
        {
            throw "Source cannot be empty."
        }

        if ([string]::IsNullOrWhiteSpace($sourceFileName))
        {
            throw "Source must include a file name."
        }

        if ([string]::IsNullOrWhiteSpace($destinationRaw))
        {
            throw "Destination must not be empty."
        }

        $destinationIsFolder = $destinationRaw.EndsWith("/") -or $destinationRaw.EndsWith("\\")
        if ($destinationIsFolder)
        {
            $destinationFolder = $destinationRaw.TrimEnd('/', '\')
            $outputFileName = $sourceFileName
        } else
        {
            $destinationFolder = [System.IO.Path]::GetDirectoryName($destinationRaw)
            $outputFileName = [System.IO.Path]::GetFileName($destinationRaw)

            if ([string]::IsNullOrWhiteSpace($outputFileName))
            {
                throw "Destination must include a file name: $destinationRaw"
            }

            if ([string]::IsNullOrWhiteSpace($destinationFolder))
            {
                $destinationFolder = "."
            }
        }

        $rules += [PSCustomObject]@{
            Source = $sourceRaw
            SearchMode = $searchMode
            SourceNorm = Normalize-MatchPath -PathText $matchSource
            DestinationFolder = $destinationFolder
            OutputFileName = $outputFileName
            Matched = $false
        }
    }

    return [PSCustomObject]@{
        RepoName = [string]$Entry.Repo
        FileRules = $rules
    }
}

function Test-RuleMatch
{
    param(
        [Parameter(Mandatory = $true)]$Rule,
        [Parameter(Mandatory = $true)][string]$CandidatePathNorm,
        [Parameter(Mandatory = $true)][string]$CandidateNameNorm
    )

    if ($Rule.SearchMode)
    {
        return $CandidatePathNorm.Contains($Rule.SourceNorm) -or $CandidateNameNorm.Contains($Rule.SourceNorm)
    }

    return $Rule.SourceNorm -eq $CandidatePathNorm -or $Rule.SourceNorm -eq $CandidateNameNorm
}

foreach ($repoEntry in $Repos)
{
    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("release-download-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null

    try
    {
        try
        {
            $config = Build-RepoConfig -Entry $repoEntry
            $repoName = $config.RepoName
            $fileRules = @($config.FileRules)

            if ($fileRules.Count -eq 0)
            {
                Write-Warning "No Files mappings configured for $repoName."
                continue
            }

            Write-Host "Processing $repoName ..." -ForegroundColor Cyan

            $apiUrl = "https://api.github.com/repos/$repoName/releases/latest"
            $release = Invoke-RestMethod -Uri $apiUrl -Headers $Headers -Method Get

            if (-not $release.assets -or $release.assets.Count -eq 0)
            {
                Write-Warning "No assets found for $repoName latest release."
                continue
            }

            foreach ($asset in $release.assets)
            {
                $assetNameLower = $asset.name.ToLowerInvariant()

                if ($assetNameLower.EndsWith(".zip"))
                {
                    $zipPath = Join-Path $tempRoot $asset.name
                    $extractDir = Join-Path $tempRoot ([System.IO.Path]::GetFileNameWithoutExtension($asset.name))

                    Write-Host "  Downloading .zip $($asset.name)"
                    Invoke-WebRequest -Uri $asset.browser_download_url -Headers $Headers -OutFile $zipPath

                    New-Item -ItemType Directory -Path $extractDir -Force | Out-Null
                    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

                    $extractedFiles = Get-ChildItem -Path $extractDir -Recurse -File
                    if (-not $extractedFiles -or $extractedFiles.Count -eq 0)
                    {
                        Write-Warning "    No files found in $($asset.name)"
                        continue
                    }

                    foreach ($item in $extractedFiles)
                    {
                        $relativePath = Get-RelativePathCompat -BasePath $extractDir -TargetPath $item.FullName
                        $relativeNorm = Normalize-MatchPath -PathText $relativePath
                        $nameNorm = Normalize-MatchPath -PathText $item.Name

                        $matchingRules = @(
                            $fileRules | Where-Object {
                                -not $_.Matched -and (Test-RuleMatch -Rule $_ -CandidatePathNorm $relativeNorm -CandidateNameNorm $nameNorm)
                            }
                        )

                        foreach ($rule in $matchingRules)
                        {
                            $targetDir = Resolve-OutputDirectory -Root $OutputFolder -RelativeFolder $rule.DestinationFolder
                            New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
                            $targetPath = Join-Path $targetDir $rule.OutputFileName

                            Copy-Item -Path $item.FullName -Destination $targetPath -Force
                            Write-Host "    Copied mapped $relativePath -> $($rule.DestinationFolder)/$($rule.OutputFileName)"
                            $rule.Matched = $true
                        }
                    }
                    continue
                }

                # Non-zip assets can also match mapping rules directly by name.
                $assetNorm = Normalize-MatchPath -PathText $asset.name
                $matchingRules = @(
                    $fileRules | Where-Object {
                        -not $_.Matched -and (Test-RuleMatch -Rule $_ -CandidatePathNorm $assetNorm -CandidateNameNorm $assetNorm)
                    }
                )

                foreach ($rule in $matchingRules)
                {
                    $targetDir = Resolve-OutputDirectory -Root $OutputFolder -RelativeFolder $rule.DestinationFolder
                    New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
                    $targetPath = Join-Path $targetDir $rule.OutputFileName

                    Write-Host "  Downloading mapped asset $($asset.name) -> $($rule.DestinationFolder)/$($rule.OutputFileName)"
                    Invoke-WebRequest -Uri $asset.browser_download_url -Headers $Headers -OutFile $targetPath
                    $rule.Matched = $true
                }
                continue
            }

            $missing = @($fileRules | Where-Object { -not $_.Matched })
            foreach ($rule in $missing)
            {
                Write-Warning "  Mapping source not found: $($rule.Source)"
            }
        } catch
        {
            Write-Warning "Failed for entry: $($_.Exception.Message)"
        }
    } finally
    {
        if (Test-Path $tempRoot)
        {
            Remove-Item -Path $tempRoot -Recurse -Force
        }
    }
}

Write-Host "Done. Files saved in: $OutputFolder" -ForegroundColor Green
