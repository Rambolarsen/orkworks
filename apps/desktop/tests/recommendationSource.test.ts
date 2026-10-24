import test from "node:test";
import assert from "node:assert/strict";
import { createRecommendationSource } from "../src/recommendationSource.ts";

const response = { recommendations: [], diagnostics: [] };
function deferred<T>() { let resolve!: (value:T)=>void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; }

test("two consumers share one in-flight list and fallback timer", async () => {
  const pending = deferred<typeof response>(); let loads = 0;
  const timers = new Map<unknown,()=>void>(); let id = 0;
  const source = createRecommendationSource({ load: () => { loads++; return pending.promise; }, scheduler: { set: fn => { timers.set(++id,fn);return id; }, clear: id => { timers.delete(id); } } });
  const offA = source.subscribe(() => {}), offB = source.subscribe(() => {});
  const refresh = source.refresh(); await Promise.resolve(); assert.equal(loads,1);
  pending.resolve(response); await refresh;
  assert.equal(timers.size,1); offA(); assert.equal(timers.size,1); offB(); assert.equal(timers.size,0);
});

test("invalidation during a request schedules exactly one trailing refresh; healthy stream stops polling", async () => {
  const first = deferred<typeof response>(); let loads = 0;
  const timers = new Map<unknown,()=>void>(); let id = 0;
  const source = createRecommendationSource({ load: () => { loads++; return loads === 1 ? first.promise : Promise.resolve(response); }, scheduler: { set: fn => { timers.set(++id,fn);return id; }, clear: id => { timers.delete(id); } } });
  const off = source.subscribe(() => {});
  source.setHealthy(true); source.invalidate(); source.invalidate();
  first.resolve(response); await source.refresh(); await Promise.resolve(); await Promise.resolve();
  assert.equal(loads,2); assert.equal(timers.size,0);
  source.setHealthy(false); assert.equal(timers.size,1);
  off(); assert.equal(timers.size,0);
});

test("disposed source never publishes an old workspace response", async () => {
  const pending = deferred<typeof response>(); let publications = 0;
  const source = createRecommendationSource({ load: () => pending.promise });
  const off = source.subscribe(() => { publications++; }); const before = publications;
  off(); pending.resolve(response); await Promise.resolve(); await Promise.resolve();
  assert.equal(publications,before);
});
