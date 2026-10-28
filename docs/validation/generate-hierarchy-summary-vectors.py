"""Independent synthetic display vectors; no source/producer/native qualification."""
import argparse
import copy
import hashlib
import json
from pathlib import Path


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def sha(value):
    return hashlib.sha256(value.encode()).hexdigest()


def domain_hash(domain, value):
    return sha(domain + '\n' + canonical(value))


assignment = dict(kind='child', workspaceId='w', runId='run', planId='p',
                  planRevision=1, taskId='t', taskVersion=1, reservationId='res',
                  parentSessionId='parent', childSessionId='worker',
                  configurationId='cfg', configurationDigest=sha('synthetic-cfg'))
root = dict(kind='root', workspaceId='w', runId='run', rootAssignmentId='root',
            parentSessionId='parent', bootstrapId='boot',
            bootstrapConfigurationDigest=sha('synthetic-bootstrap'))
criteria = [dict(id='c1', requirement='required', description='First required outcome — æ.'),
            dict(id='c2', requirement='required', description='Second required outcome.'),
            dict(id='o', requirement='optional', description='Optional context only.')]
rubric = dict(id='rubric', version=1, dimensions=[dict(id='d', description='Correctness.')], evaluatorRole='review')
result_digest = sha('synthetic-result-reference-not-a-source-record')


def target(kind, id=None):
    return dict(kind=kind, **({'id': id} if id else {}))


def reference(index, subject):
    reviewer = dict(kind='user', userReviewId='user-review')
    if index:
        reviewer_assignment = dict(assignment, taskId='review', reservationId='review-res',
                                  childSessionId='reviewer', configurationId='review-cfg',
                                  configurationDigest=sha('synthetic-review-cfg'))
        reviewer = dict(kind='child', assignmentIdentity=reviewer_assignment)
    return dict(assignmentIdentity=subject, resultRevision=1, resultDigest=result_digest,
                reviewerIdentity=reviewer, evaluationRevision=1,
                evaluationDigest=sha('synthetic-evaluation-' + str(index)))


def vector(name, values=None, *, overall='needs-rework', accepted=True, subject=None,
           counts=None, quality=None, finding=None, snapshot=1, revision='1', legacy=False):
    subject = copy.deepcopy(subject or assignment)
    approved = copy.deepcopy(criteria if not legacy else [criteria[-1]])
    accepted_head = dict(kind='result', reference=dict(assignmentIdentity=subject, resultRevision=1,
                                                     resultDigest=result_digest)) if accepted else dict(kind='none')
    values = values or [['satisfied', 'satisfied', 'meets', 3]]
    refs = sorted([reference(i, subject) for i in range(len(values))], key=canonical) if accepted else []
    # Keep original per-stream values paired with their independently ordered references.
    by_digest = {reference(i, subject)['evaluationDigest']: row for i, row in enumerate(values)}
    targets = [target('criterion', c['id']) for c in approved if c['requirement'] == 'required']
    targets += [target('quality-dimension', 'd'), target('rating')]
    targets.sort(key=canonical)
    outcomes, material_outcomes = [], []
    for t in targets:
        index = {'c1': 0, 'c2': 1, 'd': 2}.get(t.get('id'), 3)
        annotations = []
        for ref in sorted(refs, key=lambda r: r['evaluationDigest']):
            value = by_digest[ref['evaluationDigest']][index]
            source_state = value.get('sourceState', 'current') if isinstance(value, dict) else 'current'
            claimed = value['claimed'] if isinstance(value, dict) else value
            stale = source_state == 'stale'
            disputed = claimed == 'disputed'
            value = 'unsatisfied' if disputed else claimed
            annotations.append(dict(evaluationDigest=ref['evaluationDigest'], claimed=value,
                                    effective='unassessed' if stale or disputed else value, participates=True,
                                    sourceState='stale' if stale else 'current',
                                    reasons=['stale-evidence'] if stale else ['declared-conflict'] if disputed else ['declared-unassessed'] if value == 'unassessed' else []))
        effective = [a['effective'] for a in annotations]
        distinct = set(effective)
        assessed = bool(effective) and 'unassessed' not in distinct
        failure_values = {'unsatisfied', 'below-standard', 0, 1, 2}
        assessed_values = distinct - {'unassessed'}
        conflict = any('declared-conflict' in a['reasons'] for a in annotations) or (len(assessed_values) > 1 and not (t['kind'] == 'rating' and assessed_values <= {0, 1, 2}))
        fully = assessed and not conflict
        disposition = 'conflicted' if conflict else 'needs-rework' if distinct & failure_values else 'meets' if fully else 'unassessed'
        outcomes.append(dict(target=t, fullyAssessed=fully, assessments=annotations))
        material_outcomes.append(dict(target=t, disposition=disposition))
    material = dict(schemaVersion=1, assignmentIdentity=subject, accepted=accepted_head,
                    criteriaSnapshotDigest=domain_hash('orkworks.criteria-snapshot.v1', approved),
                    rubricSnapshotDigest=domain_hash('orkworks.rubric-snapshot.v1', rubric),
                    overall=overall, requiredOutcomes=material_outcomes, requiredFindings=[])
    sources = []
    if finding:
        origin = refs[0]
        disputed = finding == 'disputed'
        material['requiredFindings'] = [dict(reviewerIdentity=origin['reviewerIdentity'], findingId='f',
            description='Required fix <script> remains plain text.', location='file:1', severity='major',
            requiredImpactTargets=[target('criterion', 'c1')], disposition='disputed' if disputed else 'current')]
        sources = [dict(reviewerIdentity=origin['reviewerIdentity'], findingId='f',
                        originEvaluationDigest=origin['evaluationDigest'],
                        disputeEvaluationDigests=[refs[1]['evaluationDigest']] if disputed else [])]
    selection = dict(assignmentIdentity=subject, accepted=accepted_head, summaryRevision=revision)
    request = dict(schemaVersion=1, runId='run', expectedSnapshotVersion=snapshot, selection=selection)
    context = dict(workspaceId='w', runId='run', sidecarGeneration=sha('synthetic-lifetime'),
                   snapshotVersion=snapshot, selection=selection, evaluations=refs, fresh=True)
    main_request = dict(request, workspaceId='w', sidecarGeneration=context['sidecarGeneration'])
    counts = counts or dict(required=2, satisfied=0, unsatisfied=2, unassessed=0,
                            percentage=dict(kind='assessed', numerator=0, denominator=2))
    response = dict(schemaVersion=1, workspaceId='w', runId='run',
                    sidecarGeneration=context['sidecarGeneration'], snapshotVersion=snapshot,
                    selection=selection, detail=dict(material=material, criteria=approved, rubric=copy.deepcopy(rubric),
                    evaluations=refs, outcomes=outcomes, findingSources=sources, completeness=counts,
                    quality=quality or dict(kind='rating', value=3)))
    wires = {k: canonical(v) for k, v in dict(rendererRequest=request, mainRequest=main_request, response=response).items()}
    return dict(id=name, context=context, **{k:dict(canonicalUtf8=v, utf8Bytes=len(v.encode()), sha256=sha(v)) for k, v in wires.items()})


