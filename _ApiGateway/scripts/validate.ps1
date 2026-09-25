$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

docker compose --project-directory $root --env-file (Join-Path $root '.env.example') config --quiet
docker run --rm `
  --env-file (Join-Path $root '.env.example') `
  -e FC_ENABLE=1 `
  -e FC_OUT=/tmp/krakend.json `
  -v "${root}/config:/etc/krakend:ro" `
  krakend:2.13.3 check -n -t -c /etc/krakend/krakend.tmpl

