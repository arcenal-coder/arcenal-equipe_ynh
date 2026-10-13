import importlib.util
import io
import json
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("hermes_bridge", ROOT / "lib/hermes_bridge.py")
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)


class HermesBridgeTests(unittest.TestCase):
    def test_recruitment_projection_keeps_business_fields_not_backend_secrets(self):
        source = {
            "ok": True, "user": {"username": "alice"}, "statuses": {"new": "Nouveau"},
            "candidates": [{"id": 1, "first_name": "Alice", "in_pool": 1, "status": "new", "cv_stored": "/private/cv.pdf", "elise_scan": {"internal": "private"}}],
            "jobs": [{"id": 2, "title": "Offre", "status": "draft", "private_key": "secret"}],
            "settings": {"connectors": [{"id": "apec", "label": "APEC", "enabled": True, "mode": "api", "secret_ref": "/private/key", "endpoint": "https://endpoint.test/?token=private"}], "secret": "hidden"},
        }
        result = bridge.project_recruitment(source, "alice")
        self.assertEqual(result["candidates"][0]["in_pool"], 1)
        self.assertEqual(result["jobs"][0]["status"], "draft")
        self.assertEqual(result["settings"]["connectors"][0]["label"], "APEC")
        encoded = json.dumps(result)
        for value in ("cv_stored", "elise_scan", "secret_ref", "private_key", "endpoint.test", "/private/"):
            self.assertNotIn(value, encoded)

    def test_recruitment_source_must_confirm_the_same_user(self):
        for data in ({"ok": True, "user": {"username": "other"}}, {"ok": False, "user": {"username": "alice"}}, {"ok": True}):
            with self.assertRaises(bridge.HermesError):
                bridge.project_recruitment(data, "alice")

    def test_recruitment_bridge_only_performs_fixed_get_without_hermes_token(self):
        response = io.BytesIO(json.dumps({"ok": True, "user": {"username": "alice"}}).encode())
        with patch.object(bridge.urllib.request, "urlopen", return_value=response) as request:
            result = bridge.recruitment({"uid": "alice", "cookies": "session=fixture", "token": "hermes-fixture"})
        call = request.call_args.args[0]
        self.assertEqual(call.get_method(), "GET")
        self.assertEqual(call.full_url, "https://oskar.onyx-ingenierie.com/recrutement/api-v2.php?action=bootstrap")
        self.assertIsNone(call.get_header("X-hermes-session-token"))
        self.assertEqual(result["uid"], "alice")

    def test_identity_mismatch_fails_before_generation(self):
        with patch.object(bridge, "get_json", return_value={"username": "another-user"}), patch.object(bridge, "WebSocket") as socket:
            with self.assertRaises(bridge.HermesError):
                bridge.operate({"uid": "alice"}, {"operation": "chat", "agent": "elise"})
            socket.assert_not_called()

    def test_only_elise_and_bounded_input(self):
        with patch.object(bridge, "identity", return_value={"uid": "alice"}), patch.object(bridge, "WebSocket") as socket:
            for request in ({"operation": "chat", "agent": "andrea"}, {"operation": "chat", "agent": "elise", "input": "x" * 30001}):
                with self.assertRaises(bridge.HermesError):
                    bridge.operate({}, request)
            socket.assert_not_called()

    def test_no_simulated_reply_when_provider_fails(self):
        client = Mock()
        client.rpc.side_effect = [{"session_id": "poc-only"}, bridge.HermesError("Gemini HTTP 503")]
        with patch.object(bridge, "identity", return_value={"uid": "alice"}), patch.object(bridge, "get_json", return_value={"profiles": [{"name": "elise", "description": "Profil existant"}]}), patch.object(bridge, "WebSocket", return_value=client), patch.object(bridge.time, "sleep"):
            with self.assertRaises(bridge.HermesError):
                bridge.operate({}, {"operation": "chat", "agent": "elise", "input": "test", "instructions": "test"})
        client.close.assert_called_once()

    def test_reply_uses_profile_runtime_and_verified_author(self):
        client = Mock()
        client.rpc.side_effect = [{"session_id": "poc-only"}, {"text": "Bonjour Alice"}]
        with patch.object(bridge, "identity", return_value={"uid": "alice"}), patch.object(bridge, "get_json", return_value={"profiles": [{"name": "elise", "description": "Profil existant"}]}), patch.object(bridge, "WebSocket", return_value=client), patch.object(bridge.time, "sleep"):
            result = bridge.operate({}, {"operation": "chat", "agent": "elise", "input": "test", "instructions": "test"})
        self.assertEqual(result["uid"], "alice")
        self.assertEqual(result["source"], "hermes")
        self.assertEqual(client.rpc.call_args.args[1]["session_id"], "poc-only")
        self.assertIn("Profil existant", client.rpc.call_args.args[1]["instructions"])

    def test_unprotected_session_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            file = Path(folder) / "session.json"
            file.write_text("{}")
            file.chmod(0o644)
            with self.assertRaises(bridge.HermesError):
                bridge.load_session(file)

    def test_websocket_fragments_and_ping(self):
        client = bridge.WebSocket.__new__(bridge.WebSocket)
        # Text fragment, interleaved ping, then continuation.
        client.file = io.BytesIO(b'\x01\x06{"ok":' + b'\x89\x01p' + b'\x80\x05true}')
        client.send = Mock()
        self.assertEqual(client.receive(), {"ok": True})
        client.send.assert_called_once_with(b"p", opcode=10)

    def test_websocket_size_and_reserved_bits_refused(self):
        for data in (b'\xc1\x00', b'\x81\x7f' + struct.pack("!Q", 1_000_001)):
            client = bridge.WebSocket.__new__(bridge.WebSocket)
            client.file = io.BytesIO(data)
            with self.assertRaises(bridge.HermesError):
                client.receive()


if __name__ == "__main__":
    unittest.main()
