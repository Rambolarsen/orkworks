#!/usr/bin/env python3
"""API-only agent metadata producer. Never reads or writes session JSON files."""

import http.client
import json
import os
from pathlib import Path
import re
import shlex
import sys

MAX_BODY = 8192
MAX_RESPONSE = 2 * 1024 * 1024
LEGACY_MAX_RESPONSE = 1024 * 1024
TIMEOUT_SECONDS = 5


class ProducerError(Exception):
    pass


class RevisionConflict(ProducerError):
    pass


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ProducerError("duplicate metadata JSON key")
        result[key] = value
    return result


def valid_revision(value):
    return isinstance(value, str) and re.fullmatch(r"0|[1-9][0-9]{0,19}", value) is not None and int(value) < 2**64


class Transport:
    def __init__(self, http_proxy=None):
        self.max_response = MAX_RESPONSE
        self.session_id = os.environ.get("ORKWORKS_SESSION_ID", "")
        port = os.environ.get("ORKWORKS_PORT", "")
        self.token = os.environ.get("ORKWORKS_REPORT_TOKEN", "")
        if re.fullmatch(r"[A-Za-z0-9_-]{1,128}", self.session_id) is None:
            raise ProducerError("invalid metadata session environment")
        if re.fullmatch(r"[0-9]{1,5}", port) is None or not 0 < int(port) <= 65535:
            raise ProducerError("invalid metadata port environment")
        if not self.token or len(self.token) > 4096 or any(ord(c) < 33 or ord(c) > 126 for c in self.token):
            raise ProducerError("invalid metadata reporting capability")
        self.proxy_port = None
        if http_proxy is not None:
            # Caller-selected or qualified managed worktree relay. Do not use URL parsing
            # that normalizes controls, credentials or ambiguous host spellings.
            proxy = re.fullmatch(r"http://127\.0\.0\.1:([0-9]{1,5})/?", http_proxy)
            if proxy is None or not 0 < int(proxy[1]) <= 65535:
                raise ProducerError("invalid HTTP proxy; expected http://127.0.0.1:<port>")
            self.proxy_port = int(proxy[1])
        self.port = int(port)
        self.path = "/sessions/" + self.session_id + "/metadata"

    def request(self, method, body=None, validate_only=False):
        encoded = None if body is None else json.dumps(body, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        if encoded is not None and len(encoded) > MAX_BODY:
            raise ProducerError("metadata patch exceeds the size limit")
        # Unqualified ambient proxies are ignored. The selected relay receives the
        # capability; the origin remains pinned and redirects are never followed.
        connection = http.client.HTTPConnection("127.0.0.1", self.proxy_port or self.port, timeout=TIMEOUT_SECONDS)
        try:
            path = self.path + ("?validateOnly=true" if validate_only else "")
            if self.proxy_port is not None:
                path = "http://127.0.0.1:" + str(self.port) + path
            connection.request(method, path, body=encoded, headers={"Authorization": "Bearer " + self.token, "Content-Type": "application/json"})
            response = connection.getresponse()
            if response.status == 409:
                raise RevisionConflict("metadata revision conflict; reread and retry the intended patch")
            if response.status != 200:
                raise ProducerError("metadata API request failed (HTTP " + str(response.status) + ")")
            raw = response.read(self.max_response + 1)
            if len(raw) > self.max_response:
                raise ProducerError("metadata response exceeds the size limit")
            value = json.loads(raw, object_pairs_hook=unique_object)
            if not isinstance(value, dict) or not valid_revision(value.get("metadataRevision")):
                raise ProducerError("invalid metadata API response")
            return value
        except (OSError, http.client.HTTPException, ValueError):
            # Neither server response bodies nor transport exceptions are safe
            # diagnostics: they can contain capabilities or request details.
            raise ProducerError("metadata API transport or response failed") from None
        finally:
            connection.close()

    def read(self):
        value = self.request("GET")
        if value.get("id") != self.session_id:
            raise ProducerError("metadata API returned a different session")
        return value

    def bootstrap(self):
        record = self.read()
        revision = record["metadataRevision"]
        # This exercises the actual PATCH route/schema/revision/auth boundary
        # without touching metadata, even when the existing source is user.
        probe = {"metadataRevision": revision, "patch": {"agentMetadata": {"summary": None}}}
        result = self.request("PATCH", probe, validate_only=True)
        if result.get("validated") is not True or result["metadataRevision"] != revision:
            raise ProducerError("metadata bootstrap was not validated")
        if self.read()["metadataRevision"] != revision:
            raise ProducerError("metadata bootstrap changed revision; reread before retrying")
        return result


def metadata_protocol(required=False):
    # Ordinary storage negotiation is independent of the gated native marker.
    # A present unknown value never downgrades to another protocol or to files.
    ordinary = os.environ.get("ORKWORKS_SESSION_STORAGE_PROTOCOL")
    native = os.environ.get("ORKWORKS_SESSION_METADATA_API_VERSION")
    if ordinary is not None:
        if ordinary != "sidecar-v1" or (native is not None and native != "1"):
            raise ProducerError("unsupported session metadata protocol environment")
        return "ordinary"
    if native == "1":
        return "native"
    if required:
        raise ProducerError("API-only metadata protocol version 1 is required")
    return None


class Client(Transport):
    def __init__(self, http_proxy=None):
        protocol = metadata_protocol(required=True)
        super().__init__(http_proxy)
        if protocol == "native":
            self.max_response = LEGACY_MAX_RESPONSE


def worktree_instructions():
    if not all(os.environ.get(key) for key in ("ORKWORKS_SESSION_ID", "ORKWORKS_PORT", "ORKWORKS_REPORT_TOKEN")):
        return None
    helper = str(Path(__file__).resolve())
    helper = "'" + helper.replace("'", "''") + "'" if os.name == "nt" else shlex.quote(helper)
    context = (
        "OrkWorks task worktree reporting: after selecting or creating your task worktree, "
        "before editing there, run `python3 " + helper + " worktree ABSOLUTE_PATH` from your "
        "actual tool execution context. This command automatically uses the Codex-managed "
        "HTTP relay when CODEX_NETWORK_PROXY_ACTIVE=1 qualifies its strict loopback HTTP_PROXY; "
        "unqualified ambient proxy settings remain ignored. A selected relay failure never "
        "falls back to direct access. If another context requires an already owner-authorized "
        "sandbox-managed HTTP proxy, explicitly select the verified proxy with "
        "`python3 " + helper + " worktree ABSOLUTE_PATH --http-proxy TRUSTED_PROXY_URL` "
        "instead of the direct command. TRUSTED_PROXY_URL must be the actual authorized "
        "managed relay, in the form http://127.0.0.1:PORT; it receives the reporting capability. "
        "Do not infer trust from ambient proxy variables or a loopback address alone. "
        "If no authorized route is available, report that location could not be updated. "
        "Report intentional task-worktree switches too. "
        "Temporary read-only commands elsewhere must not change the report. "
        "Use `worktree --clear` only to withdraw the binding. Only the owning agent reports; "
        "subagents must not replace their owning session's worktree. "
        "This command updates only Git location, never attention, task, summary or native identity. "
        "On failure, report that location could not be updated; do not change sandbox policy, "
        "retry a conflict blindly, or write metadata files. Never print or persist inherited credentials."
    )
    return {"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": context}}


