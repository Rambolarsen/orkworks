#!/usr/bin/env python3
"""Local SonarQube lifecycle, exact-source scans and comparable JSON reports."""
import argparse
import base64
from contextlib import contextmanager
import csv
import hashlib
import json
import os
from pathlib import Path
import secrets
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
METRICS = ('ncloc', 'complexity', 'cognitive_complexity', 'duplicated_lines',
           'duplicated_lines_density', 'functions')
COPY_ROOTS = ('apps/desktop/', 'crates/orkworksd/', 'crates/process-ownership-fixture/')
SOURCE_ROOTS = ('apps/desktop/src/', 'apps/desktop/electron/', 'crates/orkworksd/src/')
GENERATED = {'node_modules', 'target', 'dist', 'dist-electron', 'out', 'release', '.sonar', '.scannerwork'}
EXTENSIONS = {'.rs', '.toml', '.lock', '.ts', '.tsx', '.js', '.mjs', '.cjs', '.json',
              '.yaml', '.yml', '.html', '.css', '.svg', '.md', '.sh', '.ps1', '.patch'}
SCANNER_VERSION = '5.0.1'


def run(*args, cwd=ROOT, capture=False, env=None, encoding="utf-8"):
    return subprocess.run(args, cwd=cwd, env=env, check=True,
                          text=True, encoding=encoding, stdout=subprocess.PIPE if capture else None).stdout


def digest(value):
    return hashlib.sha256(value).hexdigest()


def project_key(root):
    return 'orkworks-' + digest(str(root.resolve()).encode())[:16]


def snapshot(root, destination):
    """Copy Git-listed working files, not HEAD blobs or platform-specific output."""
    names = run('git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard',
                cwd=root, capture=True).split('\0')
    hashed = hashlib.sha256()
    count = 0
    for name in sorted(set(filter(None, names))):
        relative = Path(name)
        if name not in ('sonar-project.properties', 'rust-toolchain.toml', '.nvmrc'):
            if not name.startswith(COPY_ROOTS) or relative.suffix not in EXTENSIONS:
                continue
        if GENERATED.intersection(relative.parts) or relative.name.startswith('.env'):
            continue
        source = root / relative
        if source.is_symlink():
            raise ValueError('Source symlink is not supported: ' + name)
        if not source.exists():
            continue
        if not source.is_file() or not source.resolve().is_relative_to(root.resolve()):
            raise ValueError('Source must be a regular file inside the checkout: ' + name)
        data = source.read_bytes()
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
        target.chmod(source.stat().st_mode & 0o777)
        hashed.update(name.encode() + b'\0' + digest(data).encode() + b'\0')
        if name.startswith(SOURCE_ROOTS) and relative.suffix in {'.rs', '.ts', '.tsx'}:
            count += 1
    return {'commit': run('git', 'rev-parse', 'HEAD', cwd=root, capture=True).strip(),
            'branch': run('git', 'branch', '--show-current', cwd=root, capture=True).strip(),
            'dirty': bool(run('git', 'status', '--porcelain', cwd=root, capture=True).strip()),
            'fingerprint': hashed.hexdigest(), 'sourceFiles': count}


def metric_values(measures):
    values = {item['metric']: float(item['value']) for item in measures if 'value' in item}
    return {key: values.get(key) for key in METRICS}


def completed_task(payload, project):
    task = payload['task']
    if task['status'] in ('FAILED', 'CANCELED'):
        raise ValueError('Sonar background analysis ' + task['status'])
    if task['status'] != 'SUCCESS':
        return None
    if task.get('componentKey') != project or not task.get('analysisId'):
        raise ValueError('Completed analysis does not belong to this project')
    return task['analysisId']


def verify_analysis(expected_id, payload):
    analyses = payload.get('analyses', [])
    if not analyses or analyses[0]['key'] != expected_id:
        raise ValueError('Latest server analysis differs from the submitted snapshot; rescan')


def compare(before, after):
    if before['configuration'] != after['configuration']:
        raise ValueError('Cannot compare different analysis configuration, scope or analyzers')
    result = {}
    for key in METRICS:
        old, new = before['metrics'].get(key), after['metrics'].get(key)
        result[key] = {'before': old, 'after': new,
                       'delta': None if old is None or new is None else new - old}
    return result