U = dict(kind='unassessed')
vectors = [
    vector('r2-required-stale', [[dict(claimed='unsatisfied', sourceState='stale'),'unsatisfied','meets',3]], counts=dict(required=2,satisfied=0,unsatisfied=1,unassessed=1,percentage=U)),
    vector('r2-optional-stale', [['unsatisfied','unsatisfied','meets',3]], snapshot=2, revision='2'),
    vector('failure-plus-unassessed', [['unsatisfied','satisfied','meets',3],['unassessed','satisfied','meets',3]],
           counts=dict(required=2,satisfied=1,unsatisfied=0,unassessed=1,percentage=U)),
    vector('ratings-0-2', [['satisfied','satisfied','meets',0],['satisfied','satisfied','meets',2]],
           counts=dict(required=2,satisfied=2,unsatisfied=0,unassessed=0,percentage=dict(kind='assessed',numerator=2,denominator=2)), quality=dict(kind='disagreed',values=[0,2])),
    vector('ratings-2-3', [['satisfied','satisfied','meets',2],['satisfied','satisfied','meets',3]], overall='unassessed',
           counts=dict(required=2,satisfied=2,unsatisfied=0,unassessed=0,percentage=dict(kind='assessed',numerator=2,denominator=2)), quality=U),
    vector('missing-dimension-rating-1', [['satisfied','satisfied','unassessed',1]],
           counts=dict(required=2,satisfied=2,unsatisfied=0,unassessed=0,percentage=dict(kind='assessed',numerator=2,denominator=2)), quality=U),
    vector('rating-3-below-standard', [['satisfied','satisfied','below-standard',3]],
           counts=dict(required=2,satisfied=2,unsatisfied=0,unassessed=0,percentage=dict(kind='assessed',numerator=2,denominator=2)), quality=U),
    vector('meets', overall='meets-requirements', counts=dict(required=2,satisfied=2,unsatisfied=0,unassessed=0,percentage=dict(kind='assessed',numerator=2,denominator=2))),
    vector('root-meets', subject=root, overall='meets-requirements', counts=dict(required=2,satisfied=2,unsatisfied=0,unassessed=0,percentage=dict(kind='assessed',numerator=2,denominator=2))),
    vector('root-no-result', accepted=False, subject=root, overall='unassessed', quality=U,
           counts=dict(required=2,satisfied=0,unsatisfied=0,unassessed=2,percentage=U)),
    vector('legacy-zero-required', [['unassessed','unassessed','unassessed','unassessed']], legacy=True, overall='unassessed', quality=U,
           counts=dict(required=0,satisfied=0,unsatisfied=0,unassessed=0,percentage=U)),
    vector('current-finding', [['unsatisfied','unsatisfied','meets',3]], finding='current'),
    vector('disputed-finding', [['disputed','unsatisfied','meets',3],['disputed','unsatisfied','meets',3]], finding='disputed',
           counts=dict(required=2,satisfied=0,unsatisfied=1,unassessed=1,percentage=U)),
]
# Retain same summary material while nonmaterial rating data changes.
for name, rating, snapshot in [('rating-change-0',0,3),('rating-change-2',2,4)]:
    vectors.append(vector(name, [['unsatisfied','unsatisfied','meets',rating]], quality=dict(kind='rating',value=rating), snapshot=snapshot))
packet = dict(fixtureSchemaVersion=1, evidenceKind='synthetic-display-components-not-source-valid', vectors=vectors)
parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
