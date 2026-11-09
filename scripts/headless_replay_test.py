#!/usr/bin/env python3
"""Behavioral tests for the fixed headless replay command, without a container daemon."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
CID = 'a' * 64
FAKE = r'''#!/usr/bin/env python3
import io,json,os,pathlib,sys,tarfile,time
args=sys.argv[1:]; mode=os.environ.get('FAKE_MODE','ok'); state=pathlib.Path(os.environ['FAKE_STATE'])
with (state/'calls').open('a') as f: f.write(json.dumps(args)+'\n')
cmd=args[0]
if cmd=='info': print(json.dumps({'host':{'arch':'amd64','security':{'rootless':True}}}));sys.exit(0)
if cmd=='image': print(json.dumps(['HOME=/tmp/orkworks-home']));sys.exit(0)
if cmd=='container':
 if '{{json .Mounts}}' in args:
  print(json.dumps([{'Type':'bind'}] if mode=='implicit-mount' else []));sys.exit(0)
 values=['HOME=/tmp/orkworks-home','container=podman']
 if mode=='ambient-env': values.append('AWS_SECRET_ACCESS_KEY=private-qualification-sentinel')
 if mode=='proxy-injected': values.append('HTTP_PROXY=private-qualification-sentinel')
 print(json.dumps(values));sys.exit(0)
if cmd=='build':
 if mode=='build-fail': sys.exit(7)
 if mode=='build-timeout': time.sleep(10)
 if mode=='orphaned-cli':
  import subprocess
  subprocess.Popen([sys.executable,'-c',"import pathlib,time; time.sleep(2.0); pathlib.Path("+repr(str(state/'orphan-survived'))+").touch()"])
  (state/'orphan-started').touch()
  sys.exit(0)
 pathlib.Path(args[args.index('--iidfile')+1]).write_text('b'*64)
 context=pathlib.Path(args[-1]);(state/'context').write_text(json.dumps([str(p.relative_to(context)) for p in context.rglob('*') if p.is_file()]))
if cmd=='create':
 if mode=='create-fail': sys.exit(8)
 if mode!='create-no-cid': pathlib.Path(args[args.index('--cidfile')+1]).write_text('a'*64)
 if mode=='bad-cid': pathlib.Path(args[args.index('--cidfile')+1]).write_text('../foreign')
 if mode=='create-timeout': time.sleep(10)
if cmd=='start':
 (state/'started').touch()
 if mode in ['timeout','interrupt']: time.sleep(10)
 if mode=='flood': sys.stdout.write('x'*(3*1024*1024))
 if mode=='test-fail': sys.exit(1)
if cmd=='kill': (state/'killed').touch()
if cmd=='inspect':
 if mode=='unreachable' or (mode=='loses-access' and (state/'inspected').exists()): sys.exit(9)
 (state/'inspected').touch()
 print(json.dumps({'Running':mode=='needs-kill' and not (state/'killed').exists(),'ExitCode':1 if mode=='test-fail' else 0}))
if cmd=='cp':
 if mode=='export-fail': sys.exit(10)
 name=args[-2].rsplit('/',1)[-1]
 if name.startswith('failure.'): sys.exit(1)
 stream=io.BytesIO()
 with tarfile.open(fileobj=stream,mode='w') as archive:
  info=tarfile.TarInfo('../escape' if mode=='traversal' else name)
  data=b'fixture'
  if mode=='oversize':data=b'x'*(9*1024*1024)
  info.size=len(data)
  if mode=='symlink':info.type=tarfile.SYMTYPE;info.linkname='/tmp/foreign';info.size=0
  archive.addfile(info,io.BytesIO(data) if info.isfile() else None)
 sys.stdout.buffer.write(stream.getvalue())
if cmd in ['stop','kill','rm'] and mode in ['unreachable','loses-access']:sys.exit(9)
'''


class ReplayTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base/'repo'; self.root.mkdir()
        files = ['Containerfile', 'apps/desktop/tsconfig.json', 'apps/desktop/package.json', 'apps/desktop/pnpm-lock.yaml',
                 'apps/desktop/pnpm-workspace.yaml', 'apps/desktop/scripts/setup-electron.mjs',
                 'apps/desktop/tests/applicationShell.test.mjs', 'apps/desktop/src/App.tsx',
                 'apps/desktop/src/.env', '.env']
        for name in files:
            p=self.root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text('sentinel' if '.env' in name else 'fixture')
        subprocess.run(['git','init','-q',str(self.root)],check=True)
        subprocess.run(['git','-C',str(self.root),'add','.'],check=True)
        self.bin=self.base/'bin';self.bin.mkdir();engine=self.bin/'podman';engine.write_text(FAKE);engine.chmod(0o755)
        self.state=self.base/'state';self.state.mkdir()
        self.env=patch.dict(os.environ,{'PATH':str(self.bin)+os.pathsep+os.environ['PATH'],'FAKE_STATE':str(self.state),'FAKE_MODE':'ok'})
        self.env.start();self.addCleanup(self.env.stop)
        spec=importlib.util.spec_from_file_location('headless_replay',HERE/'headless_replay.py')
        self.runner=importlib.util.module_from_spec(spec) if spec and Path(spec.origin).exists() else None
        if self.runner:spec.loader.exec_module(self.runner)
        self.budgets={'build':2,'create':1,'run':1,'export':3,'cleanup':2}

    def run_fixture(self,mode='ok'):
        self.assertIsNotNone(self.runner,'headless replay implementation is missing')
        with patch.dict(os.environ,{'FAKE_MODE':mode}),contextlib.redirect_stdout(io.StringIO()):
            code,out=self.runner.run('podman',self.root,self.base,self.budgets)
        self.out=out;self.receipt=json.loads((out/'result.json').read_text());return code

    def calls(self):
        return [json.loads(line) for line in (self.state/'calls').read_text().splitlines()] if (self.state/'calls').exists() else []

    def test_success_uses_fixed_isolation_and_cleans_exact_cid(self):
        self.assertEqual(self.run_fixture(),0)
        calls=self.calls();create=next(c for c in calls if c[0]=='create')
        for value in ['--network=none','--cap-drop=ALL','--security-opt=no-new-privileges','--user=1000:1000','--init','--memory=2g','--pids-limit=256','--shm-size=256m']:
            self.assertIn(value,create)
        self.assertIn('--http-proxy=false',create)
        self.assertIn('HTTP_PROXY=',create)
        self.assertIn('https_proxy=',create)
        self.assertFalse(any(x in create for x in ['--privileged','--volume','-v','--mount']))
        self.assertIn(['rm',CID],calls)
        self.assertEqual(self.receipt['cleanup'],'removed')
        exported=list((self.out/'replay').iterdir());self.assertEqual(len(exported),11)
        self.assertTrue(all(p.stat().st_uid==os.getuid() and p.stat().st_gid==os.getgid() for p in exported))
        context=json.loads((self.state/'context').read_text())
        self.assertIn('apps/desktop/src/App.tsx',context)
        self.assertIn('apps/desktop/tsconfig.json',context)
        self.assertFalse(any('.env' in p or '.git' in p for p in context))
        self.assertFalse((self.out/'context').exists(),'private build context is removed')

    def test_input_symlink_never_reaches_daemon(self):
        target=self.root/'apps/desktop/src/App.tsx';target.unlink();target.symlink_to(self.base/'secret')
        (self.base/'secret').write_text('private')
        self.assertNotEqual(self.run_fixture(),0)
        self.assertFalse(any(c[0]=='build' for c in self.calls()))

    def test_unavailable_engine_never_starts(self):
        (self.bin/'podman').unlink()
        with patch('shutil.which',return_value=None):
            self.assertNotEqual(self.run_fixture(),0)
        self.assertFalse(self.calls())

    def test_setup_failures_do_not_delete_guessed_containers(self):
        for mode in ['build-fail','create-fail','create-no-cid','bad-cid']:
            with self.subTest(mode=mode):
                (self.state/'calls').unlink(missing_ok=True)
                self.assertNotEqual(self.run_fixture(mode),0)
                self.assertFalse(any(c[0] in ['stop','kill','rm'] for c in self.calls()))

    def test_build_timeout_is_bounded(self):
        self.budgets['build']=1
        start=time.monotonic();self.assertNotEqual(self.run_fixture('build-timeout'),0)
        self.assertTrue(any(c[0]=='build' for c in self.calls()))
        self.assertLess(time.monotonic()-start,3)

    def test_cli_timeout_also_stops_its_inherited_pipe_child(self):
        self.assertIsNotNone(self.runner)
        log=self.runner.Log(self.base/'orphan.log')
        try:
            with patch.dict(os.environ,{'FAKE_MODE':'orphaned-cli'}):
                with self.assertRaises(self.runner.Failure):
                    self.runner.command(['podman','build'],time.monotonic()+1,log)
        finally:
            log.close()
        self.assertTrue((self.state/'orphan-started').exists())
        time.sleep(2.1)
        self.assertFalse((self.state/'orphan-survived').exists(), 'owned CLI descendant escaped its timeout')

    def test_create_timeout_uses_written_cid_for_cleanup(self):
        self.budgets['create']=0.5
        self.assertNotEqual(self.run_fixture('create-timeout'),0)
        self.assertIn(['rm',CID],self.calls())

    def test_test_failure_exports_evidence_and_removes_container(self):
        self.assertNotEqual(self.run_fixture('test-fail'),0)
        self.assertTrue((self.out/'replay/visible-wide.png').exists())
        self.assertIn(['rm',CID],self.calls())

    def test_run_timeout_stops_exact_container(self):
        self.budgets['run']=0.5
        self.assertNotEqual(self.run_fixture('timeout'),0)
        calls=self.calls();self.assertTrue(any(c[0]=='stop' and c[-1]==CID for c in calls))
        self.assertIn(['rm',CID],calls)

    def test_stop_falls_back_to_exact_cid_kill(self):
        self.assertNotEqual(self.run_fixture('needs-kill'),0)
        self.assertIn(['kill',CID],self.calls())
        self.assertIn(['rm',CID],self.calls())

    def test_interrupt_cli_keeps_evidence_and_cleans_owned_container(self):
        with patch.dict(os.environ,{'FAKE_MODE':'interrupt','TMPDIR':str(self.base)}):
            process=subprocess.Popen(['bash',str(HERE/'headless-replay.sh'),'--engine','podman'],
                                     stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        def reap():
            if process.poll() is None:
                process.kill()
            process.communicate(timeout=2)
        self.addCleanup(reap)
        deadline=time.monotonic()+5
        while not (self.state/'started').exists() and time.monotonic()<deadline:
            time.sleep(.02)
        self.assertTrue((self.state/'started').exists())
        process.send_signal(signal.SIGTERM)
        stdout,stderr=process.communicate(timeout=5)
        self.assertNotEqual(process.returncode,0,stderr.decode())
        directory=Path(next(line.removeprefix('Artifacts: ') for line in stdout.decode().splitlines() if line.startswith('Artifacts: ')))
        receipt=json.loads((directory/'result.json').read_text())
        self.assertEqual(receipt['reason'],'interrupted')
        self.assertEqual(receipt['cleanup'],'removed')
        self.assertIn(['rm',CID],self.calls())

    def test_public_entry_rejects_command_passthrough(self):
        result=subprocess.run(['bash',str(HERE/'headless-replay.sh'),'--engine','podman','sh','-c','echo unsafe'],capture_output=True)
        self.assertEqual(result.returncode,2)
        self.assertFalse(self.calls())

    def test_output_is_drained_but_retained_log_is_bounded(self):
        self.assertEqual(self.run_fixture('flood'),0)
        data=(self.out/'run.log').read_bytes();self.assertLessEqual(len(data),2*1024*1024)
        self.assertIn(b'[output truncated]',data)

    def test_invalid_or_unbounded_artifacts_are_not_written(self):
        for mode in ['symlink','traversal','oversize','export-fail']:
            with self.subTest(mode=mode):
                self.assertNotEqual(self.run_fixture(mode),0)
                self.assertFalse((self.out/'replay/visible-wide.png').exists())
                self.assertIn(['rm',CID],self.calls())
        self.assertFalse((self.base/'escape').exists())

    def test_unreachable_cleanup_is_reported_not_claimed(self):
        self.assertNotEqual(self.run_fixture('unreachable'),0)
        self.assertNotEqual(self.receipt['cleanup'],'removed')
        self.assertEqual(self.receipt['container_id'],CID)
        self.assertTrue((self.out/'cid').exists())

    def test_later_engine_failure_preserves_last_confirmed_state(self):
        self.assertNotEqual(self.run_fixture('loses-access'),0)
        self.assertEqual(self.receipt['last_confirmed_state']['Running'],False)
        self.assertEqual(self.receipt['last_confirmed_state']['ExitCode'],0)
        self.assertIn('container-created',self.receipt['milestones'])
        self.assertIn('container-state-confirmed',self.receipt['milestones'])
        self.assertNotIn('container-removed',self.receipt['milestones'])
        self.assertNotEqual(self.receipt['cleanup'],'removed')

    def test_implicit_environment_or_mount_is_refused_before_execution_without_leaking_values(self):
        for mode in ['ambient-env','proxy-injected','implicit-mount']:
            with self.subTest(mode=mode):
                (self.state/'calls').unlink(missing_ok=True)
                self.assertNotEqual(self.run_fixture(mode),0)
                self.assertFalse(any(c[0]=='start' for c in self.calls()))
                self.assertIn(['rm',CID],self.calls())
                self.assertNotIn('private-qualification-sentinel',(self.out/'run.log').read_text())
                self.assertNotIn('private-qualification-sentinel',(self.out/'result.json').read_text())

    def test_wrong_export_ownership_is_a_failure(self):
        self.assertIsNotNone(self.runner)
        with patch.object(self.runner,'verify_owner',side_effect=RuntimeError('wrong owner')):
            self.assertNotEqual(self.run_fixture(),0)
        self.assertIn(['rm',CID],self.calls())

if __name__=='__main__':unittest.main()
