"""Check the manual smoke output without relying on shell quoting or locale."""
import json
import sys
from pathlib import Path
text = Path(sys.argv[1]).read_text(encoding="utf-8")
snapshot = json.loads(text[text.index('{'):])
assert snapshot["schemaVersion"] == 1
assert snapshot["downloadedBytes"] == 0, snapshot["downloadedBytes"]
assert snapshot["reusedBytes"] > 0
assert snapshot["javaExecutable"]
print("Official installation reused with zero artifact download bytes; Java probe passed")
