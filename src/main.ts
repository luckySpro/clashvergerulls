import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { buildDraft, catalogDiff, escapeHtml as e } from './model.mjs';
import { words, type Key } from './i18n';
import seed from '../rules/catalog.json';
import './style.css';
type Service = {
    id: string;
    name: string;
    source: string;
    domains: string[];
};
type Catalog = {
    schema: number;
    version: string;
    services: Service[];
};
type Rule = {
    domain: string;
    policy: string;
};
type Snapshot = {
    root: string;
    profile: string;
    target: string;
    fingerprint: string;
    groups: string[];
    prepend: string[];
};
type Preview = {
    snapshot: Snapshot;
    before: string;
    after: string;
    warnings: string[];
};
type Receipt = {
    target: string;
    backup: string;
    applied_hash: string;
};
type Sync = {
    catalog: Catalog;
    sha256: string;
    url: string;
};
let language = Math.max(0, ['zh-CN', 'zh-TW', 'en', 'ja'].findIndex(x => navigator.language.startsWith(x)));
let page: 'services' | 'personal' | 'config' | 'sync' = 'services', catalog: Catalog = seed;
let policies: Record<string, string> = {}, overrides: Record<string, string[]> = {}, custom: Rule[] = [], china = true;
let root = '', repo = 'luckySpro/clashvergerulls', branch = 'main', proxy = '', candidates: string[] = [], receipt: Receipt | null = null;
let snapshot: Snapshot | null = null, preview: Preview | null = null, synced: Sync | null = null, search = '', editor: string | null = null;
let dirty = false, busy = false, message = '', error = false;
const t = (k: Key) => words[k][language];
const app = document.querySelector<HTMLDivElement>('#app')!;
const button = (id: string, label: string, primary = false) => `<button id="${id}" class="${primary ? 'primary' : ''}" ${busy ? 'disabled' : ''}>${e(label)}</button>`;
function effective(): Catalog { return { ...catalog, services: catalog.services.map(s => ({ ...s, domains: overrides[s.id] ?? s.domains })) }; }
function state() { return { schema: 1, language, catalog, policies, overrides, custom, china, root, repo, branch, proxy, receipt }; }
function modified() { dirty = true; preview = null; }
function policyOptions(selected: string) { const list = [...new Set(['PROXY', ...(snapshot?.groups ?? []), 'DIRECT', 'REJECT', 'FOLLOW'])]; return list.map(p => `<option value="${e(p)}" ${p === selected ? 'selected' : ''}>${e(p === 'FOLLOW' ? t('follow') : p === 'DIRECT' ? t('direct') : p === 'REJECT' ? t('reject') : p)}</option>`).join(''); }
function currentDraft() { return buildDraft(effective(), policies, custom, china); }
async function run(action: () => Promise<void>) { if (busy)
    return; busy = true; error = false; message = t('busy'); render(); try {
    await action();
}
catch (err) {
    error = true;
    message = String(err);
}
finally {
    busy = false;
    render();
} }
async function save() { await invoke('save_workspace', { value: state() }); dirty = false; message = t('saved'); }
function render() {
    app.innerHTML = `<div class="shell"><aside><div class="brand">Verge<br>Route Studio<small>NETWORK / WORKSPACE</small></div><nav>${(['services', 'personal', 'config', 'sync'] as const).map((k, i) => `<button data-page="${k}" class="${page === k ? 'active' : ''}">${String(i + 1).padStart(2, '0')}　${t(k)}</button>`).join('')}</nav><div class="footer"><select id="language" aria-label="Language">${['简体中文', '繁體中文', 'English', '日本語'].map((n, i) => `<option value="${i}" ${language === i ? 'selected' : ''}>${n}</option>`).join('')}</select>M&J Group · v0.1.0<br>473276@qq.com<br>Local data. Your control.</div></aside><main><header><div><div class="eyebrow">VERGE ROUTE STUDIO</div><h1>${t(page)}</h1><div class="subtle">${t('subtitle')}</div></div><div class="row"><span class="status">${dirty ? t('pending') : t('offline')}</span>${button('save', t('save'))}</div></header>${message ? `<div id="message" role="status" class="${error ? 'error' : ''}">${e(message)}</div>` : ''}<section id="content">${content()}</section></main></div>`;
    app.querySelectorAll<HTMLButtonElement>('[data-page]').forEach(b => b.onclick = () => { page = b.dataset.page as typeof page; render(); });
    bind('language', 'change', () => { language = Number(value('language')); modified(); render(); });
    bind('save', 'click', () => run(save));
    bind('search', 'input', () => { search = value('search'); const start = (document.getElementById('search') as HTMLInputElement).selectionStart; render(); const input = document.getElementById('search') as HTMLInputElement; input.focus(); input.setSelectionRange(start, start); });
    app.querySelectorAll<HTMLSelectElement>('[data-policy]').forEach(b => b.onchange = () => { policies[b.dataset.policy!] = b.value; modified(); render(); });
    app.querySelectorAll<HTMLButtonElement>('[data-edit]').forEach(b => b.onclick = () => { editor = b.dataset.edit!; render(); });
    app.querySelectorAll<HTMLButtonElement>('[data-remove]').forEach(b => b.onclick = () => { custom.splice(Number(b.dataset.remove), 1); modified(); render(); });
    bind('cancel', 'click', () => { editor = null; render(); });
    bind('reset', 'click', () => { if (editor)
        delete overrides[editor]; editor = null; modified(); render(); });
    bind('edit-save', 'click', () => { const ds = value('domains-edit').split(/\s+/).filter(Boolean); if (!ds.length || !ds.every(validDomain) || new Set(ds).size !== ds.length) {
        message = t('invalid');
        error = true;
        render();
        return;
    } overrides[editor!] = ds; editor = null; modified(); message = t('updated'); render(); });
    bind('add', 'click', () => { const domain = value('domain').trim().toLowerCase(); if (!validDomain(domain)) {
        message = t('invalid');
        error = true;
        render();
        return;
    } custom = custom.filter(r => r.domain !== domain); custom.push({ domain, policy: value('custom-policy') }); modified(); render(); });
    bind('china', 'change', () => { china = (document.getElementById('china') as HTMLInputElement).checked; modified(); render(); });
    bind('root', 'input', () => { root = value('root'); snapshot = null; modified(); });
    bind('detect', 'change', () => { root = value('detect'); snapshot = null; modified(); render(); });
    bind('read', 'click', () => run(async () => { const loaded = await invoke<Snapshot>('inspect', { root }); snapshot = loaded; root = loaded.root; preview = null; message = t('readOk'); }));
    bind('preview', 'click', () => run(async () => { preview = await invoke('preview', { root, draft: currentDraft() }); snapshot = preview!.snapshot; message = t('preview'); }));
    bind('apply', 'click', () => run(async () => { if (!preview)
        throw new Error('Preview required'); receipt = await invoke('apply', { root, draft: currentDraft(), fingerprint: preview.snapshot.fingerprint }); preview = null; modified(); await save(); snapshot = await invoke('inspect', { root }); message = t('applied'); }));
    bind('rollback', 'click', () => run(async () => { await invoke('rollback', { root, receipt }); receipt = null; preview = null; await save(); snapshot = await invoke('inspect', { root }); message = t('rolled'); }));
    for (const id of ['repo', 'branch'])
        bind(id, 'input', () => { repo = value('repo'); branch = value('branch'); synced = null; modified(); });
    bind('proxy', 'input', () => { proxy = value('proxy'); modified(); });
    bind('fetch', 'click', () => run(async () => { synced = await invoke('sync_catalog', { repo, branch, proxy }); message = t('preview'); }));
    bind('accept', 'click', () => { if (synced) {
        const retained = catalog.services.filter(s => overrides[s.id] && !synced!.catalog.services.some(n => n.id === s.id));
        catalog = { ...synced.catalog, services: [...synced.catalog.services, ...retained] };
        synced = null;
        modified();
        message = t('accepted');
        render();
    } });
    if (busy)
        app.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLSelectElement | HTMLTextAreaElement>('button,input,select,textarea').forEach(x => x.disabled = true);
}
function validDomain(s: string) { return s.length <= 253 && s.includes('.') && !/^\d+\.\d+\.\d+\.\d+$/.test(s) && s.split('.').every(p => /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(p)); }
function value(id: string) { return (document.getElementById(id) as HTMLInputElement).value; }
function bind(id: string, event: string, fn: () => unknown) { document.getElementById(id)?.addEventListener(event, fn); }
function content(): string {
    if (page === 'services') {
        const services = effective().services.filter(s => (s.name + ' ' + s.domains.join(' ')).toLowerCase().includes(search.toLowerCase()));
        return `<div class="stats">${[[catalog.services.length, t('serviceCount')], [effective().services.reduce((n, s) => n + s.domains.length, 0), t('domainCount')], [custom.length, t('customCount')]].map(([n, l]) => `<div class="card"><div class="subtle">${l}</div><div class="count">${n}</div></div>`).join('')}</div><p class="subtle">${t('policyHint')}</p><div class="toolbar"><input id="search" type="text" aria-label="${t('search')}" placeholder="${t('search')}" value="${e(search)}"></div>${editor ? `<div class="card"><h2>${e(catalog.services.find(s => s.id === editor)?.name ?? '')}</h2><p class="subtle">${t('domainsHint')}</p><textarea id="domains-edit" aria-label="${t('domain')}">${e(effective().services.find(s => s.id === editor)?.domains.join('\n') ?? '')}</textarea><div class="toolbar">${button('edit-save', t('confirm'), true)}${button('reset', t('reset'))}${button('cancel', t('cancel'))}</div></div>` : ''}<div class="grid">${services.map(s => `<article class="card"><div class="service-head"><h2>${e(s.name)}</h2><select aria-label="${e(s.name)} ${t('policy')}" data-policy="${e(s.id)}">${policyOptions(policies[s.id] ?? 'PROXY')}</select></div><div class="domains">${s.domains.map(d => `<span class="domain">${e(d)}</span>`).join('')}</div><div class="row between"><span class="subtle">${s.domains.length} ${t('domain')}</span><button data-edit="${e(s.id)}">${t('edit')}</button></div><details><summary>Source</summary><div class="source">${e(s.source)}</div></details></article>`).join('')}</div>`;
    }
    if (page === 'personal')
        return `<div class="card"><h2>${t('personal')}</h2><div class="row"><input id="domain" type="text" placeholder="macked.app" aria-label="${t('domain')}"><select id="custom-policy" aria-label="${t('policy')}">${policyOptions('PROXY')}</select>${button('add', t('add'), true)}</div><table><thead><tr><th>${t('domain')}</th><th>${t('policy')}</th><th></th></tr></thead><tbody>${custom.map((r, i) => `<tr><td>${e(r.domain)}</td><td>${e(r.policy)}</td><td><button data-remove="${i}">${t('remove')}</button></td></tr>`).join('')}</tbody></table>${!custom.length ? `<div class="empty">${t('empty')}</div>` : ''}</div><div class="card"><label class="check"><input type="checkbox" id="china" ${china ? 'checked' : ''}>${t('china')}</label><p class="subtle">${t('order')}</p></div>`;
    if (page === 'config')
        return `<div class="card"><h2>${t('root')}</h2><p class="subtle">${t('configHint')}</p><div class="field"><input id="root" type="text" value="${e(root)}" aria-label="${t('root')}"></div><div class="row"><select id="detect" aria-label="${t('detect')}"><option value="${e(root)}">${t('detect')}</option>${candidates.map(c => `<option value="${e(c)}">${e(c)}</option>`).join('')}</select>${button('read', t('read'))}${button('preview', t('preview'), true)}</div></div>${snapshot ? `<div class="card"><div class="subtle">${t('profile')}</div><h2>${e(snapshot.profile)}</h2><div class="subtle">${t('target')}</div><p class="path">${e(snapshot.target)}</p><details><summary>${t('existing')} (${snapshot.prepend.length})</summary><pre>${e(snapshot.prepend.join('\n'))}</pre></details></div>` : ''}${preview ? `<div class="notice">${preview.warnings.map(e).join('\n')}</div><div class="grid"><div><h2>${t('before')}</h2><pre>${e(preview.before)}</pre></div><div><h2>${t('after')}</h2><pre>${e(preview.after)}</pre></div></div><div class="toolbar">${button('apply', t('apply'), true)}${button('cancel-preview', t('cancel'))}</div>` : ''}${receipt ? `<div class="card"><p class="subtle">${t('backup')}</p><p class="path">${e(receipt.backup)}</p>${button('rollback', t('rollback'))}</div>` : ''}`;
    const diff = synced ? catalogDiff(catalog, synced.catalog) : null;
    return `<div class="card"><h2>GitHub → ${t('serviceCount')}</h2><p class="subtle">${t('syncHint')}</p><div class="grid"><label class="field"><span>${t('repository')}</span><input type="text" id="repo" value="${e(repo)}"></label><label class="field"><span>${t('branch')}</span><input type="text" id="branch" value="${e(branch)}"></label></div><label class="field"><span>${t('proxyInput')}</span><input type="text" id="proxy" placeholder="http://127.0.0.1:7897" value="${e(proxy)}"></label><div class="toolbar">${button('fetch', t('fetch'), true)}<span class="status">${t('current')}: ${e(catalog.version)}</span></div></div>${synced && diff ? `<div class="card"><h2>${e(catalog.version)} → ${e(synced.catalog.version)}</h2><div class="path">${e(synced.url)}<br>SHA-256: ${e(synced.sha256)}</div><div class="grid"><div><p>${t('added')} (${diff.added.length})</p><pre>${e(diff.added.join('\n') || '—')}</pre></div><div><p>${t('removed')} (${diff.removed.length})</p><pre>${e(diff.removed.join('\n') || '—')}</pre></div></div><details><summary>JSON</summary><pre>${e(JSON.stringify(synced.catalog, null, 2))}</pre></details>${button('accept', t('accept'), true)}</div>` : ''}`;
}
app.addEventListener('click', evt => { if ((evt.target as HTMLElement).id === 'cancel-preview') {
    preview = null;
    render();
} });
render();
if (isTauri()) {
    await run(async () => { const boot = await invoke<{
        candidates: string[];
        catalog: Catalog;
        saved: ReturnType<typeof state> | null;
    }>('bootstrap'); candidates = boot.candidates; root = candidates[0] ?? ''; catalog = boot.catalog; if (boot.saved?.schema === 1) {
        const s = boot.saved;
        language = s.language >= 0 && s.language < 4 ? s.language : 0;
        catalog = s.catalog;
        policies = s.policies;
        overrides = s.overrides;
        custom = s.custom;
        china = s.china;
        root = s.root;
        repo = s.repo;
        branch = s.branch;
        proxy = s.proxy ?? '';
        receipt = s.receipt;
    } message = ''; });
    await getCurrentWindow().onCloseRequested(async (event) => { if (busy) {
        event.preventDefault();
        return;
    } if (dirty && !window.confirm(t('unsaved')))
        event.preventDefault(); });
}
else {
    message = t('nativeOnly');
    render();
}
