#!/usr/bin/env python3
"""Fixed, bounded container lifecycle for OrkWorks' trusted replay fixture."""
import argparse
import json
import os
from pathlib import Path, PurePosixPath
import re
import selectors
import shutil
import signal
import stat
import subprocess
import tarfile
import tempfile
import time

BUDGETS = {'build': 900, 'create': 30, 'run': 90, 'export': 30, 'cleanup': 30}
LOG_LIMIT = 2 * 1024 * 1024
MARKER = b'\n[output truncated]\n'
STAGES = ('visible-wide', 'visible-narrow', 'initially-hidden', 'revealed-wide', 'revealed-narrow')
FILES = {name + ext: (8 if ext == '.png' else 1) * 1024 * 1024
         for name in (*STAGES, 'failure') for ext in ('.png', '.json')}
FILES['subprocess.log'] = 2 * 1024 * 1024
REQUIRED = {name + ext for name in STAGES for ext in ('.png', '.json')} | {'subprocess.log'}
PROXY_ENV = ('HTTP_PROXY', 'HTTPS_PROXY', 'FTP_PROXY', 'ALL_PROXY', 'NO_PROXY',
             'http_proxy', 'https_proxy', 'ftp_proxy', 'all_proxy', 'no_proxy')
INPUTS = {'Containerfile', 'apps/desktop/tsconfig.json', 'apps/desktop/package.json', 'apps/desktop/pnpm-lock.yaml',
          'apps/desktop/pnpm-workspace.yaml', 'apps/desktop/scripts/setup-electron.mjs',
          'apps/desktop/tests/applicationShell.test.mjs'}


class Failure(Exception):
    pass


class Cancelled(Exception):
    pass


class Log:
    def __init__(self, path):
        self.file = path.open('wb')
        self.size = 0
        self.truncated = False

    def write(self, data):
        remaining = LOG_LIMIT - len(MARKER) - self.size
        if remaining > 0:
            chunk = data[:remaining]
            self.file.write(chunk)
            self.size += len(chunk)
        if len(data) > max(0, remaining) and not self.truncated:
            self.file.write(MARKER)
            self.truncated = True
        self.file.flush()

    def close(self):
        self.file.close()


def remaining(deadline):
    value = deadline - time.monotonic()
    if value <= 0:
        raise Failure('operation timed out')
    return value


def command(args, deadline, log, *, sink=None, limit=1024 * 1024, quiet=False):
    """Drain both pipes without unbounded buffering; kill only our live CLI group."""
    remaining(deadline)
    process = subprocess.Popen(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, start_new_session=True)
    captured = bytearray()
    count = 0
    try:
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ, 'stdout')
            selector.register(process.stderr, selectors.EVENT_READ, 'stderr')
            while selector.get_map():
                for key, _ in selector.select(min(remaining(deadline), 0.1)):
                    data = os.read(key.fileobj.fileno(), 65536)
                    if not data:
                        selector.unregister(key.fileobj)
                        continue
                    if key.data == 'stderr':
                        log.write(data)
                    elif sink is not None:
                        count += len(data)
                        if count > limit:
                            raise Failure('artifact transfer exceeded byte limit')
                        sink.write(data)
                    else:
                        if not quiet:
                            log.write(data)
                        if len(captured) < limit:
                            captured.extend(data[:limit-len(captured)])
            process.wait(timeout=remaining(deadline))
        return process.returncode, bytes(captured)
    except subprocess.TimeoutExpired as error:
        raise Failure('operation timed out') from error
    finally:
        if process.returncode is None:
            # Do not poll/reap the leader before killing its owned group: an
            # exited leader may have a descendant keeping one pipe open.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=2)
        process.stdout.close()
        process.stderr.close()