class Api:
    def __init__(self, settings, basic=False):
        self.url = 'http://127.0.0.1:9000'
        self.authorization = ('Basic ' + base64.b64encode(
            ('admin:' + settings['SONAR_ADMIN_PASSWORD']).encode()).decode()
            if basic else 'Bearer ' + settings['SONAR_TOKEN'])
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(self, endpoint, params=None, post=False):
        query = urllib.parse.urlencode(params or {})
        url = self.url + '/' + endpoint + (('?' + query) if query and not post else '')
        request = urllib.request.Request(url, data=query.encode() if post else None,
                                         headers={'Authorization': self.authorization})
        try:
            with self.opener.open(request, timeout=30) as response:
                raw = response.read()
                return json.loads(raw) if raw else {}
        except urllib.error.HTTPError as error:
            raise ValueError('Sonar API ' + endpoint + ' returned HTTP ' + str(error.code)) from None

    def pages(self, endpoint, field, params):
        items, page = [], 1
        while True:
            payload = self.request(endpoint, dict(params, p=page, ps=500))
            batch = payload[field]
            items.extend(batch)
            total = payload.get('paging', {}).get('total', payload.get('total'))
            if total is None or total > 10000:
                raise ValueError('Sonar pagination cannot provide a complete ' + field + ' inventory')
            if len(items) >= total:
                return items
            if not batch:
                raise ValueError('Incomplete Sonar ' + field + ' pagination')
            page += 1


@contextmanager
def scan_lock(path, message='A scan already owns this worktree'):
    with path.open('a+b') as lock:
        if os.name == 'nt':
            import msvcrt
            if path.stat().st_size == 0:
                lock.write(b'0')
                lock.flush()
            lock.seek(0)
            acquire = lambda: msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
            release = lambda: msvcrt.locking(lock.fileno(), msvcrt.LK_UNLCK, 1)
        else:
            import fcntl
            acquire = lambda: fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            release = lambda: fcntl.flock(lock, fcntl.LOCK_UN)
        try:
            acquire()
        except OSError:
            raise ValueError(message) from None
        try:
            yield
        finally:
            lock.seek(0)
            release()


def default_settings_path(windows=None):
    if windows is None:
        windows = os.name == 'nt'
    if windows:
        return Path(os.environ['LOCALAPPDATA']) / 'OrkWorks/sonar.env'
    return Path.home() / '.config/orkworks/sonar.env'


def settings_file():
    override = os.environ.get('SONAR_ENV_FILE')
    return (Path(override).expanduser() if override else default_settings_path()).absolute()


def write_settings(path, settings):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.is_symlink():
        raise ValueError('Credential file must not be a symlink')
    descriptor, temporary = tempfile.mkstemp(prefix='.sonar-', dir=path.parent)
    try:
        with os.fdopen(descriptor, 'w', encoding='utf-8', newline='\n') as stream:
            if os.name == 'nt':
                sid = next(csv.reader([run('whoami', '/user', '/fo', 'csv', '/nh', capture=True, encoding='oem').strip()]))[1]
                run('icacls', temporary, '/reset', capture=True, encoding='oem')
                run('icacls', temporary, '/inheritance:r', '/grant:r', '*' + sid + ':(F)', capture=True, encoding='oem')
            else:
                os.fchmod(stream.fileno(), 0o600)
            stream.write(''.join(key + '=' + value + '\n' for key, value in settings.items()))
        os.replace(temporary, path)
    finally:
        if Path(temporary).exists():
            Path(temporary).unlink()


def new_password():
    return 'Aa1-' + secrets.token_hex(24)


def load_settings(create=False):
    path = settings_file()
    if path.is_symlink():
        raise ValueError('Credential file must not be a symlink')
    if not path.exists() and create:
        write_settings(path, {name: new_password() for name in
                             ('SONAR_DB_PASSWORD', 'SONAR_ADMIN_PASSWORD')})
    if not path.exists():
        raise ValueError('Run up first to initialize local credentials')
    if os.name != 'nt' and path.stat().st_mode & 0o077:
        raise ValueError('Credential file must have mode 600: ' + str(path))
    settings = dict(line.split('=', 1) for line in path.read_text(encoding='utf-8').splitlines() if line)
    return settings


def compose(*args, env=None):
    environment = dict(os.environ, **(env or {}))
    environment.update(load_settings())
    return run('podman', 'compose', '-p', 'orkworks-sonar', '-f', str(ROOT / 'compose.sonar.yaml'),
               '--env-file', str(settings_file()), '--profile', 'scan', *args, env=environment)


def wait_for(check, seconds=900):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        result = check()
        if result:
            return result
        time.sleep(2)
    raise ValueError('Timed out waiting for SonarQube; inspect the local container logs')


def stack_lock():
    lock = default_settings_path().with_suffix('.stack.lock')
    lock.parent.mkdir(parents=True, exist_ok=True)
    return scan_lock(lock, 'SonarQube stack is in use; retry after the active operation finishes')


def up():
    with stack_lock():
        initialize()