def read_patch():
    raw = sys.stdin.buffer.read(MAX_BODY + 1)
    if len(raw) > MAX_BODY:
        raise ProducerError("metadata patch exceeds the size limit")
    try:
        envelope = json.loads(raw, object_pairs_hook=unique_object)
    except (ValueError, UnicodeError):
        raise ProducerError("invalid metadata patch JSON") from None
    if not isinstance(envelope, dict) or set(envelope) != {"metadataRevision", "patch"} or not valid_revision(envelope["metadataRevision"]) or not isinstance(envelope["patch"], dict):
        raise ProducerError("metadata patch requires an explicit revision and patch object")
    return envelope


def instructions():
    protocol = metadata_protocol()
    if protocol is None:
        return None
    helper_path = str(Path(__file__).resolve())
    helper = "\'" + helper_path.replace("\'", "\'\'") + "\'" if os.name == "nt" else shlex.quote(helper_path)
    context = (
        ("OrkWorks ordinary session storage uses API-only sidecar-v1 reporting for this session. "
         if protocol == "ordinary" else
         "OrkWorks session metadata uses API-only protocol version 1 for this session. ") +
        "Never write session metadata JSON files directly, including on API failure. "
        "Before any metadata mutation, run `python3 " + helper + " bootstrap` from your actual "
        "agent shell/tool execution context under its effective sandbox. This performs "
        "authenticated GET and validation-only PATCH without changing metadata. "
        "If bootstrap fails, stop metadata reporting; do not weaken the sandbox or use file fallback. "
        "Use `python3 " + helper + " get` to read the current metadataRevision. "
        "Send a JSON object with metadataRevision (the exact string read) and patch to "
        "`python3 " + helper + " patch` on stdin. This repeats bootstrap before writing. "
        "On revision conflict, reread and reconsider the patch; never retry blindly. "
        "The patch may contain attentionState (the complete observedStatus, needsUserInput, "
        "detectedQuestion, suggestedOptions tuple) and/or agentMetadata (summary, planPath, "
        "or a complete workFields snapshot: task, nextAction, workPhase, blockerDescription, "
        "failedCommand, failedTest). Use null explicitly to clear a nullable field. "
        "Reporting credentials are inherited environment capabilities: never print, log, "
        "persist, or put them in tool arguments. This protocol does not authorize "
        "native attention clearing or changes to other sessions."
    )
    return {"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": context}}


