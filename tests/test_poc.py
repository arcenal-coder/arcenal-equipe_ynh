import http.client
import importlib.util
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import types
import unittest
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("identity", ROOT / "lib/identity.py")
identity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(identity)


class DirectoryNameTests(unittest.TestCase):
    def test_priority_and_fallbacks(self):
        for attrs, expected in [
            ({"givenName": ["Émilie".encode()], "displayName": [b"Other"]}, "Émilie"),
            ({"givenName": [b" "], "displayName": [b"Alice Dupont"]}, "Alice Dupont"),
            ({"cn": [b"Alice"]}, "Alice"), ({}, "alice.ldap"),
        ]:
            self.assertEqual(identity.first_name(attrs, "alice.ldap"), expected)

    def test_invalid_attributes_and_uid(self):
        with self.assertRaises(ValueError):
            identity.first_name({"givenName": [b"Alice\nadmin"]}, "alice")
        with self.assertRaises(ValueError):
            identity.resolve("admin)(uid=*)")

    def test_directory_lookup_and_failure_are_not_header_fallbacks(self):
        connection = Mock()
        connection.search_s.return_value = [("uid=alice,ou=users,dc=yunohost,dc=org", {"givenName": [b"Alice"]})]
        ldap = types.ModuleType("ldap")
        ldap.initialize = Mock(return_value=connection)
        ldap.OPT_NETWORK_TIMEOUT = 1
        ldap.OPT_TIMEOUT = 2
        ldap.SCOPE_SUBTREE = 2
        ldap_filter = types.ModuleType("ldap.filter")
        ldap_filter.escape_filter_chars = lambda value: value
        with patch.dict("sys.modules", {"ldap": ldap, "ldap.filter": ldap_filter}):
            self.assertEqual(identity.resolve("alice"), "Alice")
            ldap.initialize.assert_called_once_with("ldap://127.0.0.1:389")
            self.assertIn("(uid=alice)", connection.search_s.call_args.args[2])
            connection.unbind_s.assert_called_once()
            connection.search_s.return_value = []
            with self.assertRaises(ValueError):
                identity.resolve("alice")
            connection.search_s.side_effect = RuntimeError("LDAP offline")
            with self.assertRaises(RuntimeError):
                identity.resolve("alice")
            self.assertEqual(connection.unbind_s.call_count, 3)


