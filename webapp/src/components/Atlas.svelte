<script>
  import { atlasCharts, atlasNodes, addToast } from '../lib/store.js';
  import { pushEvent } from '../lib/sync.js';
  import AtlasBranch from './AtlasBranch.svelte';

  export let projectId;
  export let api;
  export let readOnly = false;

  let activeChartId = null;
  let title = '';
  let showHidden = false;

  $: charts = [...$atlasCharts.values()].filter(c => c.project_id === projectId)
    .sort((a, b) => a.name.localeCompare(b.name));
  $: activeChart = activeChartId ? $atlasCharts.get(activeChartId) : null;
  $: chartNodes = [...$atlasNodes.values()].filter(n => n.chart_id === activeChartId);
  $: rows = flatten(chartNodes.filter(n => showHidden || !n.hidden), true);
  $: childMap = mapChildren(chartNodes.filter(n => showHidden || !n.hidden));
  $: roots = childMap.get('root') || [];

  function mapChildren(nodes) {
    const children = new Map();
    for (const node of nodes) {
      const key = node.parent_id || 'root';
      if (!children.has(key)) children.set(key, []);
      children.get(key).push(node);
    }
    for (const list of children.values()) list.sort((a, b) => (a.position ?? 0) - (b.position ?? 0) || String(a.created_at).localeCompare(String(b.created_at)));
    return children;
  }

  function flatten(nodes, honorCollapsed) {
    const children = new Map();
    for (const node of nodes) {
      const key = node.parent_id || 'root';
      if (!children.has(key)) children.set(key, []);
      children.get(key).push(node);
    }
    for (const list of children.values()) list.sort((a, b) => (a.position ?? 0) - (b.position ?? 0) || String(a.created_at).localeCompare(String(b.created_at)));
    const out = [];
    const walk = (parent, depth) => {
      for (const node of children.get(parent || 'root') || []) {
        out.push({ node, depth });
        if (!honorCollapsed || !node.collapsed) walk(node.id, depth + 1);
      }
    };
    walk(null, 0);
    return out;
  }

  async function createChart() {
    if (readOnly || !title.trim()) return;
    const now = new Date().toISOString();
    const chart = { id: crypto.randomUUID(), project_id: projectId, name: title.trim(), created_at: now, updated_at: now };
    const root = { id: crypto.randomUUID(), chart_id: chart.id, project_id: projectId, parent_id: null, title: chart.name, position: 0, collapsed: false, marked: false, hidden: false, color: null, created_at: now, updated_at: now };
    atlasCharts.update(m => new Map(m).set(chart.id, chart));
    atlasNodes.update(m => new Map(m).set(root.id, root));
    await pushEvent(api, 'atlas_chart', chart.id, 'create', chart);
    await pushEvent(api, 'atlas_node', root.id, 'create', root);
    activeChartId = chart.id; title = '';
  }

  async function addNode(parentId = null) {
    if (readOnly) return;
    const value = window.prompt(parentId ? 'Name this branch' : 'Name this root branch');
    if (!value?.trim()) return;
    const siblings = chartNodes.filter(n => (n.parent_id || null) === parentId);
    const now = new Date().toISOString();
    const node = { id: crypto.randomUUID(), chart_id: activeChartId, project_id: projectId, parent_id: parentId, title: value.trim(), position: Math.max(-1, ...siblings.map(n => n.position ?? 0)) + 1, collapsed: false, marked: false, hidden: false, color: chartNodes.find(n => n.id === parentId)?.color ?? null, created_at: now, updated_at: now };
    atlasNodes.update(m => new Map(m).set(node.id, node));
    await pushEvent(api, 'atlas_node', node.id, 'create', node);
  }

  async function saveNode(node, patch) {
    if (readOnly) return;
    const updated = { ...node, ...patch, updated_at: new Date().toISOString() };
    atlasNodes.update(m => new Map(m).set(node.id, updated));
    await pushEvent(api, 'atlas_node', node.id, 'update', updated);
  }

  async function colorBranch(node) {
    if (readOnly) return;
    const answer = window.prompt('Branch color: 0 Default, 1 Red, 2 Orange, 3 Amber, 4 Yellow, 5 Lime, 6 Green, 7 Teal, 8 Cyan, 9 Sky, 10 Blue, 11 Indigo, 12 Violet, 13 Purple, 14 Pink, 15 Rose, 16 White', node.color == null ? '0' : String(node.color + 1));
    if (answer == null || !/^(?:[0-9]|1[0-6])$/.test(answer.trim())) return;
    const color = Number(answer) === 0 ? null : Number(answer) - 1;
    const ids = new Set([node.id]);
    let changed = true;
    while (changed) { changed = false; for (const child of chartNodes) if (ids.has(child.parent_id) && !ids.has(child.id)) { ids.add(child.id); changed = true; } }
    for (const id of ids) {
      const branch = chartNodes.find(n => n.id === id);
      if (branch) await saveNode(branch, { color });
    }
  }

  async function removeNode(node) {
    if (readOnly || !confirm(`Remove “${node.title}” and all its branches?`)) return;
    const ids = new Set([node.id]);
    let changed = true;
    while (changed) { changed = false; for (const n of chartNodes) if (n.parent_id && ids.has(n.parent_id) && !ids.has(n.id)) { ids.add(n.id); changed = true; } }
    atlasNodes.update(m => { const n = new Map(m); for (const id of ids) n.delete(id); return n; });
    for (const id of ids) await pushEvent(api, 'atlas_node', id, 'delete', { project_id: projectId, chart_id: activeChartId });
  }

  async function removeChart(chart) {
    if (readOnly || !confirm(`Remove Chart “${chart.name}”?`)) return;
    const nodes = [...$atlasNodes.values()].filter(n => n.chart_id === chart.id);
    atlasNodes.update(m => { const n = new Map(m); for (const node of nodes) n.delete(node.id); return n; });
    atlasCharts.update(m => { const n = new Map(m); n.delete(chart.id); return n; });
    for (const node of nodes) await pushEvent(api, 'atlas_node', node.id, 'delete', node);
    await pushEvent(api, 'atlas_chart', chart.id, 'delete', chart);
    activeChartId = null;
  }

  function text(format) {
    const all = flatten(chartNodes, false);
    if (format === 'text') return all.map(({node, depth}) => `${'\t'.repeat(depth)}${node.marked ? '✓ ' : ''}${node.title}`).join('\n') + '\n';
    return `# ${activeChart.name}\n\n` + all.map(({node, depth}) => `${'  '.repeat(depth)}- ${node.marked ? '[x] ' : ''}${node.title}`).join('\n') + '\n';
  }

  function html() {
    const escape = s => s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
    const render = parent => { const kids = chartNodes.filter(n => (n.parent_id || null) === parent).sort((a,b)=>a.position-b.position); return kids.length ? `<ul>${kids.map(n=>`<li class="${n.marked?'marked':''}">${escape(n.title)}${render(n.id)}</li>`).join('')}</ul>` : ''; };
    return `<!doctype html><html><head><meta charset="utf-8"><title>${escape(activeChart.name)}</title><style>body{background:#09090b;color:#e4e4e7;font:16px system-ui;margin:3rem}h1{color:#f59e0b}ul{list-style:none;padding-left:2rem;border-left:1px solid #3f3f46}li{margin:.55rem 0}li:before{content:'◆';color:#a855f7;margin-right:.65rem}.marked{text-decoration:line-through;color:#a1a1aa}</style></head><body><h1>${escape(activeChart.name)}</h1>${render(null)}</body></html>`;
  }

  function download(name, content, type) { const a=document.createElement('a'); a.href=URL.createObjectURL(new Blob([content],{type})); a.download=name; a.click(); setTimeout(()=>URL.revokeObjectURL(a.href),500); }
  function slug() { return activeChart.name.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'') || 'chart'; }
  async function copyText() { await navigator.clipboard.writeText(text('text')); addToast('Chart copied to clipboard', 'success'); }
  function exportPdf() { const win=window.open('', '_blank'); if (!win) return addToast('Allow popups to export PDF', 'error'); win.document.write(html()); win.document.close(); setTimeout(()=>win.print(),250); }
