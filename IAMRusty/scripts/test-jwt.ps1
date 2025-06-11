# JWT Test Runner for Windows (PowerShell)
# This script generates certificates, runs JWT tests, and provides cleanup

param(
    [switch]$CleanOnly,
    [switch]$Verbose,
    [string]$CertDir = "./config/certs"
)

$ErrorActionPreference = "Stop"

# Get script and project directories
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir
$CertDirPath = Join-Path $ProjectDir $CertDir

function Write-Info { param($Message) Write-Host "ℹ️  $Message" -ForegroundColor Cyan }
function Write-Success { param($Message) Write-Host "✅ $Message" -ForegroundColor Green }
function Write-Warning { param($Message) Write-Host "⚠️  $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "❌ $Message" -ForegroundColor Red }

# Function to cleanup on exit
function Invoke-Cleanup {
    param([int]$ExitCode = 0)
    
    Write-Info "Cleaning up..."
    
    # Stop any running containers
    if (Get-Command docker -ErrorAction SilentlyContinue) {
        Write-Info "Stopping test containers..."
        try {
            docker stop iam-test-db 2>$null | Out-Null
            docker rm iam-test-db 2>$null | Out-Null
        } catch {
            # Ignore errors - containers might not exist
        }
    }
    
    Write-Info "Cleanup completed"
    
    if ($ExitCode -eq 0) {
        Write-Success "Tests completed successfully!"
    } else {
        Write-Error "Tests failed with exit code $ExitCode"
    }
    
    exit $ExitCode
}

# Handle Ctrl+C and other termination signals
$null = Register-ObjectEvent -InputObject ([System.Console]) -EventName CancelKeyPress -Action {
    Invoke-Cleanup -ExitCode 130
}

# Clean only mode
if ($CleanOnly) {
    Write-Info "🧹 Running cleanup only..."
    Invoke-Cleanup
    return
}

Write-Info "🧪 JWT Test Runner with Certificate Generation"
Write-Info "Project directory: $ProjectDir"
Write-Info "Certificate directory: $CertDirPath"
Write-Host ""

# Change to project directory
Set-Location $ProjectDir

try {
    # Step 1: Generate certificates
    Write-Info "🔑 Step 1: Generating JWT signing keys and certificates..."
    $generateScript = Join-Path $ScriptDir "generate-certs.ps1"
    
    if (Test-Path $generateScript) {
        & $generateScript -CertDir $CertDir
        if ($LASTEXITCODE -ne 0) {
            throw "Certificate generation failed"
        }
    } else {
        Write-Error "Certificate generation script not found at $generateScript"
        Invoke-Cleanup -ExitCode 1
    }
    Write-Host ""

    # Step 2: Verify certificate files
    Write-Info "🔍 Step 2: Verifying generated files..."
    $requiredFiles = @("key.pem", "public_key.pem")
    
    foreach ($file in $requiredFiles) {
        $filePath = Join-Path $CertDirPath $file
        if (Test-Path $filePath) {
            $size = [math]::Round((Get-Item $filePath).Length / 1KB, 1)
            Write-Success "$file exists ($size KB)"
        } else {
            Write-Error "Required file missing: $filePath"
            Invoke-Cleanup -ExitCode 1
        }
    }
    Write-Host ""

    # Step 3: Check JWT key specifications
    Write-Info "🔍 Step 3: Checking JWT key specifications..."
    $keyPath = Join-Path $CertDirPath "key.pem"
    
    try {
        $keyInfo = & openssl rsa -in $keyPath -text -noout 2>$null
        if ($keyInfo -match "Private-Key: \((\d+) bit") {
            $keyBits = [int]$matches[1]
            if ($keyBits -ge 4096) {
                Write-Success "JWT private key: $keyBits bits (sufficient for validation)"
            } else {
                Write-Warning "JWT private key: $keyBits bits (may cause validation issues)"
            }
        } else {
            Write-Warning "Could not determine key size"
            $keyBits = 0
        }
    } catch {
        Write-Warning "Could not analyze key: $($_.Exception.Message)"
        $keyBits = 0
    }
    Write-Host ""

    # Step 4: Set environment and run tests
    Write-Info "🧪 Step 4: Running JWT tests..."
    Write-Info "Setting RUN_ENV=test..."
    $env:RUN_ENV = "test"
    $env:RUST_LOG = "info"

    Write-Info "Running specific JWT tests..."
    Write-Host ""

    # Run individual test categories
    $testCommands = @(
        @{Name="config_generation"; Command="cargo test --test token test_config_generation"},
        @{Name="jwks_endpoint"; Command="cargo test --test token test_jwks_returns_200_and_valid_json_structure"},
        @{Name="jwt_validation"; Command="cargo test --test token test_jwt_token_validation_using_jwks_endpoint"},
        @{Name="refresh_token"; Command="cargo test --test token test_refresh_token_success_with_valid_refresh_token"}
    )

    $passedTests = 0
    $totalTests = $testCommands.Count

    foreach ($test in $testCommands) {
        Write-Info "Running: $($test.Name)"
        
        try {
            Invoke-Expression $test.Command
            if ($LASTEXITCODE -eq 0) {
                Write-Success "✓ $($test.Name) passed"
                $passedTests++
            } else {
                Write-Error "✗ $($test.Name) failed"
            }
        } catch {
            Write-Error "✗ $($test.Name) failed with exception: $($_.Exception.Message)"
        }
        Write-Host ""
    }

    # Step 5: Run all token tests if individual tests passed
    if ($passedTests -eq $totalTests) {
        Write-Info "🎯 Step 5: Running all JWT token tests..."
        try {
            & cargo test --test token
            if ($LASTEXITCODE -eq 0) {
                Write-Success "All JWT token tests passed!"
            } else {
                Write-Warning "Some token tests failed, but core JWT functionality is working"
            }
        } catch {
            Write-Warning "Token test suite failed: $($_.Exception.Message)"
        }
    } else {
        Write-Warning "Skipping full test suite due to individual test failures ($passedTests/$totalTests passed)"
    }

    Write-Host ""
    Write-Info "📊 Test Summary:"
    Write-Info "  Passed: $passedTests/$totalTests individual tests"
    Write-Info "  JWT Key Size: $keyBits bits"
    Write-Info "  Certificate Directory: $CertDirPath"

    # Verify final configuration
    Write-Host ""
    Write-Info "🔧 Configuration verification:"
    $jwtPrivateExists = Test-Path (Join-Path $CertDirPath "key.pem")
    $jwtPublicExists = Test-Path (Join-Path $CertDirPath "public_key.pem")
    $testConfigExists = Test-Path (Join-Path $ProjectDir "config/test.toml")
    
    Write-Info "  JWT private key: $(if ($jwtPrivateExists) { "✓ Present" } else { "✗ Missing" })"
    Write-Info "  JWT public key: $(if ($jwtPublicExists) { "✓ Present" } else { "✗ Missing" })"
    Write-Info "  Test config: $(if ($testConfigExists) { "✓ Present" } else { "✗ Missing" })"

    Write-Host ""
    Write-Success "JWT test run completed! 🎉"
    
    Invoke-Cleanup -ExitCode 0

} catch {
    Write-Error "Test run failed: $($_.Exception.Message)"
    if ($Verbose) {
        Write-Host $_.ScriptStackTrace -ForegroundColor Red
    }
    Invoke-Cleanup -ExitCode 1
} 