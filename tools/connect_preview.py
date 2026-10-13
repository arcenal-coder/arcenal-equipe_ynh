"""Prepare a private, temporary SSO session for the connected local POC."""
import argparse
import getpass
import http.cookiejar
import json
import os
from pathlib import Path
import re
import tempfile
import urllib.request


def prepare(runtime_parent):
    origin = "https://onyx-ingenierie.com"
    username = os.environ.pop("ARCENAL_LOGIN_USER", None) or input("Identifiant YunoHost : ").strip()
    password = os.environ.pop("ARCENAL_LOGIN_PASSWORD", None) or getpass.getpass("Mot de passe YunoHost : ")
    jar = http.cookiejar.CookieJar()
    opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
    request = urllib.request.Request(
        origin + "/yunohost/portalapi/login",
        data=json.dumps({"credentials": username + ":" + password}).encode(),
        headers={"Content-Type": "application/json", "Origin": origin}, method="POST",
    )
    opener.open(request, timeout=30).read()
    del password, request
    identity = json.load(opener.open(origin + "/yunohost/portalapi/me", timeout=30))
    if identity.get("username") != username:
        raise ValueError("Unexpected SSO identity")
    page = opener.open(origin + "/arcenal/", timeout=30).read().decode()
    match = re.search(r'__HERMES_SESSION_TOKEN__\s*=\s*("(?:\\.|[^"\\])*")', page)
    if not match:
        raise ValueError("Hermes session bootstrap unavailable")
    token = json.loads(match.group(1))
    cookies = "; ".join(cookie.name + "=" + cookie.value for cookie in jar)
    headers = {"X-Hermes-Session-Token": token, "Cookie": cookies}
    auth = json.load(opener.open(urllib.request.Request(origin + "/arcenal/api/auth/me", headers=headers), timeout=30))
    profiles = json.load(opener.open(urllib.request.Request(origin + "/arcenal/api/profiles", headers=headers), timeout=30))
    if "elise" not in [profile.get("name") for profile in profiles.get("profiles", [])]:
        raise ValueError("Existing Élise profile unavailable")
    runtime = Path(tempfile.mkdtemp(prefix="arcenal-connected-", dir=runtime_parent))
    config = {
        "origin": origin, "base_path": "/arcenal", "token": token, "cookies": cookies,
        "uid": identity["username"], "display_name": identity.get("fullname") or identity["username"],
        "hermes_user_id": auth.get("user_id"), "profiles": [p["name"] for p in profiles["profiles"]],
    }
    session = runtime / "session.json"
    descriptor = os.open(session, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w") as file:
        json.dump(config, file)
    print("Session SSO vérifiée ; profil Élise existant confirmé.")
    print("Configuration protégée :", session)
    return session


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime-parent", required=True, type=Path)
    args = parser.parse_args()
    try:
        prepare(args.runtime_parent)
    except Exception:
        raise SystemExit("Préparation SSO/Hermes impossible ; aucun secret affiché.")