</script>

<section class="atlas">
  {#if !activeChart}
    <header><div><h2>Campaign Atlas</h2><p>Chart ideas, paths, and possibilities.</p></div></header>
    {#if !readOnly}<form on:submit|preventDefault={createChart}><input bind:value={title} placeholder="Name a new Chart…"><button>Create Chart</button></form>{/if}
    <div class="charts">{#each charts as chart (chart.id)}<article><button class="open" on:click={() => activeChartId=chart.id}>◈ {chart.name}<small>{[...$atlasNodes.values()].filter(n=>n.chart_id===chart.id).length} branches</small></button>{#if !readOnly}<button class="danger" on:click={() => removeChart(chart)}>×</button>{/if}</article>{:else}<div class="empty">The Atlas is blank. Chart the first path.</div>{/each}</div>
  {:else}
    <header><button class="back" on:click={() => activeChartId=null}>← Atlas</button><div><h2>The Cartographer · {activeChart.name}</h2><p>{rows.length} visible branches</p></div><div class="exports"><button on:click={() => showHidden = !showHidden}>{showHidden ? 'Hide hidden' : 'Show hidden'}</button><button on:click={copyText}>Copy</button><button on:click={()=>download(`${slug()}.md`,text('md'),'text/markdown')}>Markdown</button><button on:click={()=>download(`${slug()}.txt`,text('text'),'text/plain')}>Text</button><button on:click={()=>download(`${slug()}.html`,html(),'text/html')}>HTML</button><button on:click={exportPdf}>PDF</button></div></header>
    {#if !readOnly}<button class="add-root" on:click={() => addNode(null)}>+ Root branch</button>{/if}
    <div class="mind-map">
      {#each roots as root (root.id)}
        <AtlasBranch node={root} children={childMap} {readOnly} onSave={saveNode} onAdd={addNode} onRemove={removeNode} onColor={colorBranch} root />
      {:else}
        <div class="empty">This chart has no central thought yet.</div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .atlas{padding:.5rem;color:#d4d4d4}header{display:flex;align-items:center;gap:1rem;justify-content:space-between;margin-bottom:1rem}h2{margin:0;color:#f59e0b;font-size:1rem;text-transform:uppercase;letter-spacing:.08em}p{margin:.25rem 0;color:#666;font-size:.78rem}form{display:flex;gap:.5rem;margin-bottom:1rem}input{background:#050505;border:1px solid #2a2a2a;border-radius:5px;color:#ddd;padding:.55rem .7rem;font:inherit}form input{flex:1}button{background:#111;border:1px solid #2a2a2a;border-radius:5px;color:#a855f7;padding:.45rem .65rem;cursor:pointer;font:inherit}.charts article{display:flex;border:1px solid #202020;border-radius:6px;margin:.5rem 0}.open{flex:1;text-align:left;border:0;color:#ddd;padding:.8rem}.open small{float:right;color:#555}.danger{border:0;color:#ef4444}.empty{text-align:center;color:#555;padding:3rem}.exports{display:flex;gap:.3rem;flex-wrap:wrap;justify-content:flex-end}.back{color:#06b6d4}.add-root{margin-bottom:.75rem}.mind-map{display:flex;align-items:center;gap:2rem;min-height:18rem;overflow:auto;border:1px solid #202020;border-radius:7px;padding:1.5rem;background-image:radial-gradient(#27272a 1px,transparent 1px);background-size:18px 18px}
</style>
