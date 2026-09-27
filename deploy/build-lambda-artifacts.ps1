[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$artifactRoot = Join-Path $repoRoot "deploy\artifacts"
$expectedRoot = Join-Path $repoRoot "deploy"
if (-not $artifactRoot.StartsWith($expectedRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to operate outside the repository deploy directory."
}

if (Test-Path -LiteralPath $artifactRoot) {
  Remove-Item -LiteralPath $artifactRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $artifactRoot | Out-Null

$services = @(
  @{ Name = "cart"; Directory = "PPS_CartService"; Prisma = $false },
  @{ Name = "order"; Directory = "PPS_OrderService"; Prisma = $true },
  @{ Name = "payment"; Directory = "PPS_PaymentService"; Prisma = $true }
)

Push-Location (Join-Path $repoRoot "packages\messaging")
try { npm run build } finally { Pop-Location }

$messagingTarget = Join-Path $artifactRoot "packages\messaging"
New-Item -ItemType Directory -Force -Path $messagingTarget | Out-Null
Copy-Item (Join-Path $repoRoot "packages\messaging\package.json") $messagingTarget
Copy-Item (Join-Path $repoRoot "packages\messaging\dist") $messagingTarget -Recurse

foreach ($service in $services) {
  $source = Join-Path $repoRoot $service.Directory
  $target = Join-Path $artifactRoot $service.Name
  Push-Location $source
  try {
    if ($service.Prisma) { npm run db:generate }
    npm run build
  } finally { Pop-Location }

  New-Item -ItemType Directory -Force -Path $target | Out-Null
  Copy-Item (Join-Path $source "dist") $target -Recurse
  Copy-Item (Join-Path $source "package.json") $target
  Copy-Item (Join-Path $source "package-lock.json") $target
  if ($service.Prisma) { Copy-Item (Join-Path $source "prisma") $target -Recurse }

  Push-Location $target
  try {
    if ($service.Prisma) {
      npm ci
      npx prisma generate
      npm prune --omit=dev
    } else {
      npm ci --omit=dev
    }
  } finally { Pop-Location }

  if ($service.Name -in @("order", "payment")) {
    Push-Location $target
    try {
      npm install --omit=dev --ignore-scripts --no-save "@aws-sdk/client-sns@^3.1141.0" "amqp-connection-manager@5.0.0" "amqplib@2.0.1"
    } finally { Pop-Location }

    $linkedMessaging = Join-Path $target "node_modules\@pps\messaging"
    if (Test-Path -LiteralPath $linkedMessaging) {
      Remove-Item -LiteralPath $linkedMessaging -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $linkedMessaging | Out-Null
    Copy-Item (Join-Path $messagingTarget "package.json") $linkedMessaging
    Copy-Item (Join-Path $messagingTarget "dist") $linkedMessaging -Recurse

    $engineCache = Join-Path $target "node_modules\@prisma\engines\node_modules\.cache"
    if (Test-Path -LiteralPath $engineCache) {
      Remove-Item -LiteralPath $engineCache -Recurse -Force
    }

    @(
      "node_modules\.cache",
      "node_modules\prisma",
      "node_modules\@prisma\engines",
      "node_modules\@prisma\internals",
      "node_modules\@typescript",
      "node_modules\typescript"
    ) | ForEach-Object {
      $buildOnlyPath = Join-Path $target $_
      if (Test-Path -LiteralPath $buildOnlyPath) {
        Remove-Item -LiteralPath $buildOnlyPath -Recurse -Force
      }
    }

    Get-ChildItem (Join-Path $target "node_modules\.prisma\client") -Filter "*windows*.dll.node" -ErrorAction SilentlyContinue |
      Remove-Item -Force
  }
}

Write-Host "Lambda artifacts are ready at $artifactRoot"
