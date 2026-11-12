#!/usr/bin/env python3
"""Independent synthetic documentation bytes; never captures actual files/Git."""
import argparse
import hashlib
import json
import sys
from pathlib import Path


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode('utf-8')


def sha(value):
    return hashlib.sha256(value).hexdigest()


def seal(value, field, domain):
    value[field] = sha(domain.encode('ascii') + b'\n' + canonical(value))
    return value


def packet():
    task = dict(workspaceId='w', runId='r', planId='p', planRevision=1, taskId='t')
    bindings = dict(task=task, repositoryBindingDigest='a' * 64, sourceWorktreeBindingDigest='b' * 64)
    context = dict(**bindings, baseCommit='c' * 40, objectFormat='sha1')
    captures, excerpts, questions = [], [], []
    for suffix, content, start, length in [('a', 'Aé🙂Z\n', 1, 6), ('b', 'Retained data only.\n', 0, 8)]:
        raw = content.encode('utf-8')
        source_id = 'source-' + suffix
        capture = seal(dict(schemaVersion=1, **bindings, sourceId=source_id,
                            path='src/note-' + suffix + '.txt', revision=context['baseCommit'],
                            content=content, sizeBytes=len(raw), contentDigest=sha(raw)),
                       'captureDigest', 'orkworks.research-source-capture.v1')
        captures.append(capture)
        excerpts.append(dict(sourceId=source_id, startByte=start, lengthBytes=length,
                             reference=dict(kind='file', id='e-' + suffix, path=capture['path'],
                                            revision=capture['revision'], location=f'bytes:{start}-{start + length}',
                                            excerptDigest=sha(raw[start:start + length]))))
        questions.append(dict(questionId='q-' + suffix, questionText='What is retained?', sourceIds=[source_id],
                              knownInputs='Synthetic text only.', expectedEvidence='A byte-exact excerpt.',
                              stoppingConditions='Stop after the retained input.', uncertainty='Native origin is unverified.'))
    scope = seal(dict(schemaVersion=1, **bindings,
                      captures=[dict(task=task, sourceId=c['sourceId'], captureDigest=c['captureDigest']) for c in captures],
                      questions=questions, excerpts=excerpts), 'scopeDigest', 'orkworks.research-task-scope.v1')
    return dict(context=context, captures=captures, scope=scope)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    wire = (json.dumps(packet(), ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode('utf-8')
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
