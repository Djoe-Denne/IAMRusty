@echo off
REM AIForAll Services Management Script for Windows

setlocal EnableDelayedExpansion

set COMPOSE_FILE=docker-compose.yml

:print_header
echo ================================
echo   AIForAll Services Manager
echo ================================
goto :eof

:show_help
call :print_header
echo.
echo Usage: %~nx0 [COMMAND]
echo.
echo Commands:
echo   start     Start all services
echo   stop      Stop all services
echo   restart   Restart all services
echo   logs      Show logs for all services
echo   status    Show status of all services
echo   clean     Stop and remove all containers, networks, and volumes
echo   build     Build all Docker images
echo   health    Check health of all services
echo   reset-db  Truncate all database tables
echo   help      Show this help message
echo.
echo Examples:
echo   %~nx0 start              # Start all services
echo   %~nx0 logs               # Show all logs
goto :eof

:check_requirements
where docker >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Docker is not installed or not in PATH
    exit /b 1
)

where docker-compose >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Docker Compose is not installed or not in PATH
    exit /b 1
)

if not exist "%COMPOSE_FILE%" (
    echo [ERROR] docker-compose.yml not found in current directory
    exit /b 1
)
goto :eof

:start_services
echo [INFO] Starting all services...
docker-compose up -d
if %errorlevel% equ 0 (
    echo [INFO] Services started successfully!
    echo.
    echo [INFO] Service URLs:
    echo   - IAMRusty:   http://localhost:8080 ^(HTTP^), https://localhost:8443 ^(HTTPS^)
    echo   - Telegraph:  http://localhost:8081
    echo   - PostgreSQL: localhost:5432
    echo   - LocalStack: http://localhost:4566
) else (
    echo [ERROR] Failed to start services
    exit /b 1
)
goto :eof

:stop_services
echo [INFO] Stopping all services...
docker-compose down
if %errorlevel% equ 0 (
    echo [INFO] Services stopped successfully!
) else (
    echo [ERROR] Failed to stop services
    exit /b 1
)
goto :eof

:restart_services
echo [INFO] Restarting all services...
docker-compose restart
if %errorlevel% equ 0 (
    echo [INFO] Services restarted successfully!
) else (
    echo [ERROR] Failed to restart services
    exit /b 1
)
goto :eof

:show_logs
docker-compose logs %*
goto :eof

:show_status
echo [INFO] Service status:
docker-compose ps
goto :eof

:clean_all
echo [WARN] This will remove all containers, networks, and volumes!
set /p answer="Are you sure? (y/N) "
if /i "%answer%"=="y" (
    echo [INFO] Cleaning up...
    docker-compose down -v --remove-orphans
    docker system prune -f
    echo [INFO] Cleanup completed!
) else (
    echo [INFO] Cleanup cancelled.
)
goto :eof

:build_images
echo [INFO] Building Docker images...
docker-compose build --no-cache
if %errorlevel% equ 0 (
    echo [INFO] Images built successfully!
) else (
    echo [ERROR] Failed to build images
    exit /b 1
)
goto :eof

:check_health
echo [INFO] Checking service health...
for %%s in (postgres localstack iam-service telegraph-service) do (
    for /f "tokens=*" %%i in ('docker-compose ps -q %%s 2^>nul') do (
        if "%%i" neq "" (
            for /f "tokens=*" %%j in ('docker inspect --format="{{.State.Health.Status}}" %%i 2^>nul') do (
                if "%%j"=="healthy" (
                    echo   %%s: healthy
                ) else if "%%j"=="unhealthy" (
                    echo   %%s: unhealthy
                ) else if "%%j"=="starting" (
                    echo   %%s: starting
                ) else (
                    echo   %%s: no health check
                )
            )
        ) else (
            echo   %%s: not running
        )
    )
)
goto :eof

:reset_database
echo [WARN] This will truncate all database tables!
set /p answer="Are you sure? (y/N) "
if /i "%answer%"=="y" (
    echo [INFO] Resetting database...
    docker-compose --profile tools run --rm truncate-db
    if %errorlevel% equ 0 (
        echo [INFO] Database reset completed!
    ) else (
        echo [ERROR] Failed to reset database
        exit /b 1
    )
) else (
    echo [INFO] Database reset cancelled.
)
goto :eof

:main
call :check_requirements
if %errorlevel% neq 0 exit /b 1

set command=%1
if "%command%"=="" set command=help

if "%command%"=="start" (
    call :start_services
) else if "%command%"=="stop" (
    call :stop_services
) else if "%command%"=="restart" (
    call :restart_services
) else if "%command%"=="logs" (
    shift
    call :show_logs %*
) else if "%command%"=="status" (
    call :show_status
) else if "%command%"=="clean" (
    call :clean_all
) else if "%command%"=="build" (
    call :build_images
) else if "%command%"=="health" (
    call :check_health
) else if "%command%"=="reset-db" (
    call :reset_database
) else if "%command%"=="help" (
    call :show_help
) else if "%command%"=="--help" (
    call :show_help
) else if "%command%"=="-h" (
    call :show_help
) else (
    echo [ERROR] Unknown command: %command%
    echo.
    call :show_help
    exit /b 1
)

goto :eof

call :main %* 