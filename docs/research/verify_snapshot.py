"""Offline checks of Phase 0 evidence. This is not launcher implementation."""

import hashlib
import json
from pathlib import Path, PurePosixPath
import posixpath
import re


ROOT = Path(__file__).resolve().parent
checks = []


def check(condition, description):
    if not condition:
        raise AssertionError(description)
    checks.append(description)


def load(name):
    return json.loads((ROOT / name).read_bytes())


def verify_descriptor(name, descriptor):
    content = (ROOT / name).read_bytes()
    check(hashlib.sha1(content).hexdigest() == descriptor['sha1'],
          f'{name}: parent SHA1')
    if 'size' in descriptor:
        check(len(content) == descriptor['size'], f'{name}: parent size')


def safe_relative(path):
    p = PurePosixPath(path)
    return not p.is_absolute() and '..' not in p.parts and '\\' not in path


def main():
    provenance = load('provenance.json')
    for record in provenance['records']:
        check(safe_relative(record['file']), 'safe snapshot filename')
        content = (ROOT / record['file']).read_bytes()
        check(len(content) == record['bytes'], f"{record['file']}: recorded size")
        for algorithm in ('sha1', 'sha256'):
            check(hashlib.new(algorithm, content).hexdigest() == record[algorithm],
                  f"{record['file']}: recorded {algorithm}")
        check(record['url'].startswith('https://'), 'HTTPS source')

    entry = next(x for x in load('version_manifest_v2.json')['versions']
                 if x['id'] == '1.21.1')
    verify_descriptor('1.21.1.json', entry)
    version = load('1.21.1.json')
    check(version['javaVersion'] == {'component': 'java-runtime-delta',
                                    'majorVersion': 21}, 'metadata Java 21 delta')
    check(version['mainClass'] == 'net.minecraft.client.main.Main', 'main class')
    check(len(version['libraries']) == 97, '97 library records in this revision')
    check(all('natives' not in lib for lib in version['libraries']),
          'no legacy natives fields in this revision')
    for lib in version['libraries']:
        artifact = lib['downloads']['artifact']
        check(safe_relative(artifact['path']), 'relative official library path')
        check(bool(re.fullmatch('[0-9a-f]{40}', artifact['sha1'])), 'library SHA1 shape')

    placeholders = sorted(set(re.findall(r'\$\{([^}]+)\}',
                                         json.dumps(version['arguments']))))
    check(len(placeholders) == 21, '21 argument placeholders in this revision')
    check({'clientid', 'auth_xuid'}.issubset(placeholders), 'additional identity placeholders')
    verify_descriptor('asset-index-17.json', version['assetIndex'])
    objects = load('asset-index-17.json')['objects']
    check(len(objects) == 3911, '3911 logical assets')
    hashes = {obj['hash'] for obj in objects.values()}
    check(len(hashes) == 3888, '3888 unique asset hashes')
    check(all(re.fullmatch('[0-9a-f]{40}', h) for h in hashes), 'asset SHA1 shapes')

    catalog = load('java-runtime-all.json')
    platforms = {}
    for platform in ('windows-x64', 'linux', 'mac-os', 'mac-os-arm64'):
        candidates = catalog[platform][version['javaVersion']['component']]
        check(len(candidates) == 1, f'{platform}: one observed runtime candidate')
        candidate = candidates[0]
        check(candidate['availability']['progress'] == 100, f'{platform}: fully available')
        verify_descriptor(f'runtime-{platform}.json', candidate['manifest'])
        files = load(f'runtime-{platform}.json')['files']
        executable = ('bin/java.exe' if platform == 'windows-x64' else
                      'jre.bundle/Contents/Home/bin/java' if platform.startswith('mac-os')
                      else 'bin/java')
        check(files[executable]['type'] == 'file', f'{platform}: Java executable entry')
        if platform != 'windows-x64':
            check(files[executable]['executable'], f'{platform}: executable bit')
        for name, info in files.items():
            check(safe_relative(name), f'{platform}: relative runtime path')
            check(info['type'] in ('file', 'link', 'directory'), 'known runtime entry kind')
            if info['type'] == 'file':
                check('raw' in info['downloads'], 'raw runtime download available')
            elif info['type'] == 'link':
                target = posixpath.normpath(posixpath.join(posixpath.dirname(name), info['target']))
                check(safe_relative(target) and target in files,
                      f'{platform}: internal declared link target')
        platforms[platform] = {'version': candidate['version']['name'],
                               'manifestSha1': candidate['manifest']['sha1'],
                               'entries': len(files), 'executable': executable}

    agent = load('authlib-injector-latest.json')
    check(bool(re.fullmatch('[0-9a-f]{64}', agent['checksums']['sha256'])), 'agent SHA256 descriptor')
    ely = load('ely-api-metadata.json')
    check(ely['meta']['serverName'] == 'Ely.by' and 'signaturePublickey' in ely,
          'public Ely.by injector metadata')
    print(json.dumps({'status': 'passed', 'checks': len(checks),
                      'scope': 'offline research metadata only; no binaries or application tested',
                      'versionMetadataSha1': entry['sha1'], 'placeholders': placeholders,
                      'logicalAssets': len(objects), 'uniqueAssets': len(hashes),
                      'platforms': platforms, 'agentDescriptorVersion': agent['version']},
                     indent=2, ensure_ascii=False))


if __name__ == '__main__':
    main()
