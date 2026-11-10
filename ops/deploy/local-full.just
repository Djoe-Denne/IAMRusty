# Sequential phase graph. No services, credentials, specs, or ambient kubeconfig.
set shell := ["powershell.exe", "-NoProfile", "-NonInteractive", "-Command"]
set dotenv-load := false

plan:
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase plan none

target: plan
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase target plan

quiesce: target
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase quiesce target

foundation: quiesce
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase foundation quiesce

dependencies: foundation
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase dependencies foundation

bootstrap: dependencies
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase bootstrap dependencies

applications-first: bootstrap
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase applications-first bootstrap

final: applications-first
    @& $env:AIFORALL_LOCAL_FULL_PYTHON -B ops/deploy/deploy-local-full.py _phase final applications-first

run: final
    @Write-Output 'local-full declared phases completed; behavioral qualification remains separate'
