"""Model-free fixture tests: passing is not native/production qualification."""
import dataclasses
import hashlib
import json
from pathlib import Path
import unittest

from peer import CONFIG, Binding, FixtureError, Peer, RunCursor


class ScriptedClient:
    """Direct calls only; snapshots, not notification text, drive acknowledgements."""
    def __init__(self, peer):
        self.peer = peer
        self.binding = peer.binding
        self.cursor = RunCursor(0, peer.instance_id)

    def wait(self, timeout=30):
        return self.peer.wait(self.binding, self.cursor, timeout)

    def consume(self, request):
        result = self.peer.result(request)
        if result is None or result["status"] not in ("events", "refresh_required"):
            return result
        previous_cursor = self.cursor.value
        snapshot = self.peer.read(self.binding)
        self.cursor = snapshot.cursor
        if result["status"] == "events":
            for event in result["events"]:
                if event["cursor"] <= previous_cursor:
                    continue
                self.peer.acknowledge(self.binding, snapshot, f"event-{event['cursor']}")
        return result


def scripted_transcript():
    peer = Peer()
    client = ScriptedClient(peer)
    request = client.wait()
    transcript = [{"step": "waiting", "pending": peer.result(request) is None}]
    peer.emit("report", "report-1")
    transcript.append({"step": "report", "receipt": client.consume(request)})
    client.consume(request)
    transcript.append({"step": "duplicate", "effects": peer.effect_count})
    for n in range(257):
        peer.emit("capacity", f"slot-{n}")
    transcript.append({"step": "gap", "receipt": client.consume(client.wait()),
                       "adoptedCursor": client.cursor.value, "effects": peer.effect_count})
    peer.approve_plan(peer.read(peer.binding).version)
    peer.reserve(peer.binding, peer.read(peer.binding), "task-a")
    client.cursor = peer.read(peer.binding).cursor
    request = client.wait()
    old = peer.binding
    peer.end_runtime()
    transcript.append({"step": "runtime-ended", "receipt": peer.result(request)})
    fresh = peer.resume(old.native_id, old.workspace, old.configuration_digest)
    snapshot = peer.read(fresh)
    # JSON representation is only for this synthetic evidence packet.
    transcript.append({"step": "resumed", "snapshot": json.loads(json.dumps(dataclasses.asdict(snapshot)))})
    request = peer.wait(fresh, snapshot.cursor)
    peer.close("complete")
    transcript.append({"step": "closed", "receipt": peer.result(request)})
    return transcript


