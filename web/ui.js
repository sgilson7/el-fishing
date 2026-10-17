// The HTML over the game: panels, the tension ring, the arm keys, the
// scope's circle and the message line, all built from the core's view
// model. The page does not know what a panel means; it draws what the core
// describes and sends each button's action back through `act`.

const make = (tag, className, text) => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

export function makeUi(el, act) {
  let hints = [];
  const seen = { panels: "", arms: "", keys: "" };

  // a label whose text is a glossary term shows the definition on hover
  const mark = (node) => {
    const hint = hints.find((h) => h.term.toLowerCase() === node.textContent.toLowerCase());
    if (hint) {
      node.dataset.term = hint.term;
      node.title = hint.definition;
    }
    return node;
  };

  const button = (a) => {
    const b = make("button", a.primary ? "btn go" : "btn soft", a.label);
    b.addEventListener("click", () => act(a.name, a.value));
    return b;
  };

  function panel(p) {
    const box = make("section", "panel");
    box.id = p.id;
    if (p.eyebrow) box.append(make("div", "eyebrow", p.eyebrow));
    if (p.heading) box.append(mark(make("h2", "", p.heading)));
    if (p.text) box.append(make("p", "", p.text));
    for (const [label, value] of p.rows || []) {
      const row = make("div", "row");
      row.append(mark(make("span", "", label)), make("span", "value", value));
      box.append(row);
    }
    if (p.list) {
      const list = make("ul", "list");
      for (const item of p.list) list.append(make("li", "", item));
      box.append(list);
    }
    for (const [label, choice] of p.choices || []) {
      const row = make("div", "row");
      const opts = make("span", "chips");
      for (const o of choice.options) {
        const chip = make("button", o.on ? "chip on" : "chip", o.label);
        chip.addEventListener("click", () => act(choice.name, o.value));
        opts.append(chip);
      }
      row.append(make("span", "", label), opts);
      box.append(row);
    }
    // a dropdown: the same kind of choice as chips, for a place's weathers (T18)
    for (const [label, choice] of p.selects || []) {
      const row = make("div", "row");
      const pick = make("select", "pick");
      pick.setAttribute("aria-label", label);
      for (const o of choice.options) {
        const opt = make("option", "", o.label);
        opt.value = o.value;
        opt.selected = o.on;
        pick.append(opt);
      }
      pick.addEventListener("change", () => act(choice.name, pick.value));
      row.append(make("span", "", label), pick);
      box.append(row);
    }
    if (p.actions) {
      const row = make("div", "actions");
      for (const a of p.actions) row.append(button(a));
      box.append(row);
    }
    return box;
  }

  function keyCap(k) {
    return make("span", k.on ? "key on" : "key", k.key);
  }

  function arms(a) {
    const box = el("arms");
    const grid = make("div", "arm-grid");
    for (const [label, keys] of a.rows) {
      grid.append(make("span", "", label));
      const cell = make("span", "cell");
      for (const k of keys) cell.append(k.key ? keyCap(k) : make("small", "muted", k.note));
      grid.append(cell);
    }
    box.replaceChildren(make("div", "eyebrow", a.heading), grid);
  }

  function keys(list) {
    const bar = el("keys");
    bar.replaceChildren(
      ...list.map(([k, label]) => {
        const item = make("span", "item");
        for (const part of k.split(" ")) item.append(make("span", "key", part));
        item.append(make("span", "", label));
        return item;
      }),
    );
  }

  function scope(s) {
    const box = el("scope");
    box.hidden = !s.on;
    box.querySelector(".scope-zoom").textContent = s.zoom;
    box.querySelector(".scope-fish").textContent = s.fish || "";
    box.querySelector(".scope-status").textContent = s.status || "";
    const zone = box.querySelector(".zone");
    zone.hidden = !s.ring;
    zone.classList.toggle("inside", !!s.inside);
    if (s.ring) {
      zone.style.setProperty("--x", s.ring.x + "%");
      zone.style.setProperty("--y", s.ring.y + "%");
      zone.style.setProperty("--r", s.ring.r);
    }
  }

  function ring(t, show) {
    const box = el("ring");
    box.hidden = !show;
    box.dataset.state = t.state;
    box.style.setProperty("--t", t.fraction);
    box.querySelector(".ring-percent").textContent = t.percent + "%";
    box.querySelector(".ring-of").textContent = t.of_break;
    box.querySelector(".ring-label").textContent = t.label;
  }

  // the fight cue under the message: wind, hold or haul (T13)
  function cue(c) {
    const box = el("cue");
    box.hidden = !c;
    if (!c) return;
    box.dataset.state = c.state;
    if (box.textContent !== c.text) box.textContent = c.text;
  }

  // the power wind's battery, flashing while it pulls (T15)
  function battery(b, show) {
    const box = el("battery");
    box.hidden = !show;
    box.dataset.state = b.state;
    box.style.setProperty("--b", b.fraction);
    box.querySelector(".battery-label").textContent = b.label;
    box.querySelector(".battery-status").textContent = b.status;
  }

  return {
    draw(view) {
      hints = view.hints;
      document.body.dataset.screen = view.screen;
      for (const [k, v] of Object.entries(view.data)) document.body.dataset[k] = String(v);
      const panels = JSON.stringify(view.panels);
      if (panels !== seen.panels) {
        seen.panels = panels;
        el("panels").replaceChildren(...view.panels.map(panel));
      }
      const armsNow = JSON.stringify(view.arms);
      if (armsNow !== seen.arms) {
        seen.arms = armsNow;
        arms(view.arms);
      }
      el("arms").hidden = !(view.fishing && view.arms_open);
      const keysNow = JSON.stringify(view.keys);
      if (keysNow !== seen.keys) {
        seen.keys = keysNow;
        keys(view.keys);
      }
      el("keys").hidden = !view.hud;
      ring(view.tension, view.fishing);
      cue(view.cue);
      battery(view.battery, view.fishing);
      scope(view.scope);
      const m = el("message");
      m.hidden = !view.message;
      if (m.textContent !== view.message) m.textContent = view.message;
    },
  };
}
