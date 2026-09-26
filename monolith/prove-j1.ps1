# J1 proof: host oodhive-monolith (not deploy/ nginx stub).
# Expect: GET /health 200 OK ; POST /lazaret/invoke 401 JSON {"error":"unauthorized"} (not "ok").
$ErrorActionPreference = "Continue"
$healthUrl = "http://127.0.0.1:8080/health"
$invokeUrl = "http://127.0.0.1:8080/lazaret/invoke?project_id=00000000-0000-0000-0000-000000000000"
$bodyFile = Join-Path $PSScriptRoot "prove-j1-body.json"

Write-Host "GET $healthUrl"
& curl.exe -sS -D - -o - $healthUrl
Write-Host ""
Write-Host "POST $invokeUrl (no Authorization)"
& curl.exe -sS -D - -o - -X POST $invokeUrl -H "Content-Type: application/json" --data-binary "@$bodyFile"
Write-Host ""
Write-Host "POST $invokeUrl (dummy Bearer)"
& curl.exe -sS -D - -o - -X POST $invokeUrl -H "Content-Type: application/json" -H "Authorization: Bearer dummy" --data-binary "@$bodyFile"
Write-Host ""