class PackageTests(unittest.TestCase):
    def test_rejects_macos_binary_without_creating_package(self):
        binary = ROOT / "target/debug/arcenal-equipe"
        if binary.read_bytes()[:4] == b"\x7fELF":
            self.skipTest("Linux executable; this refusal specifically tests macOS")
        with tempfile.TemporaryDirectory(prefix="arcenal-package-") as folder:
            output = Path(folder) / "poc.tar.gz"
            result = subprocess.run(["python3", "-B", str(ROOT / "tools/package.py"), "--binary", str(binary), "--output", str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn("Linux ELF", result.stderr)
            self.assertFalse(output.exists())


class DemoHTTPTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Avoid accidentally exercising another process already on the demo port.
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 8080))
        cls.process = subprocess.Popen([str(ROOT / "target/debug/arcenal-equipe"), "--demo"], cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if cls.process.poll() is not None:
                raise RuntimeError("POC exited before readiness")
            try:
                with socket.create_connection(("127.0.0.1", 8080), timeout=.2):
                    return
            except OSError:
                time.sleep(.05)
        cls.process.terminate()
        cls.process.wait(timeout=5)
        raise RuntimeError("POC did not become ready")

    @classmethod
    def tearDownClass(cls):
        cls.process.terminate()
        cls.process.wait(timeout=5)

    def request(self, method, path, body=None, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", 8080, timeout=5)
        try:
            connection.request(method, path, body=body, headers=headers or {})
            response = connection.getresponse()
            raw = response.read()
            body = raw if response.getheader("Content-Type") == "image/png" else raw.decode()
            return response.status, dict(response.getheaders()), body
        finally:
            connection.close()

    def test_home_pages_assets_profiles_and_headers(self):
        for path in ("/", "/agents/elise", "/agents/victoire", "/agents/andrea", "/profiles/elise.yaml", "/profiles/victoire.yaml", "/profiles/andrea.yaml", "/static/theme.css", "/static/app.js", "/static/elise.png"):
            status, headers, body = self.request("GET", path)
            self.assertEqual(status, 200, path)
            self.assertEqual(headers["Cache-Control"], "no-store")
            self.assertIn("frame-ancestors 'none'", headers["Content-Security-Policy"])
            self.assertTrue(body)
            if path == "/static/elise.png":
                self.assertTrue(body.startswith(b"\x89PNG\r\n\x1a\n"))
        status, _, body = self.request("GET", "/", headers={"YNH_USER": "admin", "YNH_USER_FULLNAME": "Admin"})
        self.assertEqual(status, 200)
        self.assertIn("Bonjour Camille.", body)
        self.assertNotIn("Bonjour Admin", body)

    def test_mcp_protocol_and_agent_isolation(self):
        for agent in ("elise", "victoire", "andrea"):
            for method, params in [("initialize", {"protocolVersion": "2025-03-26"}), ("tools/list", {}), ("tools/call", {"name": f"{agent}_demo_identity", "arguments": {}})]:
                status, _, body = self.request("POST", f"/mcp/{agent}", json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}), {"Content-Type": "application/json"})
                self.assertEqual(status, 200)
                data = json.loads(body)
                self.assertIn("result", data)
                if method == "tools/list":
                    self.assertEqual([t["name"] for t in data["result"]["tools"]], [f"{agent}_demo_identity"])
                if method == "tools/call":
                    self.assertIn("demo.local", body)
            status, _, body = self.request("POST", f"/mcp/{agent}", json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "unknown_tool"}}), {"Content-Type": "application/json"})
            self.assertEqual(json.loads(body)["error"]["code"], -32602)
            status, _, body = self.request("POST", f"/mcp/{agent}", json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}), {"Content-Type": "application/json"})
            self.assertEqual(status, 202)
            self.assertEqual(body, "")

    def test_refusals(self):
        self.assertEqual(self.request("POST", "/")[0], 405)
        self.assertEqual(self.request("GET", "/mcp/elise")[0], 405)
        self.assertEqual(self.request("GET", "/profiles/../../etc/passwd.yaml")[0], 404)
        self.assertEqual(self.request("POST", "/mcp/elise", "{}", {"Content-Type": "application/json", "Origin": "https://evil.test"})[0], 403)
        self.assertEqual(self.request("POST", "/mcp/elise", "x" * 16385, {"Content-Type": "application/json"})[0], 413)


class PrivateSocketTests(unittest.TestCase):
    def test_no_identity_and_invalid_identity_are_denied(self):
        with tempfile.TemporaryDirectory(prefix="arcenal-poc-") as folder:
            path = str(Path(folder) / "app.sock")
            env = dict(os.environ, ARCENAL_SOCKET=path, ARCENAL_BASE_PATH="/equipe")
            process = subprocess.Popen([str(ROOT / "target/debug/arcenal-equipe")], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                for _ in range(60):
                    if Path(path).exists():
                        break
                    if process.poll() is not None:
                        raise RuntimeError("Private POC exited")
                    time.sleep(.05)
                self.assertEqual(Path(path).stat().st_mode & 0o777, 0o660)
                for header in ("", "YNH_USER: admin)(uid=*)\r\n", "Remote-User: admin\r\n"):
                    with socket.socket(socket.AF_UNIX) as connection:
                        connection.settimeout(5)
                        connection.connect(path)
                        connection.sendall(f"GET /equipe/ HTTP/1.1\r\nHost: localhost\r\n{header}\r\n".encode())
                        data = connection.recv(4096).decode()
                        self.assertIn("HTTP/1.1 401", data)
            finally:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
