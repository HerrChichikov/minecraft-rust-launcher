"""Assert that the actual release compiler rejects the development-auth feature."""

from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
EXPECTED = 'LAUNCHER_DEV_AUTH_FORBIDDEN'

for package in ('launcher-core', 'launcher-desktop'):
    result = subprocess.run(
        ['cargo', 'check', '--locked', '--release', '-p', package, '--features', 'dev-auth'],
        cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    if result.returncode == 0 or EXPECTED not in result.stdout:
        print(result.stdout)
        sys.exit(f'{package}: release guard failed or failed for an unrelated reason')
    print(f'{package}: expected release guard rejection confirmed')
