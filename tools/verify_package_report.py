"""Require actual package-check reports; a green help-only invocation is not evidence."""
import json
from pathlib import Path
import sys


def failed(value):
    if isinstance(value, dict):
        return any(item == "failure" or failed(item) for item in value.values())
    if isinstance(value, list):
        return any(failed(item) for item in value)
    return False


reports = list(Path(sys.argv[1]).glob("results_*.json"))
if not reports:
    raise SystemExit("NON_EXÉCUTÉ : aucun rapport package_check produit.")
for report in reports:
    text = report.read_text().strip()
    if not text:
        raise SystemExit("NON_EXÉCUTÉ : rapport package_check vide.")
    data = json.loads(text)
    if failed(data):
        raise SystemExit("ÉCHEC : le rapport package_check contient des scénarios en échec.")
print("Rapports package_check présents et sans scénario marqué en échec.")
