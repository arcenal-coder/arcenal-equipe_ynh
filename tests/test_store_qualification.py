import importlib.util
from pathlib import Path
import unittest
import copy

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("qualification", ROOT / "tools/qualify_store.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.report = {"app": "fixture", "app_version": "1~ynh1", "commit": "a" * 40, "level": 0,
                       "tests": [{"test_type": kind, "test_arg": arg, "main_result": "success", "test_notes": []}
                                 for kind, arg in module.REQUIRED]}
        self.linter = {"critical": ["AppCatalog.is_in_catalog"], "error": ["AppCatalog.state_is_working"], "warning": [], "success": []}
        self.catalog = {"apps": {"fixture": {"state": "working", "category": "system_tools", "git": {"revision": "a" * 40}, "manifest": {"version": "1~ynh1"}}}}

    def test_uses_actual_target_and_preserves_official_result(self):
        value = module.qualify(self.report, self.linter, self.catalog)
        self.assertEqual(value["technical_level"], 5)
        self.assertEqual(value["upstream_official_level"], 0)
        self.assertFalse(value["official_certification"])

    def test_never_ignores_a_technical_error_or_failed_scenario(self):
        bad = copy.deepcopy(self.linter); bad["critical"].append("Script.unsafe_command")
        with self.assertRaises(ValueError): module.qualify(self.report, bad, self.catalog)
        bad = copy.deepcopy(self.report); bad["tests"][0]["main_result"] = "failure"
        with self.assertRaises(ValueError): module.qualify(bad, self.linter, self.catalog)

    def test_never_accepts_missing_checks_or_wrong_revision(self):
        bad = copy.deepcopy(self.report); bad["tests"].pop()
        with self.assertRaises(ValueError): module.qualify(bad, self.linter, self.catalog)
        bad = copy.deepcopy(self.catalog); bad["apps"]["fixture"]["git"]["revision"] = "b" * 40
        with self.assertRaises(ValueError): module.qualify(self.report, self.linter, bad)