def prepare_context(root, destination, deadline, log):
    status, data = command(['git', '-C', str(root), 'ls-files', '-z'], deadline, log, quiet=True)
    if status:
        raise Failure('cannot enumerate tracked fixture inputs')
    copied = set()
    total = 0
    for relative in data.decode('utf-8').split('\0'):
        path = PurePosixPath(relative)
        if not relative or any(part.startswith('.') for part in path.parts):
            continue
        if relative not in INPUTS and not relative.startswith(('apps/desktop/src/', 'apps/desktop/patches/')):
            continue
        source = root / relative
        for parent in (source, *source.parents):
            if parent == root:
                break
            if parent.is_symlink():
                raise Failure('fixture input symlink refused: ' + relative)
        if not source.is_file():
            raise Failure('fixture input is not a regular file: ' + relative)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        with source.open('rb') as reader, target.open('xb') as writer:
            size = 0
            while True:
                remaining(deadline)
                chunk = reader.read(65536)
                if not chunk:
                    break
                size += len(chunk)
                total += len(chunk)
                if size > 16*1024*1024 or total > 128*1024*1024:
                    raise Failure('fixture context input exceeds size limit')
                writer.write(chunk)
        copied.add(relative)
    if not INPUTS <= copied:
        raise Failure('required tracked fixture inputs are missing')


def read_id(path, *, image=False):
    if not path.exists():
        return None
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_size > 80:
        raise Failure('invalid private identity file')
    value = path.read_text().strip()
    pattern = r'(?:sha256:)?[a-f0-9]{64}' if image else r'[a-f0-9]{64}'
    if not re.fullmatch(pattern, value):
        raise Failure('invalid private identity')
    return value


def verify_owner(path):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_gid != os.getgid():
        raise Failure('export ownership/type verification failed')


def export_file(engine, cid, name, output, deadline, log):
    maximum = FILES[name]
    # Engine cp's tar stream is bounded before parsing/writing any host artifact.
    with tempfile.TemporaryFile() as archive:
        status, _ = command([engine, 'cp', cid + ':/artifacts/replay/' + name, '-'],
                            deadline, log, sink=archive, limit=maximum+65536)
        if status:
            return None
        archive.seek(0)
        try:
            with tarfile.open(fileobj=archive, mode='r:') as bundle:
                members = bundle.getmembers()
                if len(members) != 1:
                    raise Failure('artifact archive must contain exactly one regular file')
                member = members[0]
                if member.name not in (name, './'+name) or not member.isfile() or member.size > maximum:
                    raise Failure('artifact archive name/type/size refused')
                reader = bundle.extractfile(member)
                if reader is None:
                    raise Failure('artifact stream missing')
                with reader:
                    data = reader.read(maximum+1)
                if len(data) != member.size or len(data) > maximum:
                    raise Failure('artifact length mismatch')
        except tarfile.TarError as error:
            raise Failure('invalid artifact archive') from error
    target = output / name
    fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as writer:
        os.fchown(writer.fileno(), -1, os.getgid())
        writer.write(data)
    verify_owner(target)
    return len(data)


def read_environment(engine, resource, identity, deadline, log):
    # Never write environment values to the retained log or receipt.
    status, data = command([engine, resource, 'inspect', '--format', '{{json .Config.Env}}', identity],
                           deadline, log, quiet=True)
    if status:
        raise Failure('runtime environment could not be verified')
    entries = json.loads(data)
    if not isinstance(entries, list):
        raise Failure('invalid runtime environment')
    values = {}
    for entry in entries:
        if not isinstance(entry, str) or '=' not in entry:
            raise Failure('invalid runtime environment entry')
        name, value = entry.split('=', 1)
        if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*', name) or name in values:
            raise Failure('invalid or duplicate runtime environment name')
        values[name] = value
    return values


def verify_runtime(engine, cid, expected, deadline, log):
    actual = read_environment(engine, 'container', cid, deadline, log)
    allowed = dict(expected)
    allowed.update({name: '' for name in PROXY_ENV})
    allowed['HOSTNAME'] = cid[:12]
    if engine == 'podman':
        allowed['container'] = 'podman'
    wrong = {name for name, value in actual.items() if name not in allowed or value != allowed[name]}
    wrong.update(name for name in expected if name not in actual)
    if wrong:
        raise Failure('runtime environment refused (names only): ' + ', '.join(sorted(wrong)))
    status, data = command([engine, 'container', 'inspect', '--format', '{{json .Mounts}}', cid],
                           deadline, log, quiet=True)
    if status or json.loads(data) != []:
        raise Failure('unexpected or unverifiable container mounts refused')


