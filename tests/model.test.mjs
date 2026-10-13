import test from 'node:test';
import assert from 'node:assert/strict';
import { buildDraft, catalogDiff, escapeHtml } from '../src/model.mjs';
const catalog = { services: [{ id: 'a', domains: ['a.com', 'b.com'] }] };
test('personal policy overrides remote domain', () => assert.deepEqual(buildDraft(catalog, {}, [{ domain: 'a.com', policy: 'DIRECT' }], true).rules, [{ domain: 'a.com', policy: 'DIRECT' }, { domain: 'b.com', policy: 'PROXY' }]));
test('follow omits managed service rules', () => assert.equal(buildDraft(catalog, { a: 'FOLLOW' }, [], false).rules.length, 0));
test('personal follow removes managed exact domain', () => assert.deepEqual(buildDraft(catalog, {}, [{ domain: 'a.com', policy: 'FOLLOW' }], true).rules, [{ domain: 'b.com', policy: 'PROXY' }]));
test('catalog diff exposes removals', () => assert.deepEqual(catalogDiff(catalog, { services: [] }).removed, ['a: a.com', 'a: b.com']));
test('remote content escaped', () => assert.equal(escapeHtml('<img onerror="x">'), '&lt;img onerror=&quot;x&quot;&gt;'));
