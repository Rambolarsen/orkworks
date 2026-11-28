import test from "node:test";
import assert from "node:assert/strict";
import { readGitChanges } from "../electron/gitChanges.ts";

test("changes transport validates inputs before reaching the backend", async () => {
  const fetchImpl: typeof fetch = async () => { throw new Error("must not fetch"); };
  const signal = new AbortController().signal;
  for (const input of [
    {sessionId:"../x",kind:"bases"},
    {sessionId:"s",kind:"capture",comparison:"branch",baseRef:"main~1"},
    {sessionId:"s",kind:"capture",comparison:"uncommitted",commitOid:"a".repeat(40)},
    {sessionId:"s",kind:"capture",comparison:"branch",commitOid:"a".repeat(40)},
    {sessionId:"s",kind:"capture",comparison:"branch",baseRef:"refs/heads/main",commitOid:"HEAD~1"},
    {sessionId:"s",kind:"capture",comparison:"branch",baseRef:"refs/heads/main",commitOid:"A".repeat(40)},
    {sessionId:"s",kind:"file",snapshotId:"../../",fileId:"x"},
  ]) {
    await assert.rejects(readGitChanges("http://127.0.0.1:1", input, "token", fetchImpl, signal), /Invalid/);
  }
});
test("changes transport authenticates, encodes exact refs and commitOids, and propagates unavailable errors", async () => {
  const requests: {url:string,init?:RequestInit}[]=[];
  const signal=new AbortController().signal;
  const commitOid = "0123456789abcdef0123456789abcdef01234567";
  const result=await readGitChanges("http://127.0.0.1:1",{sessionId:"s",kind:"capture",comparison:"branch",baseRef:"refs/heads/main",commitOid},"secret",async (url,init)=>{
    requests.push({url:String(url),init});return new Response(JSON.stringify({snapshotId:"capture",files:[]}));
  },signal);
  assert.deepEqual(result,{snapshotId:"capture",files:[]});
  assert.equal(requests[0].url,`http://127.0.0.1:1/sessions/s/git-changes?comparison=branch&baseRef=refs%2Fheads%2Fmain&commitOid=${commitOid}`);
  assert.deepEqual(requests[0].init,{headers:{"x-orkworks-open-plan-token":"secret"},signal});
  await assert.rejects(readGitChanges("http://127.0.0.1:1",{sessionId:"s",kind:"bases"},"secret",async()=>new Response(JSON.stringify({error:"No repository"}),{status:422}),signal),/No repository/);
});