def confirmed(receipt, milestone, state=None):
    receipt['milestones'].append(milestone)
    if state is not None:
        receipt['last_confirmed_state'] = {
            key: state[key] for key in ('Status', 'Running', 'ExitCode', 'OOMKilled') if key in state
        }
        receipt['state_observed_at'] = time.time()


def inspect(engine, cid, deadline, log, receipt):
    status, data = command([engine, 'inspect', '--format', '{{json .State}}', cid], deadline, log, quiet=True)
    if status:
        raise Failure('container state could not be confirmed')
    state = json.loads(data)
    if not isinstance(state.get('Running'), bool):
        raise Failure('invalid container state')
    confirmed(receipt, 'container-state-confirmed', state)
    return state


def stop(engine, cid, deadline, log, receipt):
    command([engine, 'stop', '--time', '5', cid], deadline, log)
    state = inspect(engine, cid, deadline, log, receipt)
    if state['Running']:
        command([engine, 'kill', cid], deadline, log)
        state = inspect(engine, cid, deadline, log, receipt)
    if state['Running']:
        raise Failure('container did not stop')
    return state


def run(engine, root, parent=None, budgets=None):
    budgets = BUDGETS if budgets is None else budgets
    root = Path(root).resolve()
    output = Path(tempfile.mkdtemp(prefix='orkworks-replay-', dir=parent))
    os.chown(output, -1, os.getgid())
    context = output/'context'; context.mkdir()
    replay = output/'replay'; replay.mkdir()
    log = Log(output/'run.log')
    receipt = {'container_id': None, 'cleanup': 'not-created', 'status': 'failed', 'engine': engine, 'milestones': []}
    cid = None
    create_attempted = False
    code = 1
    try:
        executable = shutil.which(engine)
        if not executable:
            raise Failure('selected engine is unavailable: ' + engine)
        deadline = time.monotonic()+budgets['build']
        template = ('{"arch":"{{.Host.Arch}}","os":"{{.Host.OS}}","rootless":{{.Host.Security.Rootless}}}'
                    if engine == 'podman' else
                    '{"arch":"{{.Architecture}}","os":"{{.OSType}}","security":{{json .SecurityOptions}}}')
        status, info = command([engine, 'info', '--format', template], deadline, log, quiet=True)
        if status:
            raise Failure('selected engine is unreachable')
        receipt['profile'] = json.loads(info)
        confirmed(receipt, 'engine-reachable')
        prepare_context(root, context, deadline, log)
        confirmed(receipt, 'context-prepared')
        status, _ = command([engine, 'build', '--target', 'headless-replay', '--iidfile', str(output/'image-id'), str(context)], deadline, log)
        if status:
            raise Failure('image build failed')
        image = read_id(output/'image-id', image=True)
        if not image:
            raise Failure('image build did not produce its identity')
        receipt['image_id'] = image
        confirmed(receipt, 'image-built')
        expected_env = read_environment(engine, 'image', image, deadline, log)
        if any(expected_env.get(name) for name in PROXY_ENV):
            raise Failure('image proxy environment refused')
        create_attempted = True
        create_deadline = time.monotonic()+budgets['create']
        args = [engine, 'create', '--cidfile', str(output/'cid'), '--user=1000:1000',
                '--network=none', '--cap-drop=ALL', '--security-opt=no-new-privileges',
                '--init', '--memory=2g', '--pids-limit=256', '--shm-size=256m']
        if engine == 'podman':
            args.append('--http-proxy=false')
        for name in PROXY_ENV:
            args.extend(['--env', name+'='])
        status, _ = command(args+[image], create_deadline, log)
        cid = read_id(output/'cid')
        if not cid or status:
            raise Failure('container creation failed or its identity is unsettled')
        receipt['container_id'] = cid
        confirmed(receipt, 'container-created')
        verify_runtime(engine, cid, expected_env, create_deadline, log)
        confirmed(receipt, 'runtime-configuration-verified')
        status, _ = command([engine, 'start', '--attach', cid], time.monotonic()+budgets['run'], log)
        confirmed(receipt, 'fixture-attach-returned')
        state = inspect(engine, cid, time.monotonic()+budgets['cleanup'], log, receipt)
        if state['Running'] or status or state.get('ExitCode') != 0:
            raise Failure('replay fixture did not exit successfully')
        code = 0
    except (Exception, KeyboardInterrupt) as error:
        receipt['reason'] = str(error) or 'interrupted'
        log.write(('\n' + receipt['reason'] + '\n').encode())
    finally:
        # Further signals must not skip the finite, exact-CID cleanup attempt.
        previous = {}
        for sig in (signal.SIGINT, signal.SIGTERM):
            previous[sig] = signal.signal(sig, signal.SIG_IGN)
        try:
            if cid is None and create_attempted:
                try:
                    cid = read_id(output/'cid')
                except Exception as error:
                    receipt['identity_error'] = str(error)
            receipt['container_id'] = cid
            if cid:
                cleanup_start = time.monotonic()
                stopped = False
                try:
                    state = stop(engine, cid, cleanup_start+budgets['cleanup'], log, receipt)
                    stopped = not state['Running']
                except Exception as error:
                    receipt['stop_error'] = str(error)
                    code = 1
                cleanup_spent = time.monotonic()-cleanup_start
                if stopped:
                    try:
                        deadline = time.monotonic()+budgets['export']
                        total = 0; exported = set()
                        for name in FILES:
                            size = export_file(engine, cid, name, replay, deadline, log)
                            if size is not None:
                                total += size; exported.add(name)
                            if total > 64*1024*1024:
                                raise Failure('total artifact limit exceeded')
                        if code == 0 and not REQUIRED <= exported:
                            raise Failure('successful fixture is missing required artifacts')
                        receipt['exported'] = sorted(exported)
                    except Exception as error:
                        receipt['export_error'] = str(error); code = 1
                try:
                    status, _ = command([engine, 'rm', cid], time.monotonic()+max(0, budgets['cleanup']-cleanup_spent), log)
                    receipt['cleanup'] = 'removed' if status == 0 else 'unsettled'
                    if status:
                        code = 1
                    else:
                        confirmed(receipt, 'container-removed')
                except Exception as error:
                    receipt['cleanup'] = 'unsettled'; receipt['cleanup_error'] = str(error); code = 1
            elif create_attempted:
                receipt['cleanup'] = 'identity-unsettled'
                receipt['cid_file'] = str(output/'cid')
            shutil.rmtree(context)
            receipt['status'] = 'passed' if code == 0 else 'failed'
            (output/'result.json').write_text(json.dumps(receipt, indent=2)+'\n')
        finally:
            log.close()
            for sig, handler in previous.items():
                signal.signal(sig, handler)
    print('Artifacts:', output, flush=True)
    print('Result:', receipt['status'], '| cleanup:', receipt['cleanup'], flush=True)
    if receipt['cleanup'] in ('unsettled', 'identity-unsettled'):
        print('Unsettled CID:', cid, '| CID file:', output/'cid', flush=True)
        print('Last confirmed:', receipt['milestones'][-1] if receipt['milestones'] else 'none',
              receipt.get('last_confirmed_state'), flush=True)
    return code, output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', required=True, choices=('docker', 'podman'))
    args = parser.parse_args()
    def interrupted(_signal, _frame):
        raise Cancelled('interrupted')
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, interrupted)
    code, _ = run(args.engine, Path(__file__).resolve().parent.parent)
    return code


if __name__ == '__main__':
    raise SystemExit(main())
