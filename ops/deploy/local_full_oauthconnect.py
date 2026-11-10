"""Private seed guards and compatibility facade; no resource constructors."""
import re

def seed_inputs(github, simulated):
    """Validate both sources before any runtime operation; never log credentials."""
    try:
        for value in (github, simulated):
            if (set(value) != {"version", "client_id", "client_secret", "hmac_secret"}
                    or type(value["version"]) is not int or value["version"] != 1
                    or not isinstance(value["client_id"], str) or not re.fullmatch(r"[A-Za-z0-9_.-]{1,128}", value["client_id"])
                    or not isinstance(value["client_secret"], str) or not 1 <= len(value["client_secret"]) <= 4096
                    or not value["client_secret"].strip() or any(ord(c) < 32 for c in value["client_secret"])
                    or not isinstance(value["hmac_secret"], str) or not re.fullmatch(r"[a-f0-9]{64}", value["hmac_secret"])):
                raise ValueError
        if (not re.fullmatch(r"[A-Za-z0-9_-]{8,128}", simulated["client_id"])
                or not 16 <= len(simulated["client_secret"]) <= 256
                or len({github["hmac_secret"], simulated["hmac_secret"], github["client_secret"], simulated["client_secret"]}) != 4):
            raise ValueError
    except (KeyError, TypeError, ValueError):
        raise ValueError("invalid or shared OAuthConnect credentials (values redacted)") from None
    return {"version": 1, "github": github, "simulated": simulated}
