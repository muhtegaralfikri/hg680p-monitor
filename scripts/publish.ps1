param(
    [string]$SshHost = "root@192.168.2.10",
    [string]$RemoteDir = "/srv/src",
    [string]$Archive = "D:\github_project\hg680p-monitor-update.tar.gz"
)

$ErrorActionPreference = "Stop"

$ProjectDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$ParentDir = Split-Path $ProjectDir -Parent
$ProjectName = Split-Path $ProjectDir -Leaf
$ServerScript = Join-Path $ProjectDir "deploy\server-update.sh"

if (-not (Test-Path $ServerScript)) {
    throw "Server deploy script not found: $ServerScript"
}

Write-Host "Packaging $ProjectDir"
tar -czf $Archive -C $ParentDir --exclude "$ProjectName/target" --exclude "$ProjectName/.git" $ProjectName

Write-Host "Uploading archive to ${SshHost}:$RemoteDir"
scp $Archive "${SshHost}:$RemoteDir/"

Write-Host "Running server deploy"
Get-Content $ServerScript | ssh $SshHost "bash -s"
