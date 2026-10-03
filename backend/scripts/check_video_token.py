#!/usr/bin/env python3
"""Checks which signed-link format a private Bunny library accepts.

Asks for the CDN token authentication key without echoing it, signs links to
one video's playlist and thumbnail in Bunny's two token formats, and prints only
the HTTP status of each. The key is never printed or stored.

    python3 backend/scripts/check_video_token.py <cdn-hostname> <video-guid>
"""

import base64
import getpass
import hashlib
import hmac
import sys
import time
import urllib.error
import urllib.parse
import urllib.request


def b64url(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).decode().rstrip("=")


def hs256(key: str, signed_path: str, expires: str, params: str) -> str:
    digest = hmac.new(key.encode(), (signed_path + expires + params).encode(), hashlib.sha256)
    return "HS256-" + b64url(digest.digest())


def sha256(key: str, signed_path: str, expires: str, params: str) -> str:
    return b64url(hashlib.sha256((key + signed_path + expires + params).encode()).digest())


def status(url: str) -> str:
    request = urllib.request.Request(url, method="GET", headers={"User-Agent": "token-check"})
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            return str(response.status)
    except urllib.error.HTTPError as err:
        return str(err.code)
    except Exception as err:  # noqa: BLE001
        return f"error: {err}"


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    host = sys.argv[1].removeprefix("https://").rstrip("/")
    guid = sys.argv[2].strip()
    key = getpass.getpass("Token authentication key (hidden): ").strip()
    expires = str(int(time.time()) + 3600)
    directory = f"/{guid}/"
    token_path = urllib.parse.quote(directory, safe="")

    print(f"unsigned playlist: {status(f'https://{host}/{guid}/playlist.m3u8')} (403 expected)")
    for name, sign in [("HS256", hs256), ("SHA256", sha256)]:
        for file in ["playlist.m3u8", "thumbnail.jpg"]:
            path = f"/{guid}/{file}"
            # Directory token in the path, as the app uses it.
            token = sign(key, directory, expires, f"token_path={directory}")
            in_path = f"https://{host}/bcdn_token={token}&token_path={token_path}&expires={expires}{path}"
            # Token for this one file, in the query string.
            token = sign(key, path, expires, "")
            in_query = f"https://{host}{path}?token={token}&expires={expires}"
            print(f"{name:6} {file:14} directory: {status(in_path)}  single file: {status(in_query)}")


if __name__ == "__main__":
    main()
