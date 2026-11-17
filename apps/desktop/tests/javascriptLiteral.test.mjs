import assert from 'node:assert/strict';
import { runInNewContext } from 'node:vm';
import test from 'node:test';
import { javascriptLiteral } from './helpers/javascriptLiteral.mjs';

test('JavaScript literals cannot terminate an HTML script element', () => {
  const value = '</ScRiPt><script>globalThis.injected = true</script><!--';
  const literal = javascriptLiteral(value);
  assert.doesNotMatch(literal, /[<>\u2028\u2029]/u);
  const context = {};
  runInNewContext(`globalThis.result = ${literal}`, context);
  assert.equal(context.result, value);
  assert.equal(context.injected, undefined);
});

test('JavaScript literals preserve quotes, controls, separators and Unicode', () => {
  const value = '"quoted" \\path\n\r\t\0\u2028\u2029 æ 😀 <>&';
  const literal = javascriptLiteral(value);
  assert.doesNotMatch(literal, /[<>\u2028\u2029]/u);
  assert.equal(runInNewContext(literal), value);
});

test('JavaScript literals preserve nested JSON values', () => {
  const value = { text: '</script>', items: [null, true, 42, { path: 'C:\\tmp' }] };
  const literal = javascriptLiteral(value);
  assert.doesNotMatch(literal, /[<>\u2028\u2029]/u);
  assert.deepEqual(JSON.parse(literal), value);
});
