"""Keep test-path findings out of published CodeQL SARIF, not out of analysis."""

import argparse
import json
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit


def is_test_result(result, language):
    # Match the primary reported path, not paths in related locations or flows.
    locations = result.get("locations") or [{}]
    uri = locations[0].get("physicalLocation", {}).get("artifactLocation", {}).get("uri", "")
    *directories, filename = unquote(urlsplit(uri).path).split("/")
    if any(part in {"test", "tests"} for part in directories):
        return True
    if language == "rust":
        return "test" in filename
    if language == "python":
        return filename.endswith(".py") and (
            filename.startswith("test_") or filename.endswith("_test.py")
        )
    return "__tests__" in directories or re.search(
        r"\.(?:test|spec)\.[cm]?[jt]sx?$", filename
    ) is not None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("language", choices=("rust", "python", "javascript-typescript"))
    parser.add_argument("sarif_dir", type=Path)
    args = parser.parse_args()

    reports = sorted(args.sarif_dir.glob("*.sarif"))
    if not reports:
        raise SystemExit("No CodeQL SARIF reports found")
    for report in reports:
        sarif = json.loads(report.read_text(encoding="utf-8"))
        removed = 0
        for run in sarif["runs"]:
            if "results" in run:
                results = run["results"]
                run["results"] = [
                    result for result in results if not is_test_result(result, args.language)
                ]
                removed += len(results) - len(run["results"])
        report.write_text(json.dumps(sarif, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
        print(f"{report.name}: removed {removed} results on test paths")


if __name__ == "__main__":
    main()
