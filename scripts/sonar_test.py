import importlib.util
import json
import os
import sys
from unittest import mock
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('sonar', Path(__file__).with_name('sonar.py'))
sonar = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sonar)


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.email', 'test@example.invalid')
        self.git('config', 'user.name', 'Test')
        self.write('apps/desktop/src/main.ts', 'export const answer = 1;\n')
        self.write('sonar-project.properties', 'sonar.sources=apps/desktop/src\n')
        self.git('add', '.')
        self.git('commit', '-qm', 'baseline')

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.root), *args], text=True).strip()

    def write(self, name, content):
        target = self.root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def test_snapshot_includes_edits_and_untracked_source_but_not_credentials(self):
        self.write('apps/desktop/src/main.ts', 'export const answer = 2;\n')
        self.write('apps/desktop/src/new.ts', 'export const extra = 3;\n')
        self.write('apps/desktop/.env', 'SECRET=private\n')
        self.write('apps/desktop/patches/dependency.patch', 'dependency correction\n')
        destination = self.root / 'snapshot'
        info = sonar.snapshot(self.root, destination)
        self.assertEqual((destination / 'apps/desktop/src/main.ts').read_text(), 'export const answer = 2;\n')
        self.assertTrue((destination / 'apps/desktop/src/new.ts').exists())
        self.assertFalse((destination / 'apps/desktop/.env').exists())
        self.assertTrue((destination / 'apps/desktop/patches/dependency.patch').exists())
        self.assertTrue(info['dirty'])

    def test_fingerprint_changes_when_source_changes_and_ignores_generated_files(self):
        before = sonar.snapshot(self.root, self.root / 'first')['fingerprint']
        self.write('apps/desktop/node_modules/generated.ts', 'generated\n')
        unchanged = sonar.snapshot(self.root, self.root / 'second')['fingerprint']
        self.assertEqual(before, unchanged)
        self.write('apps/desktop/src/main.ts', 'export const answer = 2;\n')
        self.assertNotEqual(before, sonar.snapshot(self.root, self.root / 'third')['fingerprint'])

    def test_deleted_source_is_absent(self):
        (self.root / 'apps/desktop/src/main.ts').unlink()
        self.assertFalse((self.root / 'snapshot/apps/desktop/src/main.ts').exists())
        info = sonar.snapshot(self.root, self.root / 'snapshot')
        self.assertEqual(info['sourceFiles'], 0)

    def test_source_symlink_cannot_read_outside_repo(self):
        try:
            (self.root / 'apps/desktop/src/leak.ts').symlink_to(self.root / 'sonar-project.properties')
        except OSError as error:
            self.skipTest('Symlink creation unavailable: ' + str(error))
        with self.assertRaisesRegex(ValueError, 'symlink'):
            sonar.snapshot(self.root, self.root / 'snapshot')


