"""Independent stdlib byte oracles and literal state traces; never invokes the JS checker."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
HERE = Path(__file__).resolve().parent
C = lambda x: json.dumps(x, sort_keys=True, ensure_ascii=False, separators=(',', ':'))
B = lambda x: len(C(x).encode())
H = lambda x: hashlib.sha256(x).hexdigest()
D = lambda domain, x: H((domain + '\n' + C(x)).encode())
raw = (HERE / 'report-publication-vectors.json').read_bytes()
old = json.loads(raw)
c = old['cases'][0]
request = c['request']
binding = dict(workspaceId=request['stream']['workspaceId'], sidecarGeneration='b' * 64)
scope = lambda n: dict(**binding, scopeSequence=str(n))
# Each expected tuple is literal: status, closed-through, open, unsettled, last-admission.
traces = [
 dict(id='close-before-open', seed={}, steps=[
  ['close', '1', ['ok','1',None,None,'0']],
  ['open', '1', ['closed','1',None,None,'0']],
  ['open', '2', ['ok','1','2',None,'0']],
  ['close', '1', ['ok','1','2',None,'0']]]),
 dict(id='admission-first-drains-original', seed={}, steps=[
  ['open','1',['ok','0','1',None,'0']],
  ['admit','1',['admitted','0','1','1','1']],
  ['close','1',['ok','1',None,'1','1']],
  ['open','2',['unavailable','1',None,'1','1']],
  ['uncertain','1',['unavailable','1',None,'1','1']],
  ['committed','1',['settled','1',None,None,'1']],
  ['open','2',['ok','1','2',None,'1']],
  ['admit','2',['admitted','1','2','2','2']]]),
 dict(id='close-first-denies-write', seed={}, steps=[
  ['open','1',['ok','0','1',None,'0']],
  ['close','1',['ok','1',None,None,'0']],
  ['admit','1',['closed','1',None,None,'0']]]),
 dict(id='skips-do-not-advance', seed={}, steps=[
  ['open','2',['conflict','0',None,None,'0']],
  ['close','2',['conflict','0',None,None,'0']],
  ['inspect','1',['ok','0',None,None,'0']]]),
 dict(id='admission-counter-does-not-wrap', seed={'lastAdmissionSequence':'18446744073709551615'}, steps=[
  ['open','1',['ok','0','1',None,'18446744073709551615']],
  ['admit','1',['exhausted','0','1',None,'18446744073709551615']],
  ['close','1',['ok','1',None,None,'18446744073709551615']]]),
 dict(id='scope-counter-final-close', seed={'closedThrough':'18446744073709551614'}, steps=[
  ['open','18446744073709551615',['ok','18446744073709551614','18446744073709551615',None,'0']],
  ['close','18446744073709551615',['ok','18446744073709551615',None,None,'0']],
  ['open','18446744073709551615',['closed','18446744073709551615',None,None,'0']]])
]
row = copy.deepcopy(next(r for r in c['after'] if r['table']=='report_receipts' and r['key']['reportVersion']==2))
row['body']['schemaVersion'] = 2
row['body']['authorization'] = dict(policy='terminal-ui-correction-v1', sidecarGeneration=binding['sidecarGeneration'], scopeSequence='1', admissionSequence='1')
projection = lambda body: {k:v for k,v in body.items() if k!='lastCommit'}
row_digest = D('orkworks.hierarchy-storage-row.v3',dict(table=row['table'],key=row['key'],body=projection(row['body'])))
prior = next(r for r in old['states']['current'] if r['table']=='store_header')['body']['latest']
manifest = dict(schemaVersion=3, workspaceId=binding['workspaceId'],storageEpoch=row['body']['lastCommit']['storageEpoch'],commitSequence='2',predecessor=prior,
 commandKind='report-publication',changes=[dict(table=row['table'],key=row['key'],beforeDigest=None,afterDigest=row_digest)])
# This manifest is only a membership component, NOT the complete legal publication changed set.
row['body']['lastCommit'] = dict(storageEpoch=manifest['storageEpoch'],commitSequence='2',changeDigest=D('orkworks.hierarchy-storage-change.v3',manifest))
maxrow = dict(table='report_receipts',key=old['boundaries']['receiptKey'],body=copy.deepcopy(old['boundaries']['receiptBody']))
maxrow['body']['schemaVersion'] = 2
maxrow['body']['authorization'] = dict(policy='terminal-ui-correction-v1',sidecarGeneration='f'*64,scopeSequence='18446744073709551615',admissionSequence='18446744073709551615')
packet = dict(schemaVersion=1,evidenceKind='synthetic-scope-and-isolated-receipt-membership-not-native-or-complete-publication',
 inputSha256=H(raw),reportProfileSha256=H((HERE/'r2-report-vectors.json').read_bytes()),binding=binding,traces=traces,
 requestUtf8=C(request),requestSha256=H(C(request).encode()),reportUtf8=C(request['report']),responseUtf8=C(row['body']['response']),
 receipt=dict(row=row,rowCommitment=row_digest,manifest=manifest,metadataBytes=B(row['key'])+B(row['body']),retainedBytes=B(row['key'])+B(projection(row['body'])),authorizationBytes=B(row['body']['authorization'])+len('"authorization":,') ),
 maximum=dict(row=maxrow,metadataBytes=B(maxrow['key'])+B(maxrow['body']),overCapBytes=B(maxrow['key'])+B(maxrow['body'])-2048))
assert packet['maximum']['metadataBytes']==2149 and packet['maximum']['overCapBytes']==101
parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
args.output.write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n')
