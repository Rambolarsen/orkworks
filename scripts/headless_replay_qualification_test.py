#!/usr/bin/env python3
"""Opt-in real-engine qualification; never runs a coding tool or a real user session."""
import errno
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import signal
import stat
import subprocess
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
spec = importlib.util.spec_from_file_location('replay', HERE/'headless_replay.py')
replay = importlib.util.module_from_spec(spec)
spec.loader.exec_module(replay)
ENGINE = os.environ.get('ORKWORKS_REAL_REPLAY_ENGINE')


def read_observed_cid(path):
    """Classify a CID file while container creation may still be publishing it."""
    try:
        info = path.lstat()
    except FileNotFoundError:
        return 'missing', None, None
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_size > 80:
        raise replay.Failure('invalid private identity file')
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    except FileNotFoundError:
        return 'missing', None, None
    except OSError as error:
        if error.errno in (errno.ELOOP, errno.EMLINK):
            raise replay.Failure('invalid private identity file') from error
        raise
    try:
        opened = os.fstat(fd)
        if not stat.S_ISREG(opened.st_mode) or opened.st_uid != os.getuid() or opened.st_size > 80:
            raise replay.Failure('invalid private identity file')
        chunks = bytearray()
        while len(chunks) < 81:
            chunk = os.read(fd, 81 - len(chunks))
            if not chunk:
                break
            chunks.extend(chunk)
        raw = bytes(chunks)
    finally:
        os.close(fd)
    if len(raw) > 80:
        raise replay.Failure('invalid private identity file')
    if re.fullmatch(rb'[a-f0-9]{0,63}', raw):
        detail = 'empty' if not raw else f'partial-hex({len(raw)})'
        return 'incomplete', None, detail
    try:
        value = raw.decode('ascii').strip()
    except UnicodeDecodeError as error:
        raise replay.Failure(f'invalid private identity (non-ascii, bytes={len(raw)})') from error
    if not re.fullmatch(r'[a-f0-9]{64}', value):
        raise replay.Failure(f'invalid private identity (malformed, bytes={len(raw)})')
    cid = replay.read_id(path)
    if not cid:
        return 'missing', None, None
    return 'valid', cid, None


def observe_cid_ready(
    output,
    *,
    existing_runs,
    check_ready,
    done,
    ready,
    on_ready=None,
    on_error=None,
    poll_interval=0.1,
    incomplete_grace=5.0,
    monotonic=time.monotonic,
):
    first_incomplete = {}
    transient_incomplete = []
    transient_count = 0
    while True:
        for path in sorted(output.glob('orkworks-replay-*/cid')):
            if path.parent in existing_runs:
                continue
            try:
                status, cid, detail = read_observed_cid(path)
            except replay.Failure as error:
                if on_error:
                    on_error(error)
                return {
                    'error': error,
                    'transient_count': transient_count,
                    'transient_incomplete': transient_incomplete,
                }
            except OSError:
                continue
            if status == 'missing':
                continue
            if status == 'incomplete':
                now = monotonic()
                first = first_incomplete.setdefault(path, now)
                transient_count += 1
                if len(transient_incomplete) < 32:
                    transient_incomplete.append((path.parent.name, detail))
                if now - first >= incomplete_grace:
                    error = replay.Failure(f'persistent incomplete private identity ({detail})')
                    if on_error:
                        on_error(error)
                    return {
                        'error': error,
                        'transient_count': transient_count,
                        'transient_incomplete': transient_incomplete,
                    }
                continue
            first_incomplete.pop(path, None)
            try:
                if check_ready(cid):
                    ready.set()
                    if on_ready:
                        on_ready()
                    return {
                        'error': None,
                        'transient_count': transient_count,
                        'transient_incomplete': transient_incomplete,
                    }
            except (OSError, subprocess.TimeoutExpired):
                continue
        if done.wait(poll_interval):
            return {
                'error': None,
                'transient_count': transient_count,
                'transient_incomplete': transient_incomplete,
            }


def source_snapshot():
    paths = subprocess.check_output(['git', '-C', str(ROOT), 'ls-files', '-z']).decode().split('\0')
    result = {}
    for name in paths:
        if name in replay.INPUTS or name.startswith(('apps/desktop/src/', 'apps/desktop/patches/')):
            path = ROOT/name
            info = path.stat()
            result[name] = (hashlib.sha256(path.read_bytes()).hexdigest(), info.st_uid, info.st_gid, info.st_mode)
    return result