def initialize():
    settings = load_settings(create=True)
    compose('up', '-d', 'db', 'sonarqube')
    def healthy():
        try:
            with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(
                    'http://127.0.0.1:9000/api/system/status', timeout=5) as response:
                return json.load(response)['status'] == 'UP'
        except OSError:
            return False
    wait_for(healthy)
    valid = False
    if settings.get('SONAR_TOKEN'):
        try:
            valid = Api(settings).request('api/authentication/validate')['valid']
        except ValueError:
            pass
    if not valid:
        # Try the saved password first so an interrupted setup is resumable.
        try:
            token = Api(settings, basic=True).request('api/user_tokens/generate',
                    {'name': 'orkworks-local-' + secrets.token_hex(6)}, post=True)['token']
        except ValueError:
            initial = dict(settings, SONAR_ADMIN_PASSWORD='admin')
            Api(initial, basic=True).request('api/users/change_password',
                    {'login': 'admin', 'previousPassword': 'admin',
                     'password': settings['SONAR_ADMIN_PASSWORD']}, post=True)
            token = Api(settings, basic=True).request('api/user_tokens/generate',
                    {'name': 'orkworks-local-' + secrets.token_hex(6)}, post=True)['token']
        settings['SONAR_TOKEN'] = token
        write_settings(settings_file(), settings)
    print('SonarQube ready at http://127.0.0.1:9000; credentials: ' + str(settings_file()))


def analysis_identity(api, project, properties, image=None, toolchain=None):
    profiles = api.request('api/qualityprofiles/search', {'project': project})['profiles']
    plugins = api.request('api/plugins/installed')['plugins']
    effective = {}
    for scope, params in (('global', {}), ('project', {'component': project})):
        settings = api.request('api/settings/values', params)['settings']
        effective[scope] = {
            item['key']: {key: item[key] for key in ('value', 'values', 'fieldValues') if key in item}
            for item in settings if item['key'] not in ('sonar.core.startTime', 'sonar.core.id')}
    gate = api.request('api/qualitygates/get_by_project', {'project': project})['qualityGate']['name']
    conditions = api.request('api/qualitygates/show', {'name': gate})['conditions']
    return {'server': api.request('api/system/status')['version'],
            'plugins': sorted([p['key'], p.get('version')] for p in plugins),
            'profiles': sorted([p['key'], p['language'], p.get('rulesUpdatedAt')] for p in profiles),
            'scanner': SCANNER_VERSION, 'scannerImage': image, 'toolchain': toolchain, 'properties': properties,
            # Hash settings, rather than publishing possible secured values.
            'effectiveSettings': digest(json.dumps(effective, sort_keys=True).encode()),
            'qualityGate': {'name': gate, 'conditions': sorted(conditions, key=lambda c: c['metric'])}}


def verify_configuration(expected, current):
    if not expected or expected != current:
        raise ValueError('Analysis configuration changed or is unbound; rescan')


def collect(api, state):
    project, analysis = state['project'], state['analysisId']
    verify_analysis(analysis, api.request('api/project_analyses/search', {'project': project, 'ps': 1}))
    identity = analysis_identity(api, project, state['properties'], state.get('scannerImage'), state.get('toolchain'))
    verify_configuration(state.get('analyzerIdentity'), identity)
    component = api.request('api/measures/component',
                {'component': project, 'metricKeys': ','.join(METRICS)})['component']
    files = api.pages('api/measures/component_tree', 'components',
                {'component': project, 'qualifiers': 'FIL', 'metricKeys': ','.join(METRICS)})
    issues = api.pages('api/issues/search', 'issues', {'componentKeys': project, 'resolved': 'false'})
    gate = api.request('api/qualitygates/project_status', {'analysisId': analysis})['projectStatus']
    # Project/file measures are latest-only APIs: detect another scan during collection.
    verify_analysis(analysis, api.request('api/project_analyses/search', {'project': project, 'ps': 1}))
    verify_configuration(identity, analysis_identity(api, project, state['properties'], state.get('scannerImage'), state.get('toolchain')))
    return dict(state, configuration=digest(json.dumps(identity, sort_keys=True).encode()),
                analyzerIdentity=identity, metrics=metric_values(component.get('measures', [])),
                files=[{'path': f['path'], 'metrics': metric_values(f.get('measures', []))} for f in files],
                issues=issues, qualityGate=gate, coverage='unverified: no imported coverage report',
                lineCountScope='analyzed sources include Rust inline tests',
                verifiedAt=time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()))


def save_report(report, label, overwrite=False):
    directory = ROOT / '.sonar/reports'
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / (label + '.json')
    with target.open('w' if overwrite else 'x', encoding='utf-8') as stream:
        stream.write(json.dumps(report, indent=2) + '\n')
    print(str(target))


