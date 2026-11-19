"""Run bounded OpenAPI checks against the native IAM integration-test listener."""

import re
import sys
from pathlib import Path
from urllib.parse import urlsplit

USERNAME = "/api/auth/username/check"
RESET_VALIDATE = "/api/auth/password/reset-validate"


def fixture_origin(value):
    if not re.fullmatch(r"http://127\.0\.0\.1:[1-9][0-9]{0,4}/iam", value):
        raise ValueError("expected the IAM fixture's HTTP loopback URL")
    if not urlsplit(value).port:
        raise ValueError("expected a valid fixture port")
    return value


def require_inline(value):
    if isinstance(value, dict):
        # A future contract change must not introduce additional network access.
        if value.keys() & {"$ref", "$dynamicRef", "servers", "callbacks", "links"}:
            raise ValueError("the fixture contract may only contain inline definitions")
        for child in value.values():
            require_inline(child)
    elif isinstance(value, list):
        for child in value:
            require_inline(child)


def run(base_url):
    import requests
    import schemathesis
    import yaml
    from schemathesis.checks import (
        content_type_conformance,
        not_a_server_error,
        response_schema_conformance,
        status_code_conformance,
    )

    base_url = fixture_origin(base_url)
    spec = Path(__file__).resolve().parents[3] / "services/IAMRusty/openspecs.yaml"
    source = yaml.safe_load(spec.read_text(encoding="utf-8"))
    paths = {
        USERNAME: {"get": source["paths"][USERNAME]["get"]},
        RESET_VALIDATE: {"post": source["paths"][RESET_VALIDATE]["post"]},
    }
    require_inline(paths)
    schema = schemathesis.openapi.from_dict(
        {"openapi": source["openapi"], "info": source["info"], "paths": paths}
    )
    username = schema[USERNAME]["GET"]
    reset = schema[RESET_VALIDATE]["POST"]
    cases = (
        ("username minimum", username.Case(query={"username": "abc"}), 200),
        ("username maximum", username.Case(query={"username": "a" * 50}), 200),
        ("username missing", username.Case(), 400),
        ("username too short", username.Case(query={"username": "ab"}), 422),
        ("username too long", username.Case(query={"username": "a" * 51}), 422),
        ("username requires a letter", username.Case(query={"username": "123"}), 422),
        ("unknown reset token", reset.Case(body={"token": "fixture-unknown"}, media_type="application/json"), 400),
        ("reset token maximum", reset.Case(body={"token": "a" * 100}, media_type="application/json"), 400),
        ("reset token too long", reset.Case(body={"token": "a" * 101}, media_type="application/json"), 422),
        ("reset token missing", reset.Case(body={}, media_type="application/json"), 422),
        ("reset token wrong type", reset.Case(body={"token": 123}, media_type="application/json"), 422),
    )
    with requests.Session() as session:
        session.trust_env = False
        for label, case, expected in cases:
            response = case.call(
                base_url=base_url, session=session, allow_redirects=False, timeout=2
            )
            if response.status_code != expected:
                raise AssertionError(f"{label}: expected {expected}, got {response.status_code}")
            case.validate_response(
                response,
                checks=[
                    not_a_server_error,
                    status_code_conformance,
                    content_type_conformance,
                    response_schema_conformance,
                ],
            )
    print(f"IAM OpenAPI: {len(cases)} bounded contract cases passed")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("expected a URL from the native IAM fixture")
    run(fixture_origin(sys.argv[1]))
