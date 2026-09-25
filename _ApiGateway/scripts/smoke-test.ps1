$ErrorActionPreference = 'Stop'
$baseUrl = if ($env:PPS_GATEWAY_URL) { $env:PPS_GATEWAY_URL.TrimEnd('/') } else { 'http://localhost:8080' }

$health = Invoke-RestMethod "$baseUrl/__health"
if ($health.status -ne 'ok') { throw 'KrakenD health endpoint did not return status=ok.' }

$catalog = Invoke-RestMethod "$baseUrl/api/v1/catalog/health"
if ($catalog.status -ne 'healthy') { throw 'Catalog route did not return status=healthy.' }

Write-Host 'PPS API Gateway smoke tests passed.'

