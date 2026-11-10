"""Compatibility entrypoint; the single provider implementation is chart-owned."""
from pathlib import Path

_source = Path(__file__).resolve().parents[3] / "ops/deploy/charts/local-full/files/public/6f72a70bd830/oauth-idp-simulator.py"
__file__ = str(_source)
exec(compile(_source.read_text(encoding="utf-8"), str(_source), "exec"), globals())
