"""HTTPS/WebSocket client for the connected POC; no daemon or external library."""
import base64
import hashlib
import json
import os
from pathlib import Path
import socket
import ssl
import struct
import sys
import time
import urllib.parse
import urllib.request


class HermesError(Exception):
    pass


def load_session(path):
    path = Path(path)
    if path.stat().st_mode & 0o077:
        raise HermesError("Configuration de session insuffisamment protégée.")
    config = json.loads(path.read_text())
    if config.get("origin") != "https://onyx-ingenierie.com" or config.get("base_path") != "/arcenal":
        raise HermesError("Destination Hermes non autorisée.")
    return config


def get_json(config, path):
    request = urllib.request.Request(config["origin"] + path, headers={
        "Cookie": config["cookies"], "X-Hermes-Session-Token": config["token"],
    })
    with urllib.request.urlopen(request, timeout=20) as response:
        data = response.read(1_000_001)
        if len(data) > 1_000_000:
            raise HermesError("Réponse Hermes trop volumineuse.")
        return json.loads(data)


def identity(config):
    data = get_json(config, "/yunohost/portalapi/me")
    if data.get("username") != config["uid"]:
        raise HermesError("Identité SSO inattendue ; accès refusé.")
    return {"uid": data["username"], "display_name": data.get("fullname") or data["username"]}


def project_recruitment(data, uid):
    """Keep business fields needed by the views, never backend secrets or file paths."""
    if not data.get("ok") or data.get("user", {}).get("username") != uid:
        raise HermesError("Identité du recrutement inattendue ; accès refusé.")
    def fields(row, allowed):
        return {key: row[key] for key in allowed if key in row}
    settings = data.get("settings") or {}
    training = settings.get("eliseTraining") or {}
    profile_source = training.get("profile") or {}
    profile = fields(profile_source, ("policy_version", "scoring_version", "prompt_version"))
    profile["weights"] = fields(profile_source.get("weights") or {}, ("skills", "mobility", "salary", "experience", "title"))
    return {
        "uid": uid, "source": "onyx_recruitment", "statuses": data.get("statuses", {}),
        "candidates": [fields(row, (
            "id", "created_at", "updated_at", "first_name", "last_name", "email", "phone",
            "position", "location", "experience", "motivation", "status", "source", "notes",
            "in_pool", "archived_at", "tags", "mobility", "salary_expectation", "job_id",
            "cv_original", "cover_letter_original",
        )) for row in data.get("candidates", [])],
        "jobs": [fields(row, (
            "id", "title", "location", "contract", "description", "context", "objective", "missions",
            "skills", "status", "created_at", "updated_at", "salary_min", "salary_max", "dolibarr_ref",
        )) for row in data.get("jobs", [])],
        "activity": [fields(row, ("created_at", "actor", "action", "entity_type", "details")) for row in data.get("activity", [])],
        "settings": {
            "connectors": [fields(row, ("id", "label", "enabled", "mode")) for row in settings.get("connectors", [])],
            "rejectionTemplate": fields(settings.get("rejectionTemplate") or {}, ("subject", "body")),
            "newsletterTemplate": fields(settings.get("newsletterTemplate") or {}, ("subject", "body")),
            "matchThreshold": settings.get("matchThreshold"),
            "eliseTraining": {
                "policyMarkdown": training.get("policyMarkdown", ""),
                "profile": profile,
            },
        },
    }


