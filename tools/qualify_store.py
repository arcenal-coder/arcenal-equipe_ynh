"""Qualify technical package checks against the actual ARCenal distribution catalog.

The upstream official-catalog result is retained verbatim. Its two catalog-only
findings are substituted only by direct checks of the published ARCenal entry.
No other critical/error and no failing runtime scenario can be ignored.
The internal rating is capped at 5; no long-term/official quality is claimed.
"""
import argparse
import json
from pathlib import Path


REQUIRED = {("TEST_INSTALL", "root"), ("TEST_INSTALL", "subdir"),
            ("TEST_UPGRADE", ""), ("TEST_BACKUP_RESTORE", ""),
            ("TEST_CHANGE_URL", ""), ("TEST_PACKAGE_LINTER", "")}


def qualify(report, linter, catalog):
    app = catalog.get("apps", {}).get(report.get("app"))
    if not app or app.get("state") != "working" or not app.get("category"):
        raise ValueError("Application absente ou non déclarée fonctionnelle dans le catalogue cible.")
    if app.get("git", {}).get("revision") != report.get("commit"):
        raise ValueError("La révision du catalogue ne correspond pas au paquet vérifié.")
    if app.get("manifest", {}).get("version") != report.get("app_version"):
        raise ValueError("La version du catalogue ne correspond pas au paquet vérifié.")
    tests = report.get("tests", [])
    if not REQUIRED.issubset({(t.get("test_type"), t.get("test_arg", "")) for t in tests}):
        raise ValueError("Scénario obligatoire non exécuté.")
    if any(t.get("main_result") != "success" for t in tests):
        raise ValueError("Scénario de paquet en échec.")
    danger_notes = ("witness", "traversal", "Unsafe install", "overwritten", "manually modified")
    if any(any(word.lower() in note.lower() for word in danger_notes)
           for test in tests for note in test.get("test_notes", [])):
        raise ValueError("Contrôle transversal de sécurité ou de conservation en échec.")
    if not all(key in linter and isinstance(linter[key], list) for key in ("success", "critical", "error", "warning")):
        raise ValueError("Rapport du linter incomplet.")
    contextual = {"AppCatalog.is_in_catalog", "AppCatalog.state_is_working"}
    findings = set(linter["critical"]) | set(linter["error"])
    if findings - contextual:
        raise ValueError("Erreur technique du paquet non corrigée : " + ", ".join(sorted(findings - contextual)))
    return {"scope": "ARCenal Store, catalogue preview administré", "app": report["app"],
            "version": report["app_version"], "revision": report["commit"],
            "technical_level": 5, "upstream_official_level": report["level"],
            "official_catalog_findings": sorted(findings & contextual),
            "runtime_checks": "all_required_passed", "warnings": linter["warning"],
            "official_certification": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("report", "linter", "catalog", "output"):
        parser.add_argument("--" + name, required=True, type=Path)
    args = parser.parse_args()
    try:
        result = qualify(json.loads(args.report.read_text()), json.loads(args.linter.read_text()),
                         json.loads(args.catalog.read_text()))
    except (ValueError, KeyError, TypeError) as error:
        raise SystemExit("Qualification refusée : " + str(error))
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print("Qualification technique ARCenal : niveau 5, aucun échec technique masqué.")
