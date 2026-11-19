"""Offline research intent/source-digest examples; no store, manifest or permit."""
import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent

def canonical(x):
    return json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(',', ':'))

def sha(x):
    return hashlib.sha256(x if isinstance(x, bytes) else x.encode('utf-8')).hexdigest()

def domain(name, x):
    return sha(name + '\n' + canonical(x))

def origin(label, sequence):
    # Documentation labels only: no storage manifest or change hashing.
    return dict(storageEpoch='f' * 64, commitSequence=str(sequence), changeDigest=sha('UNQUALIFIED research authorization label: ' + label))

def packet():
    raw = (ROOT / 'research-prefix-components.json').read_bytes()
    baseline = json.loads(raw)
    chain = baseline['chain']
    assignment = chain['assignment']
    identity = assignment['assignmentIdentity']
    parent = baseline['runtimeHistory'][0]
    parent_ref = dict(**parent['key'], attachmentDigest=parent['value']['digest'])
    # Build each prestate from original source records, never a later control view.
    run = chain['startup']['context']['run']
    task = chain['startup']['scope']['task']
    input_body = baseline['inputs'][0]['value']['body']
    input_ref = {k: input_body[k] for k in ['workspaceId','runId','preparationRevision','inputRevision','inputDigest']}
    run_key = {k: run[k] for k in ['workspaceId','runId']}
    run_body = dict(schemaVersion=1, profile='single-task-retained-file-research-prefix-v1',
        **{k:run[k] for k in ['workspaceId','runId','parentSessionId','runDefinitionDigest','bootstrapConfigurationDigest','repositoryBindingDigest','sourceWorktreeBindingDigest']},
        runVersion=7,state='researching',currentInput=input_ref,planReferences=[assignment['plan']],
        currentPlan=assignment['plan'],pendingPlan=None,parentRuntime=parent_ref,blockers=[],stageContinueRequired=False,
        pendingClarification=None,pauseReason=None,resumeTarget=None)
    plan_body = dict(schemaVersion=1,plan=assignment['plan'],planVersion=2,status='active',latestApproval=assignment['approval'],task=task,replaces=None)
    task_body = dict(schemaVersion=1,task=task,taskVersion=4,status='launching',reservation=assignment['reservation'],coordinationResult=None)
    worktree = chain['allocation']['allocatedWorktree']
    observed = [dict(kind='private-git',path=worktree['canonicalPrivateGitDirectory'],object=worktree['privateGitDirectoryObject']),
                dict(kind='root',path=worktree['canonicalRoot'],object=worktree['rootObject'])]
    launch_body = dict(schemaVersion=1,task=task,reservation=assignment['reservation'],
        admission=dict(approval=assignment['approval'],parentRuntime=parent_ref,expectedTaskVersion=chain['reservation']['expectedTaskVersion'],assignmentTaskVersion=identity['taskVersion']),
        phase='allocated',allocation=assignment['allocation'],attachmentId=None,spawnRuntime=None,observedDirectories=observed,interruptedAt=None,reasonCode=None)
    controls = {name:dict(table='research_'+name+'_controls',key=copy.deepcopy(key),
        value=dict(schemaVersion=1,body=copy.deepcopy(body),lastCommit=origin(name+'-prestate',2)))
        for name,key,body in [('run',run_key,run_body),('plan',assignment['plan'],plan_body),('task',task,task_body),('launch',task,launch_body)]}
    h, committed = origin('captured-H', 3), origin('supplied-original-C', 4)
    ordinary = dict(databaseEpoch='11111111-2222-4333-8444-555555555555', historyGeneration='0', createdAt='synthetic original creation — not a timestamp proof')
    intent = dict(workspaceId=identity['workspaceId'], storageEpoch=h['storageEpoch'], admissionEpoch='e' * 64,
                  expectedCommit=h, ordinary=ordinary, assignmentIdentity=identity,
                  authority=dict(kind='original-parent-research-source', parentRuntime=parent_ref),
                  attachment=dict(childSessionId=identity['childSessionId'], attachmentId=chain['attachment']['attachmentId'], attachmentDigest=chain['attachment']['attachmentDigest']),
                  producerSource=dict(key=dict(kind='research-assignment-source', identity=identity), digest=chain['fingerprints']['assignmentSourceDigest']), controls=controls)
    body = dict(schemaVersion=1, permit=dict(sidecarGeneration=parent_ref['sidecarGeneration'], sequence='1'), intent=intent)
    auth = dict(body=body, digest=domain('orkworks.research-preparation-authorization.v1', body), lastCommit=committed)
    member = dict(schemaVersion=1, assignmentIdentity=identity, attachmentId=intent['attachment']['attachmentId'],
                  assignmentSourceDigest=intent['producerSource']['digest'], preparationCommit=committed)
    lineage = dict(createdAt=ordinary['createdAt'], linkage=member)
    assumptions = ['source/native identity and allocation verified externally', 'original authority and permit acquisition supplied',
                   'complete retained/tombstone membership and uniqueness supplied', 'source/control origin proofs and full profile supplied',
                   'paired ownership, H/G currentness and original commit outcome supplied', 'resource balances and full job/physical capacity unqualified']
    resolution = dict(basis='supplied-unqualified', originalCommand=dict(commit=committed, commandKind='research-source-attach'),
                      authorizationFamily='research_preparation_authorizations', source=intent['producerSource'],
                      attachment=chain['attachment'], assignment=assignment, ordinaryId=identity['childSessionId'],
                      lineage=lineage, authorizations=[dict(family='research_preparation_authorizations', key=identity, value=auth)])
    schedules = [dict(id=k, expected=v) for k,v in [
        ('revoke-first','deny'), ('permit-then-volatile-close','may-save/no-spawn'),
        ('durable-H-drift','deny/rollback'), ('ordinary-G-drift','deny/rollback'),
        ('ownership-lost','deny/rollback'), ('proven-rollback','old-state/sequence-spent'),
        ('unknown-disposition','retain/no-retry'), ('commit-response-lost','original-result/no-spawn'),
        ('workspace-switch','detach/retain-original-pair')]]
    projection = dict(body=body, digest=auth['digest'])
    size = lambda x: len(canonical(x).encode('utf-8'))
    return dict(inputSha256=sha(raw), assumptions=assumptions, authorizationKey=identity, authorization=auth,
                ordinaryMember=member, resolution=resolution, schedules=schedules,
                measurements=dict(controlWrapperBytes=[size(dict(key=r['key'],value=r['value'])) for r in controls.values()],
                                  authorizationCharge=size(dict(key=identity,value=projection)),
                                  authorizationWrapperBytes=size(dict(key=identity,value=auth)),
                                  ordinaryMemberBytes=size(member), suppliedJointPartsBytes=size(dict(authorization=auth,ordinaryMember=member))))

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    wire=(json.dumps(packet(),ensure_ascii=False,indent=2)+'\n').encode('utf-8')
    if args.output:
        args.output.write_bytes(wire)
    else:
        sys.stdout.buffer.write(wire)
