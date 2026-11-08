"""Synthetic direct-call peer. No I/O, real credentials, or native authority.

All guards below belong to this fake. They are not an OrkWorks implementation
or evidence that a native model received a tool result or resumed a session.
"""
from __future__ import annotations

from collections import deque
from copy import deepcopy
from dataclasses import asdict, dataclass
import hashlib
import json
import re

CONFIG = {"fixtureOnly": True, "ringSize": 256, "maxWaitSeconds": 30,
          "instanceId": "fixture-synthetic", "nativeId": "00000000-0000-0000-0000-000000000001",
          "workspace": "workspace-synthetic", "tasks": ["task-a", "task-b"]}
CONFIG_DIGEST = hashlib.sha256(json.dumps(CONFIG, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


class FixtureError(ValueError):
    pass


@dataclass(frozen=True)
class Binding:
    instance_id: str
    native_id: str
    workspace: str
    configuration_digest: str
    generation: str
    bearer: str


@dataclass(frozen=True)
class RunCursor:
    value: int
    instance_id: str = "fixture-synthetic"


@dataclass(frozen=True, eq=False)
class Request:
    request_id: str
    binding: Binding
    deadline: int


@dataclass(frozen=True)
class Snapshot:
    binding: Binding
    version: int
    cursor: RunCursor
    state: str
    stage_continue: bool
    blockers: tuple
    clarification: str | None
    grant: bool
    reserved: tuple


class Peer:
    """A controllable fake host; host mutations are direct fixture setup actions."""
    def __init__(self):
        self.instance_id = CONFIG["instanceId"]
        self.now = 0
        self.version = 1
        self.cursor = 0
        self._generation = 1
        self.binding = self._new_binding()
        self._events = deque(maxlen=CONFIG["ringSize"])
        self._requests = {}
        self._pending = None
        self._reads = {}
        self._effects = {}
        self._stage = False
        self._blockers = ()
        self._clarification = None
        self._grant = False
        self._plan_approved = False
        self._reserved = set()
        self._terminal = None

    def _new_binding(self):
        return Binding(self.instance_id, CONFIG["nativeId"], CONFIG["workspace"],
                       CONFIG_DIGEST, f"{self._generation:064x}", f"FAKE-bearer-{self._generation}")

    def _authorize(self, binding):
        if self.binding is None or binding is not self.binding:
            raise FixtureError("inactive fake binding")

    def _resolve(self, status, **fields):
        if self._pending is not None:
            request, _ = self._pending
            self._requests[request] = {"status": status, "requestId": request.request_id,
                                       "binding": asdict(request.binding), **fields}
            self._pending = None

    def _deliver_events(self):
        if self._pending is None:
            return
        _, cursor = self._pending
        if self._events and cursor < self._events[0]["cursor"] - 1:
            self._resolve("refresh_required", runVersion=self.version)
        elif cursor < self.cursor:
            events = [deepcopy(e) for e in self._events if e["cursor"] > cursor]
            self._resolve("events", runVersion=self.version, cursor=self.cursor, events=events)

    def wait(self, binding, cursor=None, timeout=30):
        self._authorize(binding)
        if self._pending is not None:
            raise FixtureError("wait already pending")
        cursor = RunCursor(0, self.instance_id) if cursor is None else cursor
        if (type(cursor) is not RunCursor or type(cursor.value) is not int
                or cursor.instance_id != self.instance_id or not 0 <= cursor.value <= self.cursor):
            raise FixtureError("invalid run cursor")
        if type(timeout) is not int or not 0 <= timeout <= CONFIG["maxWaitSeconds"]:
            raise FixtureError("invalid wait duration")
        request = Request(f"wait-{len(self._requests) + 1}", binding, self.now + timeout)
        self._requests[request] = None
        self._pending = (request, cursor.value)
        self._deliver_events()
        self.advance(0)
        return request

    def result(self, request):
        # Host-side result delivery is not another authorized run read. Already
        # pending waits may receive only their closure/revocation receipt.
        if request not in self._requests:
            raise FixtureError("unknown host request")
        return deepcopy(self._requests[request])

    def advance(self, seconds):
        if type(seconds) is not int or seconds < 0:
            raise FixtureError("invalid virtual clock delta")
        self.now += seconds
        if self._pending is not None and self.now >= self._pending[0].deadline:
            self._resolve("no_change", runVersion=self.version, cursor=self._pending[1])

    def emit(self, kind, reference):
        if (self._terminal or kind not in ("report", "approval", "blocker", "capacity")
                or not isinstance(reference, str) or not re.fullmatch(r"[A-Za-z0-9._-]{1,128}", reference)):
            raise FixtureError("invalid metadata notification")
        if self.cursor >= 2147483647 or self.version >= 2147483647:
            raise FixtureError("fixture counter exhausted")
        self.cursor += 1
        self.version += 1
        self._events.append({"cursor": self.cursor, "kind": kind, "reference": reference})
        self._deliver_events()

    def _state(self):
        if self._terminal:
            return self._terminal
        if self._stage:
            return "paused"
        if self._blockers:
            return "blocked"
        if self._clarification is not None:
            return "awaiting_clarification"
        if self._plan_approved:
            return "executing" if self._grant or len(self._reserved) == len(CONFIG["tasks"]) else "awaiting_resume_approval"
        return "synthesizing"

    def read(self, binding):
        self._authorize(binding)
        snapshot = Snapshot(binding, self.version, RunCursor(self.cursor, self.instance_id),
                            self._state(), self._stage, self._blockers, self._clarification,
                            self._grant, tuple(sorted(self._reserved)))
        self._reads[id(snapshot)] = snapshot
        return snapshot

    def _issued_read(self, binding, snapshot):
        self._authorize(binding)
        if self._reads.get(id(snapshot)) is not snapshot or snapshot.binding != binding:
            raise FixtureError("unissued fake snapshot")

    def _current_read(self, binding, snapshot):
        self._issued_read(binding, snapshot)
        if snapshot.version != self.version or snapshot.cursor.value != self.cursor:
            raise FixtureError("stale fake snapshot")

    def acknowledge(self, binding, snapshot, action_id):
        self._issued_read(binding, snapshot)
        if not isinstance(action_id, str) or not re.fullmatch(r"[A-Za-z0-9._-]{1,128}", action_id):
            raise FixtureError("invalid fake action id")
        key = (binding, snapshot.version, snapshot.cursor)
        if action_id in self._effects:
            previous_key, receipt = self._effects[action_id]
            if previous_key != key:
                raise FixtureError("fake action id conflict")
            return deepcopy(receipt)
        self._current_read(binding, snapshot)
        receipt = {"status": "acknowledged", "actionId": action_id,
                   "runVersion": snapshot.version, "cursor": snapshot.cursor.value,
                   "generation": binding.generation}
        self._effects[action_id] = (key, receipt)
        return deepcopy(receipt)

    @property
    def effect_count(self):
        return len(self._effects)

    def set_gates(self, stage_continue=False, blockers=(), clarification=None):
        self._stage, self._blockers, self._clarification = stage_continue, tuple(blockers), clarification
        self._grant = False
        self.emit("blocker", "gates")

    def approve_plan(self, expected_version):
        # Synthetic host UI action, not a notification or a production approval.
        if (self.binding is None or expected_version != self.version or self._stage
                or self._blockers or self._clarification is not None):
            raise FixtureError("fake approval blocked")
        self._plan_approved = True
        self._grant = True
        self.emit("approval", "fake-plan")

    def reserve(self, binding, snapshot, task_id):
        self._current_read(binding, snapshot)
        if not self._grant or task_id not in CONFIG["tasks"] or task_id in self._reserved:
            raise FixtureError("fake reservation denied")
        self._reserved.add(task_id)
        self.emit("capacity", task_id)

    def end_runtime(self):
        self._grant = False
        self._resolve("revoked")
        self.binding = None

    def resume(self, native_id, workspace, configuration_digest):
        if (self.binding is not None or self._terminal or native_id != CONFIG["nativeId"]
                or workspace != CONFIG["workspace"] or configuration_digest != CONFIG_DIGEST):
            raise FixtureError("exact fake resume denied")
        self._generation += 1
        self.binding = self._new_binding()
        return self.binding

    def close(self, state):
        if state not in ("complete", "cancelled") or self._terminal:
            raise FixtureError("invalid terminal transition")
        self._terminal = state
        self._grant = False
        self._resolve("run_closed")
        self.binding = None
