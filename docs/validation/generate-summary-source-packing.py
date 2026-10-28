"""Document-only source join and conservative compact JSON packing arithmetic."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def size(value):
    return len(canonical(value).encode())

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def wire(value):
    raw = canonical(value).encode()
    return dict(canonicalUtf8=raw.decode(), utf8Bytes=len(raw), sha256=sha(raw))

catalog_bytes = (HERE / 'hierarchy-summary-vectors.json').read_bytes()
store_bytes = (HERE / 'hierarchy-empty-output-profile.json').read_bytes()
catalog, store = json.loads(catalog_bytes), json.loads(store_bytes)
result = json.loads(store['sources']['resultCanonical'])
rows = store['states'][-1]['rows']
summary = json.loads(next(row['valueCanonical'] for row in rows if row['table'] == 'summary_state'))['row']
material = summary['state']['material']
subject = result['assignmentIdentity']
selection = dict(assignmentIdentity=subject, accepted=material['accepted'], summaryRevision=summary['summaryRevision'])
request = dict(schemaVersion=1, runId=subject['runId'], expectedSnapshotVersion=1, selection=selection)
context = dict(workspaceId=subject['workspaceId'], runId=subject['runId'],
               sidecarGeneration=result['authorityGeneration']['sidecarGeneration'],
               snapshotVersion=1, selection=selection, evaluations=[], fresh=True)
response = dict(schemaVersion=1, workspaceId=context['workspaceId'], runId=context['runId'],
                sidecarGeneration=context['sidecarGeneration'], snapshotVersion=1, selection=selection,
                detail=dict(material=material, criteria=store['profile']['criteria'], rubric=store['profile']['rubric'],
                evaluations=[], outcomes=[dict(target=o['target'],fullyAssessed=False,assessments=[]) for o in material['requiredOutcomes']],
                findingSources=[], completeness=dict(required=1,satisfied=0,unsatisfied=0,unassessed=1,percentage=dict(kind='unassessed')),
                quality=dict(kind='unassessed')))

ID, DIGEST = 'x' * 128, 'f' * 64
child = dict(kind='child',workspaceId=ID,runId=ID,planId=ID,planRevision=64,taskId=ID,taskVersion=2147483647,
             reservationId=ID,parentSessionId=ID,childSessionId=ID,configurationId=ID,configurationDigest=DIGEST)
root = dict(kind='root',workspaceId=ID,runId=ID,rootAssignmentId=ID,parentSessionId=ID,bootstrapId=ID,bootstrapConfigurationDigest=DIGEST)

def maximum_request(assignment):
    selection = dict(assignmentIdentity=assignment, accepted=dict(kind='result',reference=dict(assignmentIdentity=assignment,resultRevision=32,resultDigest=DIGEST)),summaryRevision='2147483647')
    renderer = dict(schemaVersion=1,runId=ID,expectedSnapshotVersion=2147483647,selection=selection)
    main = dict(renderer,workspaceId=ID,sidecarGeneration=DIGEST)
    return dict(renderer=wire(renderer),main=wire(main))

max_requests = {kind:maximum_request(value) for kind,value in [('root',root),('child',child)]}
# These deliberately non-unique, semantically incompatible shadow values only
# dominate serialized lengths. They are not candidate valid response fixtures.
reviewer = dict(kind='child',assignmentIdentity=child)
criteria = [dict(id=ID,requirement='required',description='\0'*2048)]*32
rubric = dict(id=ID,version=2147483647,dimensions=[dict(id=ID,description='\0'*2048)]*16,evaluatorRole='implementation')
reasons = ['declared-unassessed','missing-evidence','stale-evidence','evidence-unavailable','declared-conflict','invalidated','superseded','reviewer-ineligible']
annotation = dict(evaluationDigest=DIGEST,claimed='below-standard',effective='below-standard',participates=False,sourceState='unavailable',reasons=reasons)
outcomes = [dict(target=dict(kind='quality-dimension',id=ID),fullyAssessed=False,assessments=[annotation]*16)]*49
refs = [dict(assignmentIdentity=child,resultRevision=32,resultDigest=DIGEST,reviewerIdentity=reviewer,evaluationRevision=16,evaluationDigest=DIGEST)]*16
sources = [dict(reviewerIdentity=reviewer,findingId=ID,originEvaluationDigest=DIGEST,disputeEvaluationDigests=[])]*512
shadow_request = json.loads(max_requests['child']['renderer']['canonicalUtf8'])
shadow = dict(schemaVersion=1,workspaceId=ID,runId=ID,sidecarGeneration=DIGEST,snapshotVersion=2147483647,
              selection=shadow_request['selection'], detail=dict(material=None,criteria=criteria,rubric=rubric,evaluations=refs,
              outcomes=outcomes,findingSources=sources,completeness=dict(required=32,satisfied=32,unsatisfied=32,unassessed=32,
              percentage=dict(kind='assessed',numerator=32,denominator=32)),quality=dict(kind='disagreed',values=[0,1,2])))
components = {key:size(value) for key,value in shadow['detail'].items() if key != 'material'}
# Each effective dispute names one exact finding. At most 16 source evaluation
# revisions × 32 disputes can appear in all FindingSource lists combined.
dispute_bound = 16 * 32
# 66 encoded bytes per digest (64 ASCII + quotes), plus a comma. Counting a
# comma for the first item overestimates rather than undercounts an empty list.
dispute_extra = dispute_bound * (size(DIGEST)+1)
material_bound = 2*1024*1024
base_bytes = size(shadow)-size(None)
upper = base_bytes+material_bound+dispute_extra
measurements=[]
for vector in catalog['vectors']:
    measured=dict(id=vector['id'])
    for field in ['rendererRequest','mainRequest','response']:
        raw=vector[field]['canonicalUtf8'].encode()
        assert len(raw)==vector[field]['utf8Bytes'] and sha(raw)==vector[field]['sha256']
        measured[field+'Bytes']=len(raw)
    measured['materialBytes']=size(json.loads(vector['response']['canonicalUtf8'])['detail']['material'])
    measurements.append(measured)
packet=dict(schemaVersion=1,evidenceKind='synthetic-source-bridge-and-compact-packing-not-producer-qualification',
            inputs=dict(displayCatalogSha256=sha(catalog_bytes),logicalStoreSha256=sha(store_bytes)),
            sourceBridge=dict(context=context,rendererRequest=wire(request),response=wire(response)),
            maximumRequests=max_requests,displayMeasurements=measurements,
            compactResponseBound=dict(materialBytes=material_bound,componentBytes=components,
                envelopeBytes=base_bytes-sum(components.values()),baseWithoutMaterialBytes=base_bytes,
                maxEffectiveDisputeReferences=dispute_bound,disputeExtraBytes=dispute_extra,
                upperBytes=upper,limitBytes=4194304,headroomBytes=4194304-upper))
assert upper<=4194304
parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
args.output.write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n')
