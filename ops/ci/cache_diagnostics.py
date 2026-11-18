"""Summarize sccache backend errors without publishing raw logs or credentials."""

import argparse
from collections import Counter
import json
from pathlib import Path
import re


MARKERS = {
    "Cache read error:": "lookup",
    "Error executing cache write:": "write",
}
KINDS = (
    "Unexpected", "Unsupported", "ConfigInvalid", "NotFound", "PermissionDenied",
    "IsADirectory", "NotADirectory", "AlreadyExists", "RateLimited", "IsSameFile",
    "ConditionNotMatch", "RangeNotSatisfied",
)
OPERATIONS = (
    "CreateCacheEntry", "FinalizeCacheEntryUpload", "GetCacheEntryDownloadURL",
    "GetCacheEntryDownloadUrl", "read", "write", "stat",
)
SIGNALS = {
    "rate_limit": r"too many requests|rate.?limit",
    "reservation_conflict": r"already exists|already being created|unable to reserve|reserve.*conflict",
    "unauthorized": r"unauthoriz|forbidden|permission denied",
    "timeout": r"timed? ?out|timeout|deadline exceeded",
    "connection_reset": r"connection reset|broken pipe|connection closed",
    "dns": r"dns error|failed to lookup address|name or service not known",
    "tls": r"certificate|tls handshake",
    "retry_exhausted": r"retries exhausted|retry.*exhausted|max retries",
}
LOG_RECORD = re.compile(r"^\[?\d{4}-\d\d-\d\d[T ]\d\d:\d\d:\d\d")
HTTP_STATUS = re.compile(r"\b(?:status(?:_code)?|http code)\s*[:=]\s*(?:Some\()?([1-5]\d\d)\b", re.I)


def summarize(lines):
    """Only emit fixed labels and numeric counters; never redact-and-echo input."""
    counts = Counter()
    events = 0
    truncated = 0
    stage, chunks, size = None, [], 0

    def finish():
        nonlocal events
        if stage is None:
            return
        events += 1
        text = "".join(chunks)
        counts[f"{stage}.events"] += 1
        for kind in KINDS:
            if re.search(r"\b" + kind + r"\b", text):
                counts[f"{stage}.kind.{kind}"] += 1
        for operation in OPERATIONS:
            if re.search(r"\b" + operation + r"\b", text):
                counts[f"{stage}.operation.{operation}"] += 1
        for status in set(HTTP_STATUS.findall(text)):
            counts[f"{stage}.http.{status}"] += 1
        for signal, pattern in SIGNALS.items():
            if re.search(pattern, text, re.I):
                counts[f"{stage}.signal.{signal}"] += 1

    for line in lines:
        marker = next((key for key in MARKERS if key in line), None)
        if marker:
            finish()
            stage, chunks, size = MARKERS[marker], [], 0
        elif LOG_RECORD.match(line):
            finish()
            stage, chunks, size = None, [], 0
        if stage is not None:
            # Bound one malformed/multiline event, without bounding the job count.
            if size < 65536:
                remaining = 65536 - size
                chunks.append(line[:remaining])
                size += len(chunks[-1])
                if len(line) > remaining:
                    truncated += 1
                    size = 65537
            elif size == 65536:
                truncated += 1
                size += 1
    finish()
    return {
        "schema": "aiforall.sccache-diagnostics/v1",
        "events": events,
        "truncated_events": truncated,
        "counts": dict(sorted(counts.items())),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    present = args.log.is_file()
    if present:
        with args.log.open(encoding="utf-8", errors="replace") as stream:
            summary = summarize(stream)
    else:
        summary = summarize(())
    summary["log_present"] = present
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
