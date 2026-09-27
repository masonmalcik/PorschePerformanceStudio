[CmdletBinding()]
param()

$ErrorActionPreference = "Continue"

function Invoke-AuditCommand {
    param(
        [Parameter(Mandatory)] [string] $Title,
        [Parameter(Mandatory)] [scriptblock] $Command
    )
    Write-Host "`n=== $Title ===" -ForegroundColor Cyan
    try {
        & $Command
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Status: unavailable or unhealthy (exit $LASTEXITCODE)" -ForegroundColor Yellow
        } else {
            Write-Host "Status: OK" -ForegroundColor Green
        }
    } catch {
        Write-Host "Status: unavailable - $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Invoke-AuditCommand "Docker CLI" { docker --version }
Invoke-AuditCommand "Docker Engine" { docker version }
Invoke-AuditCommand "Docker Context" { docker context show; docker context ls }
Invoke-AuditCommand "Docker Compose" { docker compose version }
Invoke-AuditCommand "Docker Desktop Containers" { docker ps --format "table {{.Names}}`t{{.Status}}`t{{.Ports}}" }
Invoke-AuditCommand "kubectl Client" { kubectl version --client --output=yaml }
Invoke-AuditCommand "Kubernetes Contexts" { kubectl config get-contexts; kubectl config current-context }
Invoke-AuditCommand "Kubernetes Cluster" { kubectl cluster-info }

Write-Host "`nAudit complete." -ForegroundColor Cyan