def scan(label, overwrite=False):
    settings = load_settings()
    if 'SONAR_TOKEN' not in settings:
        raise ValueError('Run up to complete credential initialization before scanning')
    api = Api(settings)
    project = project_key(ROOT)
    directory = ROOT / '.sonar'
    directory.mkdir(exist_ok=True)
    with stack_lock(), scan_lock(directory / 'scan.lock'):
        with tempfile.TemporaryDirectory(prefix='orkworks-sonar-') as temporary:
            source = Path(temporary)
            state = snapshot(ROOT, source)
            if not state['sourceFiles']:
                raise ValueError('Snapshot has no source files')
            state.update(project=project, properties=(source / 'sonar-project.properties').read_text(encoding='utf-8'))
            if not api.request('api/projects/search', {'projects': project})['components']:
                api.request('api/projects/create', {'project': project, 'name': 'OrkWorks - ' + ROOT.name}, post=True)
            env = dict(os.environ, SONAR_WORKTREE_ID=project, SONAR_TOKEN=settings['SONAR_TOKEN'])
            compose('build', 'scanner', env=env)
            container = project + '-scan-' + secrets.token_hex(4)
            compose('run', '-d', '--no-deps', '--name', container,
                    'scanner', 'sleep', 'infinity', env=env)
            try:
                state['scannerImage'] = run('podman', 'inspect', '--format', '{{.Image}}', container, capture=True).strip()
                # Relative paths avoid drive-letter parsing and VM host-share requirements.
                run('podman', 'cp', './.', container + ':/workspace', cwd=source)
                state['toolchain'] = {name: run('podman', 'exec', container, *command, capture=True).strip()
                    for name, command in (('rustc', ('rustc', '--version', '--verbose')),
                                          ('cargo', ('cargo', '--version')),
                                          ('clippy', ('cargo', 'clippy', '--version')))}
                state['analyzerIdentity'] = analysis_identity(api, project, state['properties'],
                                                            state['scannerImage'], state['toolchain'])
                run('podman', 'exec', container, 'bash', '-c',
                    'cd apps/desktop && pnpm install --frozen-lockfile && '
                    'cd /workspace && sonar-scanner-npm "$@"', 'scan',
                    '-Dsonar.projectKey=' + project,
                    '-Dsonar.projectVersion=' + state['fingerprint'])
                run('podman', 'cp', container + ':/workspace/.scannerwork/report-task.txt',
                    './report-task.txt', cwd=source)
                task = dict(line.split('=', 1) for line in
                            (source / 'report-task.txt').read_text(encoding='utf-8').splitlines())
            finally:
                run('podman', 'rm', '-f', container)
            if task.get('projectKey') != project:
                raise ValueError('Scanner task does not belong to this worktree')
            def completion():
                payload = api.request('api/ce/task', {'id': task['ceTaskId'], 'additionalFields': 'warnings'})
                analysis = completed_task(payload, project)
                if analysis:
                    state['analysisWarnings'] = payload['task'].get('warnings', [])
                    if len(state['analysisWarnings']) != payload['task'].get('warningCount', 0):
                        raise ValueError('Incomplete analysis warning inventory')
                return analysis
            state['analysisId'] = wait_for(completion)
            report = collect(api, state)
            (directory / 'latest.json').write_text(json.dumps(state, indent=2) + '\n', encoding='utf-8')
            save_report(report, label, overwrite)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    commands.add_parser('up')
    commands.add_parser('down')
    for name in ('scan', 'report'):
        command = commands.add_parser(name)
        command.add_argument('--label', default='current')
        command.add_argument('--overwrite', action='store_true', help='Explicitly replace an existing report label')
    command = commands.add_parser('compare')
    command.add_argument('before', type=Path)
    command.add_argument('after', type=Path)
    args = parser.parse_args()
    if not getattr(args, 'label', 'current') or getattr(args, 'label', 'current').strip('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_'):
        parser.error('label accepts only letters, numbers, hyphens and underscores')
    try:
        if args.command in ('scan', 'report') and not args.overwrite:
            if (ROOT / '.sonar/reports' / (args.label + '.json')).exists():
                raise ValueError('Report label already exists; choose another label or explicitly use --overwrite')
        if args.command == 'up':
            up()
        elif args.command == 'down':
            with stack_lock():
                load_settings()
                compose('down')
        elif args.command == 'scan':
            scan(args.label, args.overwrite)
        elif args.command == 'report':
            with stack_lock():
                state = json.loads((ROOT / '.sonar/latest.json').read_text(encoding='utf-8'))
                with tempfile.TemporaryDirectory() as temporary:
                    if snapshot(ROOT, Path(temporary))['fingerprint'] != state['fingerprint']:
                        raise ValueError('Working source changed since the last scan; rescan')
                save_report(collect(Api(load_settings()), state), args.label, args.overwrite)
        else:
            print(json.dumps(compare(json.loads(args.before.read_text(encoding='utf-8')),
                                     json.loads(args.after.read_text(encoding='utf-8'))), indent=2))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, 'sonar: ' + str(error) + '\n')


if __name__ == '__main__':
    main()