@unittest.skipUnless(ENGINE in ('docker', 'podman'), 'set ORKWORKS_REAL_REPLAY_ENGINE to opt in')
class RealQualification(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.output = Path(tempfile.mkdtemp(prefix='orkworks-replay-qualification-'))
        cls.before = source_snapshot()
        cls.canary = None
        print('Qualification evidence:', cls.output, flush=True)

    @classmethod
    def tearDownClass(cls):
        try:
            cid = replay.read_id(cls.output/'canary-cid')
            if cid:
                subprocess.run([ENGINE, 'rm', cid], check=True, timeout=30, stdout=subprocess.DEVNULL)
        finally:
            (cls.output/'host.json').write_text(json.dumps({
                'host': platform.system(), 'architecture': platform.machine(),
                'source_unchanged': source_snapshot() == cls.before,
                'source_input_count': len(cls.before), 'uid': os.getuid(), 'gid': os.getgid(),
            }, indent=2)+'\n')

    def assert_removed(self, directory):
        receipt = json.loads((directory/'result.json').read_text())
        self.assertEqual(receipt['cleanup'], 'removed', receipt)
        result = subprocess.run([ENGINE, 'container', 'inspect', receipt['container_id']],
                                capture_output=True, timeout=15)
        self.assertNotEqual(result.returncode, 0, 'fixture container survived cleanup')
        if self.canary:
            subprocess.run([ENGINE, 'container', 'inspect', self.canary], check=True,
                           stdout=subprocess.DEVNULL, timeout=15)
        self.assertEqual(source_snapshot(), self.before, 'host source/ownership changed')
        return receipt

    def test_01_success_and_caller_owned_artifacts(self):
        code, directory = replay.run(ENGINE, ROOT, self.output)
        self.assertEqual(code, 0, (directory/'result.json').read_text())
        receipt = self.assert_removed(directory)
        for stage in replay.STAGES:
            png = directory/'replay'/(stage+'.png')
            self.assertEqual(png.read_bytes()[:8], b'\x89PNG\r\n\x1a\n')
            replay.verify_owner(png)
        report = json.loads((directory/'replay/revealed-narrow.json').read_text())
        self.assertEqual(len(report['rows']), 40)
        self.assertEqual(report['rows'][0], '0123456789'*12)
        self.assertEqual(report['rows'][1].strip(), 'Z')
        self.assertEqual(report['rows'][39].strip(), 'last-row')
        self.assertEqual(receipt['profile']['os'], 'linux')
        if ENGINE == 'docker':
            self.assertEqual(platform.system(), 'Linux', 'Docker Desktop is not this profile')
            self.assertIn(receipt['profile']['arch'], ('amd64','x86_64'))
            self.assertNotIn('name=rootless', receipt['profile']['security'])
        # A stopped, separately owned canary must survive all following cases.
        subprocess.run([ENGINE, 'create', '--cidfile', str(self.output/'canary-cid'),
                        '--network=none', '--cap-drop=ALL', '--user=1000:1000', receipt['image_id']],
                       check=True, timeout=30, stdout=subprocess.DEVNULL)
        type(self).canary = replay.read_id(self.output/'canary-cid')

    def run_fault(self, kind):
        original = replay.prepare_context
        def context(root, destination, deadline, log):
            original(root, destination, deadline, log)
            fixture = destination/'apps/desktop/tests/applicationShell.test.mjs'
            text = fixture.read_text()
            if kind == 'assertion':
                before = 'assert.equal(view.rows.length, 40,'
                self.assertEqual(text.count(before), 1)
                text = text.replace(before, 'assert.equal(view.rows.length, 41,', 1)
            else:
                before = '      await fresh();'
                self.assertIn(before, text)
                text = text.replace(before, before + '''
      (await import('node:fs')).writeFileSync('/artifacts/replay/qualification-ready', 'ready');
      await new Promise(() => {});''', 1)
            fixture.write_text(text)
        existing_runs = set(self.output.glob('orkworks-replay-*'))
        ready = threading.Event()
        done = threading.Event()
        observer_state = {}
        def check_ready(cid):
            result = subprocess.run([ENGINE, 'exec', cid, 'test', '-f',
                                     '/artifacts/replay/qualification-ready'],
                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=3)
            return result.returncode == 0
        def interrupt_process(_error=None):
            if kind == 'interrupt':
                os.kill(os.getpid(), signal.SIGTERM)
        def observe():
            observer_state.update(observe_cid_ready(
                self.output,
                existing_runs=existing_runs,
                check_ready=check_ready,
                done=done,
                ready=ready,
                on_ready=interrupt_process if kind == 'interrupt' else None,
                on_error=interrupt_process if kind == 'interrupt' else None,
            ))
        watcher = None
        cancelled = None
        previous = signal.getsignal(signal.SIGTERM)
        def interrupted(_signal, _frame):
            raise replay.Cancelled('interrupted')
        signal.signal(signal.SIGTERM, interrupted)
        try:
            if kind != 'assertion':
                watcher = threading.Thread(target=observe, daemon=True)
                watcher.start()
            budgets = dict(replay.BUDGETS)
            budgets['run'] = 8 if kind == 'timeout' else 90
            with patch.object(replay, 'prepare_context', context):
                code, directory = replay.run(ENGINE, ROOT, self.output, budgets)
        except replay.Cancelled as error:
            cancelled = error
        finally:
            done.set()
            if watcher:
                watcher.join(timeout=5)
            signal.signal(signal.SIGTERM, previous)
        if observer_state.get('transient_count'):
            print(
                f"CID observer tolerated {observer_state['transient_count']} transient incomplete read(s): "
                f"{observer_state['transient_incomplete']}",
                flush=True,
            )
        if observer_state.get('error') is not None:
            raise observer_state['error']
        if cancelled is not None:
            raise cancelled
        self.assertNotEqual(code, 0)
        receipt = self.assert_removed(directory)
        if kind == 'assertion':
            self.assertTrue((directory/'replay/failure.png').is_file())
            self.assertIn('recorded rows survive container resizing', (directory/'replay/subprocess.log').read_text())
        else:
            self.assertTrue(ready.is_set(), 'real renderer never reached the intentional hang')
            self.assertIn('timed out' if kind == 'timeout' else 'interrupted', receipt['reason'])

    def test_02_assertion_failure_preserves_evidence(self):
        self.run_fault('assertion')

    def test_03_timeout_stops_real_renderer(self):
        self.run_fault('timeout')

    def test_04_interrupt_stops_real_renderer(self):
        self.run_fault('interrupt')


if __name__ == '__main__':
    unittest.main(failfast=True)
