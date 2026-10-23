// The Hello package's card: what one Hello source has written, newest
// first, from its applet's /greetings endpoint.

export default function greetings(appletId) {
  if (typeof appletId !== "string" || !/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(appletId)) {
    throw new Error('greetings takes the applet\'s id, as in greetings("my_hello_greetings")');
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

    function show({ source, greetings }) {
      ctx.setTitle(`Greetings · ${source}`);
      wrap.replaceChildren();
      if (!greetings.length) {
        wrap.appendChild(el("div", "hg-note", `${source} has not run yet. Sync it.`));
        return;
      }
      for (const g of greetings) {
        const row = el("div", "hg-row", "");
        row.appendChild(el("div", "hg-text", g.greeting || "(no greeting)"));
        const when = g.at ? new Date(g.at).toLocaleString() : "";
        row.appendChild(el("div", "hg-meta", when));
        wrap.appendChild(row);
      }
    }

    ctx.setTitle("Greetings");
    wrap.appendChild(el("div", "hg-note", "Loading…"));
    fetch(url)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`${url}: HTTP ${r.status}`))))
      .then((body) => !cancelled && show(body))
      .catch((e) => {
        if (cancelled) return;
        wrap.replaceChildren(el("div", "hg-note", String(e.message || e)));
      });

    return () => {
      cancelled = true;
    };
  };
}