def main():
    arguments = sys.argv[1:]
    http_proxy = None
    if len(arguments) >= 3 and arguments[-2] == "--http-proxy":
        http_proxy = arguments[-1]
        arguments = arguments[:-2]
    worktree = len(arguments) == 2 and arguments[0] == "worktree"
    if not (worktree or (len(arguments) == 1 and arguments[0] in ("bootstrap", "get", "patch", "instructions", "worktree-instructions"))):
        raise ProducerError("usage: session-metadata.py bootstrap|get|patch [--http-proxy URL] | worktree ABSOLUTE_PATH|--clear [--http-proxy URL] | instructions | worktree-instructions")
    if arguments[0] in ("instructions", "worktree-instructions") and http_proxy is not None:
        raise ProducerError("instructions do not accept a proxy")
    if worktree:
        path = None if arguments[1] == "--clear" else arguments[1]
        if path is not None and (not Path(path).is_absolute() or len(path.encode("utf-8")) > 4096 or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in path)):
            raise ProducerError("worktree requires an absolute path")
        # Only this narrow command opts into the owner-approved Codex runtime
        # transport. The active marker qualifies the runtime-injected HTTP_PROXY;
        # a loopback address or ambient proxy alone is never sufficient.
        if http_proxy is None and os.environ.get("CODEX_NETWORK_PROXY_ACTIVE") == "1":
            http_proxy = os.environ.get("HTTP_PROXY", "")
            if "http_proxy" in os.environ and os.environ["http_proxy"] != http_proxy:
                raise ProducerError("inconsistent managed HTTP proxy environment")
        client = Transport(http_proxy)
        record = client.read()
        result = client.request("PATCH", {"metadataRevision": record["metadataRevision"], "patch": {"agentMetadata": {"worktreePath": path}}})
        if result.get("id") != client.session_id:
            raise ProducerError("metadata API returned a different session")
        print(json.dumps({"metadataRevision": result["metadataRevision"], "worktreeBinding": result.get("worktreeBinding")}))
        return
    command = arguments[0]
    if command in ("instructions", "worktree-instructions"):
        result = instructions() if command == "instructions" else worktree_instructions()
        if command == "instructions" and result is not None:
            worktree_context = worktree_instructions()
            if worktree_context:
                result["hookSpecificOutput"]["additionalContext"] += " " + worktree_context["hookSpecificOutput"]["additionalContext"]
    else:
        envelope = read_patch() if command == "patch" else None
        client = Client(http_proxy)
        if command == "get":
            result = client.read()
        else:
            result = client.bootstrap()
            if command == "patch":
                # Preserve the caller's revision; a newer bootstrap revision
                # must not silently turn a stale intent into an accepted write.
                result = client.request("PATCH", envelope)
                if result.get("id") != client.session_id:
                    raise ProducerError("metadata API returned a different session")
    if result is not None:
        print(json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except RevisionConflict as error:
        print(str(error), file=sys.stderr)
        sys.exit(3)
    except ProducerError as error:
        print(str(error), file=sys.stderr)
        sys.exit(2)