def recruitment(config):
    # This single, fixed GET endpoint cannot forward publication, deletion or settings writes.
    request = urllib.request.Request(
        "https://oskar.onyx-ingenierie.com/recrutement/api-v2.php?action=bootstrap",
        headers={"Cookie": config["cookies"], "Accept": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=25) as response:
        raw = response.read(8_000_001)
        if len(raw) > 8_000_000:
            raise HermesError("Données du recrutement trop volumineuses.")
        data = json.loads(raw)
    return project_recruitment(data, config["uid"])


class WebSocket:
    def __init__(self, config):
        host = urllib.parse.urlsplit(config["origin"]).hostname
        raw = socket.create_connection((host, 443), timeout=20)
        self.sock = ssl.create_default_context().wrap_socket(raw, server_hostname=host)
        self.file = self.sock.makefile("rb")
        key = base64.b64encode(os.urandom(16)).decode()
        path = config["base_path"] + "/api/ws?" + urllib.parse.urlencode({"token": config["token"]})
        headers = (f"GET {path} HTTP/1.1\r\nHost: {host}\r\nOrigin: {config['origin']}\r\n"
                   f"Cookie: {config['cookies']}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                   f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n")
        self.sock.sendall(headers.encode())
        status = self.file.readline(8193)
        if not status.startswith(b"HTTP/1.1 101 "):
            self.close()
            raise HermesError("Connexion WebSocket Hermes refusée.")
        response_headers = {}
        size = len(status)
        while True:
            line = self.file.readline(8193)
            size += len(line)
            if size > 8192 or not line:
                self.close(); raise HermesError("Réponse WebSocket invalide.")
            if line == b"\r\n":
                break
            name, value = line.decode().split(":", 1)
            response_headers[name.lower()] = value.strip()
        expected = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
        if response_headers.get("sec-websocket-accept") != expected:
            self.close(); raise HermesError("Validation WebSocket impossible.")
        self.sock.settimeout(100)

    def close(self):
        self.file.close()
        self.sock.close()

    def exact(self, count):
        value = self.file.read(count)
        if len(value) != count:
            raise HermesError("Connexion Hermes interrompue.")
        return value

    def send(self, payload, opcode=1):
        data = payload if isinstance(payload, bytes) else json.dumps(payload).encode()
        length = len(data)
        header = bytes([0x80 | opcode])
        if length < 126:
            header += bytes([0x80 | length])
        elif length <= 65535:
            header += bytes([0x80 | 126]) + struct.pack("!H", length)
        else:
            header += bytes([0x80 | 127]) + struct.pack("!Q", length)
        mask = os.urandom(4)
        self.sock.sendall(header + mask + bytes(byte ^ mask[i % 4] for i, byte in enumerate(data)))

    def receive(self, deadline=None):
        chunks = bytearray()
        while True:
            if deadline is not None:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise HermesError("Délai de réponse Hermes dépassé.")
                self.sock.settimeout(remaining)
            first, second = self.exact(2)
            if first & 0x70 or second & 0x80:
                raise HermesError("Trame WebSocket non supportée.")
            opcode = first & 0x0f
            length = second & 0x7f
            if length == 126:
                length = struct.unpack("!H", self.exact(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self.exact(8))[0]
            if length + len(chunks) > 1_000_000:
                raise HermesError("Réponse Hermes trop volumineuse.")
            payload = self.exact(length)
            if opcode == 8:
                raise HermesError("Hermes a fermé la conversation.")
            if opcode == 9:
                self.send(payload, opcode=10); continue
            if opcode == 10:
                continue
            if opcode not in (0, 1):
                raise HermesError("Réponse Hermes non textuelle.")
            chunks.extend(payload)
            if first & 0x80:
                return json.loads(chunks.decode())

    def rpc(self, method, params):
        self.send({"jsonrpc": "2.0", "id": "arcenal-poc", "method": method, "params": params})
        deadline = time.monotonic() + 100
        while time.monotonic() < deadline:
            message = self.receive(deadline)
            if message.get("id") != "arcenal-poc":
                continue
            if "error" in message:
                error = message["error"]
                detail = str(error.get("message", ""))
                if "503" in detail or "high demand" in detail:
                    raise HermesError("Le modèle Gemini est momentanément indisponible (HTTP 503). Réessayez plus tard.")
                if "timed out" in detail or "timeout" in detail.lower():
                    raise HermesError("Le modèle n’a pas répondu dans le délai prévu. Aucun texte simulé n’est utilisé.")
                raise HermesError("Hermes a refusé la génération. Aucun texte simulé n’est utilisé.")
            return message.get("result", {})
        raise HermesError("Délai de réponse Hermes dépassé.")


def operate(config, request):
    user = identity(config)
    if request.get("operation") == "recruitment":
        return recruitment(config)
    if request.get("operation") == "identity":
        return user
    if request.get("operation") == "status":
        model = get_json(config, "/arcenal/api/model/info?profile=elise")
        get_json(config, "/arcenal/api/auth/me")
        return {**user, "connected": True, "profile": "elise", "model": model.get("model"), "provider": model.get("provider")}
    if request.get("operation") != "chat" or request.get("agent") != "elise":
        raise HermesError("Seule Élise est raccordée dans ce premier POC.")
    text = request.get("input", "")
    if not isinstance(text, str) or not text or len(text) > 30000:
        raise HermesError("Message invalide.")
    profiles = get_json(config, "/arcenal/api/profiles").get("profiles", [])
    elise = next((profile for profile in profiles if profile.get("name") == "elise"), None)
    if not elise:
        raise HermesError("Le profil Élise n’est plus accessible.")
    instructions = (elise.get("description") or "Tu es Élise, assistante française de recrutement.")
    instructions += "\n\nCONTEXTE DU POC ARCENAL, fourni par le serveur :\n" + request["instructions"]
    client = WebSocket(config)
    try:
        session = client.rpc("session.create", {
            "profile": "elise", "source": "tool", "close_on_disconnect": True,
            "title": "ARCenal Équipe — POC de conversation",
        })
        time.sleep(4)
        result = client.rpc("llm.oneshot", {
            "profile": "elise", "session_id": session["session_id"],
            "instructions": instructions, "input": text,
            "max_tokens": 1500, "temperature": 0.4,
        })
        reply = result.get("text")
        if not isinstance(reply, str) or not reply.strip():
            raise HermesError("Hermes n’a pas renvoyé de texte.")
        return {"text": reply, "profile": "elise", "uid": user["uid"], "source": "hermes"}
    finally:
        client.close()


if __name__ == "__main__":
    try:
        config = load_session(sys.argv[1])
        request = json.loads(sys.stdin.read(40001))
        result = operate(config, request)
        print(json.dumps({"result": result}, ensure_ascii=False))
    except HermesError as error:
        message = str(error)
        if "config" in locals():
            for secret in (config.get("token", ""), config.get("cookies", "")):
                if secret:
                    message = message.replace(secret, "[masqué]")
        print(json.dumps({"error": message}, ensure_ascii=False))
        sys.exit(1)
    except Exception:
        print(json.dumps({"error": "Connexion SSO/Hermes indisponible ; aucun mode simulé utilisé."}, ensure_ascii=False))
        sys.exit(1)
