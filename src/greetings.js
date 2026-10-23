// The Hello package's card: what each Hello source in the library last
// wrote, from the applet's /greetings endpoint.

export default function greetings(appletId) {
  if (typeof appletId !== "string" || !/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(appletId)) {
    throw new Error('greetings takes the applet\'s id, as in greetings("hello_greetings")');
  }
  const url = `/applet/${appletId}/greetings`;

  return (root, ctx) => {
    let cancelled = false;
    const style = document.createElement("style");
    style.textContent = `
      .hg { font: 13px/1.5 system-ui, -apple-system, sans-serif; color: var(--datalib-fg, inherit); }
      .hg-row { padding: 8px 12px; border-bottom: 1px solid var(--datalib-border, #8882); }
      .hg-text { font-size: 15px; }
      .hg-meta { opacity: .55; font-size: 11px; font-variant-numeric: tabular-nums; }
      .hg-note { padding: 12px; opacity: .7; white-space: pre-wrap; }
    `;
    root.appendChild(style);
    const wrap = document.createElement("div");
    wrap.className = "hg";
    root.appendChild(wrap);

    function el(tag, cls, text) {
      const e = document.createElement(tag);
      e.className = cls;
      e.textContent = text;
      return e;
    }

    function show(rows) {
      wrap.replaceChildren();
      if (!rows.length) {
        wrap.appendChild(el("div", "hg-note", "No Hello source has run yet. Add one, then sync it."));
        return;
      }
      for (const r of rows) {
        const row = el("div", "hg-row", "");
        row.appendChild(el("div", "hg-text", r.greeting || "(no greeting)"));
        const when = r.written_ms ? new Date(r.written_ms).toLocaleString() : "";
        row.appendChild(el("div", "hg-meta", when ? `${r.source} · ${when}` : r.source));
        wrap.appendChild(row);
      }
    }

    ctx.setTitle("Greetings");
    wrap.appendChild(el("div", "hg-note", "Loading…"));
    fetch(url)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`${url}: HTTP ${r.status}`))))
      .then((rows) => !cancelled && show(rows))
      .catch((e) => {
        if (cancelled) return;
        wrap.replaceChildren(el("div", "hg-note", String(e.message || e)));
      });

    return () => {
      cancelled = true;
    };
  };
}