class PortabilityTests(unittest.TestCase):
    def test_generated_password_meets_default_server_policy(self):
        password = sonar.new_password()
        self.assertGreaterEqual(len(password), 12)
        self.assertTrue(any(c.isupper() for c in password))
        self.assertTrue(any(c.islower() for c in password))
        self.assertTrue(any(c.isdigit() for c in password))
        self.assertTrue(any(not c.isalnum() for c in password))

    def test_scan_lock_contends_across_processes_and_releases_after_error(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'scan.lock'
            script = ('import sys; sys.path.insert(0, sys.argv[1]); import sonar; '
                      'lock = sonar.scan_lock(__import__("pathlib").Path(sys.argv[2])); '
                      'lock.__enter__()')
            command = [sys.executable, '-c', script, str(Path(__file__).parent), str(path)]
            with self.assertRaisesRegex(RuntimeError, 'release'):
                with sonar.scan_lock(path):
                    blocked = subprocess.run(command, capture_output=True, text=True)
                    self.assertNotEqual(blocked.returncode, 0)
                    self.assertIn('already owns', blocked.stderr)
                    raise RuntimeError('release')
            acquired = subprocess.run(command, capture_output=True, text=True)
            self.assertEqual(acquired.returncode, 0, acquired.stderr)

    def test_private_settings_roundtrip_uses_utf8(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'sonar.env'
            with mock.patch.dict(os.environ, {'SONAR_ENV_FILE': str(path)}):
                settings = {'SONAR_ADMIN_PASSWORD': 'abc123', 'SONAR_DB_PASSWORD': 'xyz456'}
                sonar.write_settings(path, settings)
                self.assertEqual(sonar.load_settings(), settings)
                if os.name != 'nt':
                    self.assertEqual(path.stat().st_mode & 0o777, 0o600)
                else:
                    acl = subprocess.check_output(['icacls', str(path)], text=True, encoding='oem')
                    self.assertNotIn('(I)', acl)
                    grants = [line for line in acl.splitlines() if '(F)' in line]
                    self.assertEqual(len(grants), 1, acl)

    def test_windows_default_settings_live_in_local_appdata(self):
        with mock.patch.dict(
                os.environ, {'LOCALAPPDATA': '/user/private'}, clear=True):
            self.assertEqual(str(sonar.default_settings_path(windows=True)).replace('\\', '/'),
                             '/user/private/OrkWorks/sonar.env')


class LifecycleTests(unittest.TestCase):
    def test_compose_uses_saved_credentials_over_shell_overrides(self):
        saved = {'SONAR_DB_PASSWORD': 'stored-password', 'SONAR_TOKEN': 'stored-token'}
        with mock.patch.object(sonar, 'load_settings', return_value=saved), mock.patch.object(
                sonar, 'run') as command, mock.patch.dict(os.environ, {'SONAR_DB_PASSWORD': 'shell'}):
            sonar.compose('up', env={'SONAR_TOKEN': 'another-shell-token'})
        environment = command.call_args.kwargs['env']
        self.assertEqual(environment['SONAR_DB_PASSWORD'], 'stored-password')
        self.assertEqual(environment['SONAR_TOKEN'], 'stored-token')

    def test_up_owns_host_lock_through_initialization(self):
        with tempfile.TemporaryDirectory() as temporary:
            private = Path(temporary) / 'sonar.env'
            with mock.patch.object(sonar, 'default_settings_path', return_value=private), mock.patch.object(
                    sonar, 'initialize') as initialize:
                def attempt_competing_initializer():
                    with self.assertRaisesRegex(ValueError, 'initialization'):
                        sonar.up()
                initialize.side_effect = attempt_competing_initializer
                sonar.up()
                initialize.assert_called_once()

    def test_saved_invalid_token_is_reprovisioned(self):
        saved = {'SONAR_ADMIN_PASSWORD': 'saved-password', 'SONAR_TOKEN': 'revoked'}
        api = mock.Mock()
        def request(endpoint, params=None, post=False):
            if endpoint == 'api/authentication/validate':
                return {'valid': False}
            if endpoint == 'api/user_tokens/generate':
                return {'token': 'replacement'}
            self.fail('Unexpected API call: ' + endpoint)
        api.request.side_effect = request
        with mock.patch.object(sonar, 'load_settings', return_value=saved), mock.patch.object(
                sonar, 'compose'), mock.patch.object(sonar, 'wait_for'), mock.patch.object(
                sonar, 'Api', return_value=api), mock.patch.object(sonar, 'write_settings') as write:
            sonar.initialize()
        self.assertEqual(saved['SONAR_TOKEN'], 'replacement')
        write.assert_called_once()

    def test_baseline_cannot_be_overwritten_without_explicit_flag(self):
        with tempfile.TemporaryDirectory() as temporary, mock.patch.object(
                sonar, 'ROOT', Path(temporary)), mock.patch('builtins.print'):
            sonar.save_report({'metrics': {'ncloc': 100}}, 'baseline')
            path = Path(temporary) / '.sonar/reports/baseline.json'
            with self.assertRaises(FileExistsError):
                sonar.save_report({'metrics': {'ncloc': 80}}, 'baseline')
            self.assertEqual(json.loads(path.read_text())['metrics']['ncloc'], 100)
            sonar.save_report({'metrics': {'ncloc': 80}}, 'baseline', overwrite=True)
            self.assertEqual(json.loads(path.read_text())['metrics']['ncloc'], 80)


class ReportTests(unittest.TestCase):
    def test_configuration_rejects_server_settings_changed_since_scan(self):
        with self.assertRaisesRegex(ValueError, 'configuration'):
            sonar.verify_configuration({'effectiveSettings': 'before'},
                                       {'effectiveSettings': 'after'})

    def test_missing_configuration_cannot_refresh_an_old_report(self):
        with self.assertRaisesRegex(ValueError, 'configuration'):
            sonar.verify_configuration(None, {'effectiveSettings': 'current'})

    def test_effective_settings_change_identity_without_publishing_values(self):
        settings = [{'key': 'sonar.rust.clippy.enabled', 'value': 'true'},
                    {'key': 'sonar.core.startTime', 'value': 'first'}]
        responses = {
            'api/qualityprofiles/search': {'profiles': [{'key': 'profile', 'language': 'rust', 'rulesUpdatedAt': 'date'}]},
            'api/plugins/installed': {'plugins': [{'key': 'rust', 'version': '1'}]},
            'api/settings/values': {'settings': settings},
            'api/qualitygates/get_by_project': {'qualityGate': {'name': 'Gate'}},
            'api/qualitygates/show': {'conditions': []},
            'api/system/status': {'version': '26.9'},
        }
        api = mock.Mock()
        api.request.side_effect = lambda endpoint, params=None: (
            {'settings': []} if endpoint == 'api/settings/values' and params
            else responses[endpoint])
        before = sonar.analysis_identity(api, 'project', 'scope')
        sonar.verify_configuration(json.loads(json.dumps(before)), before)
        settings[1]['value'] = 'restart'
        self.assertEqual(before, sonar.analysis_identity(api, 'project', 'scope'))
        settings[0]['value'] = 'false'
        after = sonar.analysis_identity(api, 'project', 'scope')
        self.assertNotEqual(before['effectiveSettings'], after['effectiveSettings'])
        self.assertNotIn('clippy.enabled', json.dumps(after))
        with self.assertRaisesRegex(ValueError, 'configuration'):
            sonar.verify_configuration(before, after)

    def test_pagination_collects_all_pages_and_rejects_incomplete_inventory(self):
        api = sonar.Api({'SONAR_TOKEN': 'test-only'})
        with mock.patch.object(api, 'request', side_effect=[
            {'issues': ['a'], 'paging': {'total': 2}},
            {'issues': ['b'], 'paging': {'total': 2}}]):
            self.assertEqual(api.pages('issues', 'issues', {}), ['a', 'b'])
        for payload in ({'issues': [], 'total': 1},
                        {'issues': [], 'total': 10001}, {'issues': []}):
            with mock.patch.object(api, 'request', return_value=payload):
                with self.assertRaisesRegex(ValueError, 'pagination'):
                    api.pages('issues', 'issues', {})

    def test_missing_metric_stays_unknown(self):
        result = sonar.metric_values([{'metric': 'ncloc', 'value': '42'}])
        self.assertEqual(result['ncloc'], 42)
        self.assertIsNone(result['cognitive_complexity'])

    def test_comparison_rejects_changed_scope(self):
        before = {'configuration': 'a', 'metrics': {'ncloc': 100}}
        after = {'configuration': 'b', 'metrics': {'ncloc': 80}}
        with self.assertRaisesRegex(ValueError, 'configuration'):
            sonar.compare(before, after)

    def test_comparison_keeps_unknowns_and_reports_measured_deltas(self):
        before = {'configuration': 'a', 'metrics': {'ncloc': 100}}
        after = {'configuration': 'a', 'metrics': {'ncloc': 80, 'complexity': 5}}
        result = sonar.compare(before, after)
        self.assertEqual(result['ncloc'], {'before': 100, 'after': 80, 'delta': -20})
        self.assertIsNone(result['complexity']['delta'])

    def test_report_rejects_another_analysis_replacing_this_snapshot(self):
        with self.assertRaisesRegex(ValueError, 'analysis'):
            sonar.verify_analysis('expected', {'analyses': [{'key': 'other'}]})

    def test_failed_background_task_is_not_success(self):
        with self.assertRaisesRegex(ValueError, 'FAILED'):
            sonar.completed_task({'task': {'status': 'FAILED', 'errorMessage': 'parse failure'}}, 'project')

    def test_completed_task_is_bound_to_the_project(self):
        with self.assertRaisesRegex(ValueError, 'project'):
            sonar.completed_task({'task': {'status': 'SUCCESS', 'analysisId': 'abc', 'componentKey': 'other'}}, 'project')

    def test_worktree_projects_have_distinct_identity(self):
        self.assertNotEqual(sonar.project_key(Path('/repo')), sonar.project_key(Path('/repo-task')))


if __name__ == '__main__':
    unittest.main()
