export function buildDraft(catalog, policies, custom, chinaDirect) {
  const rules = new Map();
  for (const s of catalog.services) {
    const policy = policies[s.id] ?? 'PROXY';
    if (policy === 'FOLLOW') continue;
    for (const domain of s.domains) rules.set(domain, { domain, policy });
  }
  // Explicit personal rules always win over downloaded service defaults.
  for (const r of custom) {
    if (r.policy === 'FOLLOW') rules.delete(r.domain);
    else rules.set(r.domain, r);
  }
  return { rules: [...rules.values()], china_direct: chinaDirect };
}
export function catalogDiff(before, after) {
  const flatten = c => new Set(c.services.flatMap(s => s.domains.map(d => `${s.id}: ${d}`)));
  const a = flatten(before), b = flatten(after);
  return { added: [...b].filter(x => !a.has(x)), removed: [...a].filter(x => !b.has(x)) };
}
export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}
