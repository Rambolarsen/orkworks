import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync, sign } from 'node:crypto';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { verifyKnowledgeActivation, selectKnowledgeEntry } from '../electron/knowledgeProof.ts';

const ids = ['context-discovery','progressive-disclosure','instruction-scoping','architecture-awareness','domain-knowledge','environment-reproducibility','tool-accessibility','task-decomposition','verification','observability','change-safety','feedback-loops','durable-memory','human-oversight'];
function fixture() {
 const old = JSON.parse(readFileSync(new URL('../../../tests/fixtures/taskmaster-knowledge/valid-activation.json', import.meta.url), 'utf8'));
 const core = JSON.parse(JSON.parse(Buffer.from(old.bundleEnvelopeBase64,'base64').toString()).payload);
 core.formatVersion = 2;
 core.capabilities = ['taskmaster-assessment-v1','taskmaster-readiness-v1'];
 core.readinessRubric = {id:'brain-agent-readiness',revision:1,concepts:ids.map(conceptId => ({conceptId,pageId:core.pages[0].id,pageSha256:core.pages[0].sha256,levels:['Implicit','Discoverable','Usable','Integrated','Adaptive']}))};
 const keys = generateKeyPairSync('ed25519');
 return { core, activate() {
   const envelope = (data: unknown) => { const payload=JSON.stringify(data); return Buffer.from(JSON.stringify({payload,signature:sign(null,Buffer.from(payload),keys.privateKey).toString('base64')})); };
   const bundle=envelope(core);
   const manifest=envelope({formatVersion:1,bundles:[{formatVersion:core.formatVersion,privacyPolicyVersion:1,version:core.version,sequence:core.sequence,sha256:createHash('sha256').update(bundle).digest('hex'),path:`bundles/${core.version}.json`}]});
   return verifyKnowledgeActivation(Buffer.from(JSON.stringify({activationFormatVersion:1,bundleEnvelopeBase64:bundle.toString('base64'),manifestEnvelopeBase64:manifest.toString('base64')})),keys.publicKey.export({format:'pem',type:'spki'}).toString());
 }};
}
test('signed format two admits the exact complete readiness rubric', () => {
 const f=fixture(); const result=f.activate();
 assert.equal(result.bundle.formatVersion,2);
 assert.equal((result.bundle as any).readinessRubric.concepts.length,14);
 assert.ok(Object.isFrozen((result.bundle as any).readinessRubric.concepts));
});
test('format two rejects absent, incomplete, unordered, forged or extended rubric definitions', () => {
 const mutations=[
 (x:any)=>delete x.readinessRubric,
 (x:any)=>x.readinessRubric.concepts.pop(),
 (x:any)=>x.readinessRubric.concepts.reverse(),
 (x:any)=>x.readinessRubric.concepts[1].conceptId=x.readinessRubric.concepts[0].conceptId,
 (x:any)=>x.readinessRubric.concepts[0].pageSha256='0'.repeat(64),
 (x:any)=>x.readinessRubric.concepts[0].levels.pop(),
 (x:any)=>x.readinessRubric.concepts[0].levels[0]='',
 (x:any)=>x.readinessRubric.extra=true,
 (x:any)=>x.capabilities=x.capabilities.filter((s:string)=>s!=='taskmaster-readiness-v1'),
 ];
 for(const mutate of mutations){const f=fixture();mutate(f.core);assert.throws(()=>f.activate());}
});
test('format one cannot advertise readiness without its signed format two descriptor', () => {
 const f=fixture();f.core.formatVersion=1;assert.throws(()=>f.activate());
});
test('new selectors admit format two while preserving policy and format compatibility',()=>{
 const entry=(formatVersion:number,sequence:number)=>({formatVersion,privacyPolicyVersion:1,sequence,version:`v${sequence}`,sha256:'0'.repeat(64),path:`bundles/v${sequence}.json`});
 assert.equal(selectKnowledgeEntry({formatVersion:1,bundles:[entry(1,1),entry(2,2),entry(3,3)]}).formatVersion,2);
});