class PeerTests(unittest.TestCase):
    def test_pinned_configuration_and_source_digests(self):
        root = Path(__file__).parent
        evidence = json.loads((root / "evidence.json").read_text())
        self.assertTrue(evidence["fixtureOnly"])
        self.assertEqual(evidence["configuration"], CONFIG)
        canonical = json.dumps(evidence["configuration"], sort_keys=True, separators=(",", ":")).encode()
        self.assertEqual(hashlib.sha256(canonical).hexdigest(), evidence["configurationDigest"])
        for name, expected in evidence["codeSha256"].items():
            self.assertEqual(hashlib.sha256((root / name).read_bytes()).hexdigest(), expected, name)
        self.assertEqual(evidence["nativeCapabilities"], {
            "sameParentDelivery": "unverified", "gapRecovery": "unverified", "exactResume": "unverified"})

    def test_scripted_transcript_matches_independent_receipts(self):
        evidence = json.loads((Path(__file__).parent / "evidence.json").read_text())
        self.assertEqual(scripted_transcript(), evidence["expectedTranscript"])

    def test_wait_stays_pending_until_an_event_or_deadline(self):
        peer = Peer()
        request = peer.wait(peer.binding)
        self.assertIsNone(peer.result(request))
        peer.advance(29)
        self.assertIsNone(peer.result(request))
        peer.emit("report", "report-1")
        result = peer.result(request)
        self.assertEqual(result["status"], "events")
        self.assertEqual(result["requestId"], "wait-1")
        self.assertEqual(result["binding"], dataclasses.asdict(peer.binding))
        self.assertEqual(result["events"], [{"cursor": 1, "kind": "report", "reference": "report-1"}])

    def test_timeout_uses_virtual_time_and_never_invents_an_event(self):
        peer = Peer()
        request = peer.wait(peer.binding)
        peer.advance(30)
        peer.emit("report", "too-late-for-this-wait")
        self.assertEqual(peer.result(request)["status"], "no_change")
        self.assertEqual(peer.result(request)["cursor"], 0)
        self.assertEqual(peer.effect_count, 0)
        self.assertEqual(peer.result(peer.wait(peer.binding))["events"][0]["cursor"], 1)
        for seconds in [-1, 31, True, 0.5, float("nan")]:
            with self.subTest(seconds=seconds), self.assertRaises(FixtureError):
                peer.wait(peer.binding, timeout=seconds)
        with self.assertRaises(FixtureError):
            peer.advance(-1)
        self.assertEqual(peer.result(peer.wait(peer.binding, RunCursor(1), 0))["status"], "no_change")

    def test_only_one_outstanding_wait_for_the_fake_bearer(self):
        peer = Peer()
        request = peer.wait(peer.binding)
        with self.assertRaises(FixtureError):
            peer.wait(peer.binding)
        peer.advance(30)
        self.assertEqual(peer.result(request)["status"], "no_change")
        self.assertIsNone(peer.result(peer.wait(peer.binding)))
        with self.assertRaises(FixtureError):
            Peer().result(request)

    def test_notifications_require_current_state_reads_for_effects(self):
        peer = Peer()
        client = ScriptedClient(peer)
        for n, kind in enumerate(["report", "approval", "blocker", "capacity"], 1):
            request = client.wait()
            peer.emit(kind, f"ref-{n}")
            self.assertEqual(peer.effect_count, n - 1)
            self.assertEqual(client.consume(request)["status"], "events")
            snapshot = peer.read(peer.binding)
            self.assertFalse(snapshot.grant)
            self.assertEqual(snapshot.reserved, ())
            self.assertEqual(peer.effect_count, n)
            # Re-delivery/read of the same metadata cannot produce another effect.
            self.assertEqual(client.consume(request)["status"], "events")
            self.assertEqual(peer.effect_count, n)

    def test_old_duplicate_notification_after_new_state_has_no_new_effect(self):
        peer = Peer()
        client = ScriptedClient(peer)
        first = client.wait()
        peer.emit("report", "first")
        client.consume(first)
        second = client.wait()
        peer.emit("capacity", "second")
        client.consume(second)
        try:
            self.assertEqual(client.consume(first)["status"], "events")
        except FixtureError:
            self.fail("old duplicate notification must not become a conflicting new acknowledgement")
        self.assertEqual(peer.effect_count, 2)

    def test_stale_forged_and_conflicting_acknowledgements_are_denied(self):
        peer = Peer()
        old = peer.read(peer.binding)
        peer.emit("report", "new")
        with self.assertRaises(FixtureError):
            peer.acknowledge(peer.binding, old, "stale")
        snapshot = peer.read(peer.binding)
        with self.assertRaises(FixtureError):
            peer.acknowledge(peer.binding, dataclasses.replace(snapshot), "forged")
        receipt = peer.acknowledge(peer.binding, snapshot, "ack-1")
        peer.emit("capacity", "newer")
        self.assertEqual(peer.acknowledge(peer.binding, snapshot, "ack-1"), receipt)
        with self.assertRaises(FixtureError):
            peer.acknowledge(peer.binding, peer.read(peer.binding), "ack-1")
        self.assertEqual(peer.effect_count, 1)

    def test_gap_uses_snapshot_cursor_without_replaying_omitted_events(self):
        peer = Peer()
        client = ScriptedClient(peer)
        for n in range(256):
            peer.emit("capacity", f"slot-{n}")
        retained = peer.result(client.wait())
        self.assertEqual(len(retained["events"]), 256)
        peer.emit("capacity", "overflow")
        result = client.consume(client.wait())
        self.assertEqual(result["status"], "refresh_required")
        self.assertNotIn("events", result)
        self.assertEqual(result["runVersion"], 258)
        self.assertEqual(client.cursor, RunCursor(257))
        self.assertEqual(peer.effect_count, 0)
        pending = client.wait()
        self.assertIsNone(peer.result(pending))
        peer.emit("report", "after-snapshot")
        self.assertEqual(client.consume(pending)["events"][0]["cursor"], 258)
        self.assertEqual(peer.effect_count, 1)

    def test_cursor_namespaces_and_bounds_are_not_coerced(self):
        peer = Peer()
        for cursor in ["native-log-cursor", 0, RunCursor(True), RunCursor(-1),
                       RunCursor(1), RunCursor(0, "different-fixture")]:
            with self.subTest(cursor=cursor), self.assertRaises(FixtureError):
                peer.wait(peer.binding, cursor)

    def test_revocation_resolves_only_the_existing_wait_without_authorizing_reads(self):
        peer = Peer()
        old = peer.binding
        request = peer.wait(old)
        peer.end_runtime()
        self.assertEqual(peer.result(request)["status"], "revoked")
        self.assertEqual(peer.result(request)["binding"], dataclasses.asdict(old))
        with self.assertRaises(FixtureError):
            peer.read(old)
        with self.assertRaises(FixtureError):
            peer.wait(old)

    def test_closed_run_wakes_wait_and_remains_terminal(self):
        for state in ["complete", "cancelled"]:
            with self.subTest(state=state):
                peer = Peer()
                old = peer.binding
                request = peer.wait(old)
                peer.close(state)
                self.assertEqual(peer.result(request)["status"], "run_closed")
                with self.assertRaises(FixtureError):
                    peer.read(old)
                with self.assertRaises(FixtureError):
                    peer.resume(old.native_id, old.workspace, old.configuration_digest)
                with self.assertRaises(FixtureError):
                    peer.approve_plan(1)

    def test_binding_from_another_identically_seeded_peer_is_not_authority(self):
        peer = Peer()
        other = Peer()
        self.assertEqual(peer.binding, other.binding)
        with self.assertRaises(FixtureError):
            peer.read(other.binding)
        with self.assertRaises(FixtureError):
            peer.read(dataclasses.replace(peer.binding))

    def test_wrong_binding_is_rejected_before_wait_read_and_effect(self):
        peer = Peer()
        snapshot = peer.read(peer.binding)
        for field in dataclasses.fields(Binding):
            wrong = dataclasses.replace(peer.binding, **{field.name: "wrong"})
            with self.subTest(field=field.name):
                for call in [lambda: peer.wait(wrong), lambda: peer.read(wrong),
                             lambda: peer.acknowledge(wrong, snapshot, "ack")]:
                    with self.assertRaises(FixtureError):
                        call()
        self.assertEqual(peer.effect_count, 0)

    def test_exact_resume_requires_end_and_rejects_fallback_identities(self):
        peer = Peer()
        old = peer.binding
        with self.assertRaises(FixtureError):
            peer.resume(old.native_id, old.workspace, old.configuration_digest)
        peer.end_runtime()
        for args in [(None, old.workspace, old.configuration_digest),
                     ("latest", old.workspace, old.configuration_digest),
                     (old.native_id, "elsewhere", old.configuration_digest),
                     (old.native_id, old.workspace, "changed")]:
            with self.subTest(args=args), self.assertRaises(FixtureError):
                peer.resume(*args)
        fresh = peer.resume(old.native_id, old.workspace, old.configuration_digest)
        self.assertEqual(fresh.native_id, old.native_id)
        self.assertNotEqual(fresh.generation, old.generation)
        self.assertNotEqual(fresh.bearer, old.bearer)
        self.assertEqual(len(fresh.generation), 64)
        with self.assertRaises(FixtureError):
            peer.read(old)
        self.assertFalse(peer.read(fresh).grant)

    def test_resume_preserves_independent_gates_in_priority_order(self):
        for stage, blockers, clarification, expected in [
            (True, ("failure",), "question-1", "paused"),
            (False, ("failure",), "question-1", "blocked"),
            (False, (), "question-1", "awaiting_clarification"),
            (False, (), None, "synthesizing"),
        ]:
            with self.subTest(expected=expected):
                peer = Peer()
                peer.set_gates(stage, blockers, clarification)
                old = peer.binding
                peer.end_runtime()
                fresh = peer.resume(old.native_id, old.workspace, old.configuration_digest)
                snapshot = peer.read(fresh)
                self.assertEqual((snapshot.stage_continue, snapshot.blockers, snapshot.clarification),
                                 (stage, blockers, clarification))
                self.assertEqual(snapshot.state, expected)
                self.assertFalse(snapshot.grant)
                if stage or blockers or clarification:
                    with self.assertRaises(FixtureError):
                        peer.approve_plan(snapshot.version)

    def test_resume_never_restores_grants_or_retries_reserved_tasks(self):
        peer = Peer()
        peer.approve_plan(peer.read(peer.binding).version)
        peer.reserve(peer.binding, peer.read(peer.binding), "task-a")
        old = peer.binding
        peer.end_runtime()
        fresh = peer.resume(old.native_id, old.workspace, old.configuration_digest)
        snapshot = peer.read(fresh)
        self.assertFalse(snapshot.grant)
        self.assertEqual(snapshot.state, "awaiting_resume_approval")
        self.assertEqual(snapshot.reserved, ("task-a",))
        with self.assertRaises(FixtureError):
            peer.reserve(fresh, snapshot, "task-b")
        peer.approve_plan(snapshot.version)
        with self.assertRaises(FixtureError):
            peer.reserve(fresh, peer.read(fresh), "task-a")
        peer.reserve(fresh, peer.read(fresh), "task-b")
        peer.end_runtime()
        fresh = peer.resume(fresh.native_id, fresh.workspace, fresh.configuration_digest)
        snapshot = peer.read(fresh)
        self.assertEqual(snapshot.state, "executing")
        self.assertFalse(snapshot.grant)
        self.assertEqual(snapshot.reserved, ("task-a", "task-b"))

    def test_results_are_detached_and_notifications_are_bounded_metadata(self):
        peer = Peer()
        for args in [("unknown", "ref"), ("report", "x" * 129), ("report", "text with spaces")]:
            with self.assertRaises(FixtureError):
                peer.emit(*args)
        peer.emit("report", "r")
        request = peer.wait(peer.binding)
        result = peer.result(request)
        result["events"][0]["reference"] = "tampered"
        self.assertEqual(peer.result(request)["events"][0]["reference"], "r")


if __name__ == "__main__":
    unittest.main()
