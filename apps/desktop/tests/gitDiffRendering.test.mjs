import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import React from 'react';
import {renderToStaticMarkup} from 'react-dom/server';

const require = createRequire(import.meta.url);
const compiled = await build({entryPoints:[new URL('../src/components/GitDiff.tsx',import.meta.url).pathname],bundle:true,write:false,platform:'node',format:'cjs',packages:'external'});
const component = {exports:{}};
new Function('require','module','exports',compiled.outputFiles[0].text)(require,component,component.exports);
const GitDiff = component.exports.default;

for (const mode of ['unified','split']) test(`many-line ${mode} patches initially mount a bounded batch with an explicit continuation`, () => {
  const patch = '@@ -0,0 +1,100000 @@\n' + '+\n'.repeat(100000);
  const html = renderToStaticMarkup(React.createElement(GitDiff,{patch,path:'blank.txt',mode}));
  assert.ok((html.match(/class="git-diff-(?:line |pair")/g)||[]).length <= 1000);
  assert.ok(html.length < 1000000, 'bounded markup remains small even near the capture byte limit');
  assert.match(html,/Showing 1,000 of 100,001/);
  assert.match(html,/Show next 1,000/);
});
