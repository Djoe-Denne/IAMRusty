# J1 host oodhive-monolith HTTP proof (curl.exe). Replayable. No Rust tests.
# Flow: signup -> complete-registration -> verify (MailHog or IAM table, as IT) -> login
#       -> POST /manifesto/api/projects -> POST components (io.aiforall.reference-kv)
#       -> POST /lazaret/invoke
# Stop after the first blocking business failure (request + code + body already printed).

$ErrorActionPreference = "Continue"
$Base = "http://127.0.0.1:8080"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$Stamp = Get-Date -Format "yyyyMMddHHmmss"
$Email = "e2e-$Stamp@local.test"
$Password = "E2eCurl1234"
$Username = "e2e$Stamp"
$ProjectName = "e2e-$Stamp"

function Invoke-CurlStep {
    param(
        [Parameter(Mandatory = $true)][string]$Method,
        [Parameter(Mandatory = $true)][string]$Url,
        [string]$Body = $null,
        [string[]]$Headers = @(),
        [int[]]$Ok = @(200),
        [switch]$AllowFail
    )

    Write-Host ""
    Write-Host "=== $Method $Url ==="
    if ($Body) {
        Write-Host "request-body: $Body"
    }

    $tmpBody = Join-Path $env:TEMP ("prove-e2e-body-" + [guid]::NewGuid().ToString() + ".txt")
    $tmpHdr = Join-Path $env:TEMP ("prove-e2e-hdr-" + [guid]::NewGuid().ToString() + ".txt")
    $curlArgs = @(
        "-sS", "-X", $Method, $Url,
        "-o", $tmpBody,
        "-D", $tmpHdr,
        "-w", "%{http_code}"
    )
    foreach ($h in $Headers) {
        $curlArgs += @("-H", $h)
    }
    $tmpJson = $null
    if ($null -ne $Body) {
        $tmpJson = Join-Path $env:TEMP ("prove-e2e-req-" + [guid]::NewGuid().ToString() + ".json")
        $utf8 = New-Object System.Text.UTF8Encoding $false
        [System.IO.File]::WriteAllText($tmpJson, $Body, $utf8)
        $curlArgs += @("-H", "Content-Type: application/json", "--data-binary", "@$tmpJson")
    }

    $code = & curl.exe @curlArgs
    $exit = $LASTEXITCODE
    $respBody = ""
    if (Test-Path $tmpBody) {
        $respBody = Get-Content -Raw -LiteralPath $tmpBody -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $tmpBody -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $tmpHdr -Force -ErrorAction SilentlyContinue
    if ($tmpJson) {
        Remove-Item -LiteralPath $tmpJson -Force -ErrorAction SilentlyContinue
    }

    if ($exit -ne 0 -and -not $code) {
        Write-Host "curl.exe exit=$exit (no HTTP code)"
        if (-not $AllowFail) { exit 1 }
        return [pscustomobject]@{ Code = 0; Body = ""; Ok = $false }
    }

    Write-Host "http-code: $code"
    Write-Host "response-body: $respBody"

    $codeInt = 0
    [void][int]::TryParse("$code", [ref]$codeInt)
    $isOk = $Ok -contains $codeInt
    if (-not $isOk -and -not $AllowFail) {
        Write-Host "STOP: unexpected HTTP $codeInt (wanted $($Ok -join ','))"
        exit 1
    }
    return [pscustomobject]@{ Code = $codeInt; Body = $respBody; Ok = $isOk }
}

function Get-JsonField {
    param([string]$Json, [string]$Name)
    if (-not $Json) { return $null }
    try {
        $obj = $Json | ConvertFrom-Json
        return [string]$obj.$Name
    } catch {
        return $null
    }
}

function Test-MonolithHealth {
    $code = & curl.exe -sS -o NUL -w "%{http_code}" --max-time 3 "$Base/health"
    if ($LASTEXITCODE -ne 0) { return $false }
    return "$code" -eq "200"
}

function Test-LazaretHealth {
    $code = & curl.exe -sS -o NUL -w "%{http_code}" --max-time 3 "$Base/lazaret/health"
    if ($LASTEXITCODE -ne 0) { return $false }
    return "$code" -eq "200"
}

function Wait-Monolith {
    param([int]$Seconds = 180)
    for ($i = 0; $i -lt $Seconds; $i++) {
        if ((Test-MonolithHealth) -and (Test-LazaretHealth)) {
            Write-Host "monolith ready after ${i}s"
            return $true
        }
        Start-Sleep -Seconds 1
    }
    return $false
}

function Test-ComponentCatalog {
    $code = & curl.exe -sS -o NUL -w "%{http_code}" --max-time 2 "http://127.0.0.1:9000/api/components"
    if ($LASTEXITCODE -ne 0) { return $false }
    return "$code" -eq "200"
}

function Ensure-ComponentCatalog {
    if (Test-ComponentCatalog) {
        Write-Host "GET http://127.0.0.1:9000/api/components already 200"
        return
    }
    Write-Host "starting just component-catalog in background"
    Start-Process -FilePath "just" -ArgumentList "component-catalog" -WorkingDirectory $RepoRoot -WindowStyle Hidden
    for ($i = 0; $i -lt 20; $i++) {
        if (Test-ComponentCatalog) {
            Write-Host "component catalog ready after ${i}s"
            return
        }
        Start-Sleep -Seconds 1
    }
    Write-Host "STOP: catalog stub http://127.0.0.1:9000/api/components not 200 (just component-catalog)"
    exit 1
}

function Ensure-HostMonolith {
    Set-Location $RepoRoot
    if ((Test-MonolithHealth) -and (Test-LazaretHealth)) {
        Write-Host "GET $Base/health already 200 and /lazaret/health 200 (host monolith)."
        return
    }

    if (Test-MonolithHealth) {
        Write-Host "Port 8080 answers /health but not /lazaret/health - not the monolith. Stopping compose iam-service."
        docker compose stop iam-service
        Start-Sleep -Seconds 2
    }

    Write-Host "just up-infra"
    just up-infra
    if ($LASTEXITCODE -ne 0) {
        Write-Host "STOP: just up-infra failed"
        exit 1
    }

    Write-Host 'docker compose up -d mailhog (optional, not in up-infra)'
    docker compose up -d mailhog
    Write-Host "mailhog compose exit=$LASTEXITCODE"

    Write-Host "starting just monolith in background"
    Start-Process -FilePath "just" -ArgumentList "monolith" -WorkingDirectory $RepoRoot -WindowStyle Hidden
    if (-not (Wait-Monolith -Seconds 300)) {
        Write-Host "STOP: monolith /health did not become 200 within 300s"
        exit 1
    }
}

function Get-MailhogToken {
    param([string]$TargetEmail)
    $raw = & curl.exe -sS --max-time 5 "http://127.0.0.1:8025/api/v2/messages"
    if ($LASTEXITCODE -ne 0 -or -not $raw) { return $null }
    try {
        $msg = $raw | ConvertFrom-Json
        foreach ($item in @($msg.items)) {
            $blob = ($item | ConvertTo-Json -Depth 8)
            if ($blob -notmatch [regex]::Escape($TargetEmail)) { continue }
            $m = [regex]::Match($blob, 'token=([A-Za-z0-9_-]{10,100})')
            if ($m.Success) { return $m.Groups[1].Value }
        }
    } catch {
        return $null
    }
    return $null
}

function Get-DbVerificationToken {
    param([string]$TargetEmail)
    # Same as IAMRusty/tests/auth_username_flow.rs (IT reads user_email_verification).
    $sql = "SELECT verification_token FROM user_email_verification WHERE email = '$TargetEmail' ORDER BY created_at DESC LIMIT 1;"
    $token = docker compose exec -T postgres psql -U postgres -d iam_dev -t -A -c $sql
    if ($LASTEXITCODE -ne 0) { return $null }
    $token = ($token | Out-String).Trim()
    if ($token -and $token -notmatch "ERROR") { return $token }
    return $null
}

Ensure-HostMonolith
Ensure-ComponentCatalog

$h = Invoke-CurlStep -Method GET -Url "$Base/health" -Ok @(200)
Write-Host "health body is expected OK"

# --- 1. signup ---
$signupBody = (@{ email = $Email; password = $Password } | ConvertTo-Json -Compress)
$signup = Invoke-CurlStep -Method POST -Url "$Base/iam/api/auth/signup" -Body $signupBody -Ok @(202, 200)
$registrationToken = Get-JsonField -Json $signup.Body -Name "registration_token"
if (-not $registrationToken) {
    Write-Host "STOP: signup returned no registration_token"
    exit 1
}

$completeBody = (@{ registration_token = $registrationToken; username = $Username } | ConvertTo-Json -Compress)
$complete = Invoke-CurlStep -Method POST -Url "$Base/iam/api/auth/complete-registration" -Body $completeBody -Ok @(200)
$accessFromComplete = Get-JsonField -Json $complete.Body -Name "access_token"

# --- 2. login (verify email if IAM refuses) ---
$loginBody = (@{ email = $Email; password = $Password } | ConvertTo-Json -Compress)
$login = Invoke-CurlStep -Method POST -Url "$Base/iam/api/auth/login" -Body $loginBody -Ok @(200) -AllowFail
$accessToken = Get-JsonField -Json $login.Body -Name "access_token"

if (-not $login.Ok) {
    Write-Host "login not 200 - trying email verify (MailHog then IAM table, as IT)"
    $verifyToken = Get-MailhogToken -TargetEmail $Email
    if ($verifyToken) {
        Write-Host "verify token source: MailHog :8025"
    } else {
        $verifyToken = Get-DbVerificationToken -TargetEmail $Email
        if ($verifyToken) {
            Write-Host "verify token source: postgres iam_dev.user_email_verification (IT path)"
        }
    }
    if (-not $verifyToken) {
        Write-Host "STOP: no verification token (MailHog down and table empty). Queue is disabled on J1 so Telegraph never sends mail."
        exit 1
    }
    $encEmail = [uri]::EscapeDataString($Email)
    $encToken = [uri]::EscapeDataString($verifyToken)
    $null = Invoke-CurlStep -Method GET -Url "$Base/iam/api/auth/verify?email=$encEmail&token=$encToken" -Ok @(200)
    $login = Invoke-CurlStep -Method POST -Url "$Base/iam/api/auth/login" -Body $loginBody -Ok @(200)
    $accessToken = Get-JsonField -Json $login.Body -Name "access_token"
}

if (-not $accessToken) {
    if ($accessFromComplete) {
        Write-Host "login body had no access_token; using complete-registration token"
        $accessToken = $accessFromComplete
    } else {
        Write-Host "STOP: no access_token after login"
        exit 1
    }
}

$auth = @("Authorization: Bearer $accessToken")

# --- 3. create project (Manifesto, not Hive) ---
$projectBody = (@{
    name = $ProjectName
    owner_type = "personal"
    visibility = "private"
} | ConvertTo-Json -Compress)
$project = Invoke-CurlStep -Method POST -Url "$Base/manifesto/api/projects" -Body $projectBody -Headers $auth -Ok @(201)
$projectId = Get-JsonField -Json $project.Body -Name "id"
if (-not $projectId) {
    Write-Host "STOP: create project returned no id"
    exit 1
}

# --- 4. add Apparatus KV (poll: outbox + sentinel-sync grant is async) ---
$compBody = (@{ component_type = "io.aiforall.reference-kv" } | ConvertTo-Json -Compress)
$compUrl = "$Base/manifesto/api/projects/$projectId/components"
$comp = $null
for ($i = 0; $i -lt 30; $i++) {
    $comp = Invoke-CurlStep -Method POST -Url $compUrl -Body $compBody -Headers $auth -Ok @(201) -AllowFail
    if ($comp.Ok) { break }
    if ($comp.Code -ne 403) {
        Write-Host "STOP: unexpected HTTP $($comp.Code) (wanted 201, 403=wait for sentinel-sync)"
        exit 1
    }
    Write-Host "add_component 403 - waiting for sentinel-sync Admin grant ($($i+1)/30)"
    Start-Sleep -Seconds 1
}
if (-not $comp -or -not $comp.Ok) {
    Write-Host "STOP: POST components still 403 after 30s (sentinel-sync grant missing)"
    exit 1
}
$componentId = Get-JsonField -Json $comp.Body -Name "id"
if (-not $componentId) {
    Write-Host "STOP: add_component returned no id"
    exit 1
}

# --- 5. invoke ---
$opId = [guid]::NewGuid().ToString()
$invokeBody = (@{
    binding_id = $componentId
    operation_id = $opId
    operation = "kv.get"
    params = @{ key = "e2e" }
} | ConvertTo-Json -Compress -Depth 5)
$null = Invoke-CurlStep -Method POST -Url "$Base/lazaret/invoke?project_id=$projectId" -Body $invokeBody -Headers $auth -Ok @(200)

Write-Host ""
Write-Host "E2E OK through invoke. email=$Email project_id=$projectId component_id=$componentId"
exit 0
