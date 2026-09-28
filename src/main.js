/* Overlay clients — logique de l'interface.
 *
 * Tout l'état tient en mémoire : la recherche ne déclenche aucun appel au backend,
 * seules les écritures (CRUD, réglages, import/export) traversent l'IPC. */

"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/** Nombre maximal de fiches rendues simultanément, pour garder un affichage instantané. */
const RENDER_LIMIT = 200;

/** Cibles ouvrables, dans l'ordre d'affichage des boutons d'une fiche. */
const LINK_KINDS = [
  { kind: "devops", label: "DevOps", urlField: "devopsUrl", icon: iconDevops },
  { kind: "github", label: "GitHub", urlField: "githubUrl", icon: iconGithub },
  { kind: "adminCenter", label: "Admin", urlField: "adminCenterUrl", icon: iconAdmin },
  { kind: "folder", label: "Dossier", urlField: null, icon: iconFolder },
];

/** Navigateurs pris en charge, dans l'ordre de présentation. Les logos sont lus dans
 * `icons/browsers/` : il suffit de remplacer ces fichiers pour les changer. */
const BROWSERS = [
  { kind: "edge", label: "Microsoft Edge", profiles: true, logo: "edge.png" },
  { kind: "chrome", label: "Google Chrome", profiles: true, logo: "chrome.png" },
  { kind: "firefox", label: "Mozilla Firefox", profiles: true, logo: "firefox.png" },
  { kind: "firefoxDev", label: "Firefox Developer Edition", profiles: true, logo: "firefox-dev.png" },
  { kind: "brave", label: "Brave", profiles: true, logo: "brave.png" },
  { kind: "vivaldi", label: "Vivaldi", profiles: true, logo: "vivaldi.png" },
  { kind: "opera", label: "Opera", profiles: false, logo: "opera.png" },
  { kind: "system", label: "Navigateur par défaut", profiles: false, logo: "system.svg" },
];

/** Jetons reconnus dans les gabarits d'URL. */
const TEMPLATE_TOKENS = ["tenant", "devopsId", "githubId"];

/** Hauteur maximale d'un menu déroulant, bornée en plus par la place disponible. */
const MENU_MAX_HEIGHT = 500;

/** Apparences proposées dans les réglages. */
const THEMES = [
  { value: "system", main: "Système", sub: "suit le thème de Windows" },
  { value: "light", main: "Clair", sub: "" },
  { value: "dark", main: "Sombre", sub: "" },
];

/** État applicatif. */
const state = {
  customers: [],
  settings: null,
  dataPath: "",
  browsers: [],
  version: "",
  update: null,
  installing: false,
  settingsSection: null,
  filtered: [],
  activeIndex: 0,
  view: "list",
  editingId: null,
  pendingDelete: null,
};

const dom = {
  panel: document.getElementById("panel"),
  backdrop: document.getElementById("backdrop"),
  subtitle: document.getElementById("subtitle"),
  search: document.getElementById("search"),
  clear: document.getElementById("btn-clear"),
  list: document.getElementById("list"),
  empty: document.getElementById("empty"),
  toast: document.getElementById("toast"),
  views: {
    list: document.getElementById("view-list"),
    form: document.getElementById("view-form"),
    settings: document.getElementById("view-settings"),
  },
  form: document.getElementById("form"),
  formPreview: document.getElementById("form-preview"),
  workspaces: document.getElementById("workspaces"),
  links: document.getElementById("links"),
  settingsForm: document.getElementById("settings-form"),
  autostart: document.getElementById("autostart"),
  dataPath: document.getElementById("data-path"),
  browserProfiles: document.getElementById("browser-profiles"),
  settingsTitle: document.getElementById("settings-title"),
  settingsMenu: document.getElementById("settings-menu"),
  settingsSections: document.querySelectorAll(".settings-section"),
  settingsActions: document.getElementById("settings-actions"),
  themeList: document.getElementById("theme-list"),
  hotkeyInput: document.getElementById("hotkey"),
  hotkeyButton: document.getElementById("btn-record-hotkey"),
  hotkeyDialog: document.getElementById("hotkey-dialog"),
  hotkeyPreview: document.getElementById("hotkey-preview"),
  hotkeyMessage: document.getElementById("hotkey-message"),
  updateButton: document.getElementById("btn-update"),
  updateDialog: document.getElementById("update-dialog"),
  updateVersion: document.getElementById("update-version"),
  updateDate: document.getElementById("update-date"),
  updateSize: document.getElementById("update-size"),
  updateFlavor: document.getElementById("update-flavor"),
  updateNotes: document.getElementById("update-notes"),
  updateProgress: document.getElementById("update-progress"),
  updateStatus: document.getElementById("update-status"),
  updateInstall: document.getElementById("btn-update-install"),
  updateLater: document.getElementById("btn-update-later"),
};

/* ------------------------------------------------------------------ outils */

/** Crée un élément avec ses classes, attributs et enfants. */
function el(tag, options = {}, children = []) {
  const node = document.createElement(tag);
  if (options.class) node.className = options.class;
  if (options.text !== undefined) node.textContent = options.text;
  if (options.html !== undefined) node.innerHTML = options.html;
  for (const [name, value] of Object.entries(options.attrs || {})) {
    if (value === null || value === false) continue;
    node.setAttribute(name, value === true ? "" : String(value));
  }
  for (const child of children) node.appendChild(child);
  return node;
}

/** Supprime accents et casse, pour une recherche tolérante. */
function fold(value) {
  return value
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

/** Encode une valeur d'URL comme le fait le backend, pour l'aperçu du formulaire. */
function encodeSegment(value) {
  return encodeURIComponent(value.trim()).replace(
    /[!'()*]/g,
    (character) => "%" + character.charCodeAt(0).toString(16).toUpperCase()
  );
}

/** Applique un gabarit d'URL à une fiche. Retourne `null` si le lien n'est pas calculable. */
function applyTemplate(template, customer, requiredField) {
  if (!template || !customer[requiredField] || !customer[requiredField].trim()) return null;
  const url = template
    .replace(/\{tenant\}/g, encodeSegment(customer.tenant || ""))
    .replace(/\{devopsId\}/g, encodeSegment(customer.devopsId || ""))
    .replace(/\{githubId\}/g, encodeSegment(customer.githubId || ""));
  return /^https?:\/\//i.test(url) ? url : null;
}

let toastTimer = 0;

/** Affiche un message éphémère en bas de l'écran. */
function toast(message, tone = "ok") {
  window.clearTimeout(toastTimer);
  dom.toast.textContent = message;
  dom.toast.className = "toast is-" + tone;
  dom.toast.hidden = false;
  // Un cycle de rendu est nécessaire pour que la transition d'opacité s'applique.
  requestAnimationFrame(() => dom.toast.classList.add("is-visible"));
  toastTimer = window.setTimeout(
    () => {
      dom.toast.classList.remove("is-visible");
      toastTimer = window.setTimeout(() => {
        dom.toast.hidden = true;
      }, 200);
    },
    tone === "error" ? 5200 : 2600
  );
}

/** Exécute une commande backend en convertissant l'erreur en message lisible. */
async function call(command, args) {
  try {
    return await invoke(command, args);
  } catch (error) {
    toast(String(error), "error");
    throw error;
  }
}

/* ------------------------------------------------------------- apparence */

const lightQuery = window.matchMedia("(prefers-color-scheme: light)");

/** Traduit le réglage d'apparence en thème effectif et l'applique au document. */
function applyTheme(mode) {
  const resolved = mode === "system" ? (lightQuery.matches ? "light" : "dark") : mode;
  document.documentElement.dataset.theme = resolved;
}

// En mode « système », suivre Windows en direct, sans rouvrir l'overlay.
lightQuery.addEventListener("change", () => {
  if (state.settings && state.settings.theme === "system") applyTheme("system");
});

/* --------------------------------------------------- liste déroulante maison */

/**
 * Applique une classe d'animation d'entrée, puis la retire au bout de `duration`.
 *
 * Le retrait est confié à un minuteur et non à `animationend` : une image
 * d'animation partant de `opacity: 0` laisserait l'élément invisible pour
 * toujours si le compositeur n'avance pas — ce qui arrive juste après la reprise
 * d'une webview suspendue. Un minuteur JavaScript, lui, se déclenche toujours.
 */
function playEntrance(node, className, duration) {
  node.classList.remove(className);
  // Force un recalcul afin que le retrait puis l'ajout relancent bien
  // l'animation, au lieu d'être fusionnés en un non-évènement.
  void node.offsetWidth;
  node.classList.add(className);
  window.setTimeout(() => node.classList.remove(className), duration);
}

/** Liste déroulante ouverte, s'il y en a une : une seule à la fois. */
let openSelect = null;

/**
 * Construit une liste déroulante thémable dans `container`.
 *
 * Le menu flotte par-dessus le contenu, sous le champ, ou au-dessus quand la place
 * manque en bas. Il est positionné en absolu dans le formulaire qui défile, et non
 * en fixe sur la fenêtre : il suit ainsi le défilement sans être recalculé. Sa
 * hauteur est bornée par la place visible de ce formulaire, ce qui le garde à
 * l'abri du rognage.
 *
 * Chaque option est `{ value, main, sub?, icon? }`, `icon` étant une fonction qui
 * fabrique le nœud de l'icône.
 */
function createSelect(container, onChange) {
  const iconSlot = el("span", { class: "select-icon", attrs: { hidden: true } });
  const main = el("span", { class: "select-main" });
  const sub = el("span", { class: "select-sub" });
  const trigger = el(
    "button",
    {
      class: "select-trigger",
      attrs: {
        type: "button",
        "aria-haspopup": "listbox",
        "aria-expanded": "false",
        "aria-labelledby": container.dataset.labelledBy || null,
      },
    },
    [
      iconSlot,
      el("span", { class: "select-texts" }, [main, sub]),
      el("span", { class: "select-caret", html: iconCaret() }),
    ]
  );
  const menu = el("div", { class: "select-menu", attrs: { role: "listbox", hidden: true } });
  container.replaceChildren(trigger, menu);

  let options = [];
  let value = null;
  let highlighted = 0;

  const controller = {
    /** Remplace les entrées et la valeur sélectionnée. */
    setOptions(nextOptions, nextValue) {
      options = nextOptions;
      controller.setValue(nextValue);
    },
    /** Sélectionne une valeur, en la créant à la volée si elle est inconnue. */
    setValue(nextValue) {
      value = nextValue;
      if (value !== null && !options.some((option) => option.value === value)) {
        options = [{ value, main: value, sub: "inconnu sur cette machine" }, ...options];
      }
      const current = options.find((option) => option.value === value);
      main.textContent = current ? current.main : "—";
      sub.textContent = current && current.sub ? current.sub : "";
      sub.hidden = !current || !current.sub;
      const hasIcon = Boolean(current && current.icon);
      iconSlot.replaceChildren(...(hasIcon ? [current.icon()] : []));
      iconSlot.hidden = !hasIcon;
      if (!menu.hidden) renderOptions();
    },
    /** Grise la liste quand elle n'offre aucun choix réel. */
    setDisabled(disabled) {
      trigger.disabled = disabled;
      if (disabled) close();
    },
    getValue: () => value,
    close,
  };

  function renderOptions() {
    menu.replaceChildren(
      ...options.map((option, index) =>
        el(
          "button",
          {
            class: "select-option" + (index === highlighted ? " is-highlighted" : ""),
            attrs: {
              type: "button",
              role: "option",
              "aria-selected": String(option.value === value),
              "data-value": option.value,
            },
          },
          [
            ...(option.icon ? [el("span", { class: "select-icon" }, [option.icon()])] : []),
            el("span", { class: "select-texts" }, [
              el("span", { class: "select-main", text: option.main }),
              ...(option.sub ? [el("span", { class: "select-sub", text: option.sub })] : []),
            ]),
            el("span", { class: "select-check", html: iconCheck() }),
          ]
        )
      )
    );
  }

  /** Place le menu sous le champ, ou au-dessus s'il y tient mieux. */
  function placeMenu() {
    const scroller = container.closest(".form");
    const bounds = scroller ? scroller.getBoundingClientRect() : { top: 0, bottom: window.innerHeight };
    const anchor = trigger.getBoundingClientRect();
    const gap = 10;
    const below = Math.min(bounds.bottom, window.innerHeight) - anchor.bottom - gap;
    const above = anchor.top - Math.max(bounds.top, 0) - gap;

    menu.style.maxHeight = "";
    const natural = Math.min(menu.scrollHeight, MENU_MAX_HEIGHT);
    const opensUp = natural > below && above > below;
    const room = opensUp ? above : below;
    const height = Math.max(96, Math.min(MENU_MAX_HEIGHT, room));
    menu.style.maxHeight = `${height}px`;
    container.classList.toggle("opens-up", opensUp);
  }

  function open() {
    if (trigger.disabled) return;
    if (openSelect && openSelect !== controller) openSelect.close();
    highlighted = Math.max(0, options.findIndex((option) => option.value === value));
    renderOptions();
    trigger.scrollIntoView({ block: "nearest" });
    menu.hidden = false;
    placeMenu();
    playEntrance(menu, "is-entering", 150);
    container.classList.add("is-open");
    trigger.setAttribute("aria-expanded", "true");
    openSelect = controller;
    menu.querySelector(".is-highlighted")?.scrollIntoView({ block: "nearest" });
  }

  function close() {
    if (menu.hidden) return;
    menu.hidden = true;
    container.classList.remove("is-open", "opens-up");
    trigger.setAttribute("aria-expanded", "false");
    if (openSelect === controller) openSelect = null;
  }

  function commit(index) {
    const option = options[index];
    if (!option) return;
    close();
    if (option.value === value) return;
    controller.setValue(option.value);
    onChange(option.value);
  }

  function move(delta) {
    highlighted = (highlighted + delta + options.length) % options.length;
    renderOptions();
    menu.querySelector(".is-highlighted")?.scrollIntoView({ block: "nearest" });
  }

  trigger.addEventListener("click", () => (menu.hidden ? open() : close()));

  trigger.addEventListener("keydown", (event) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === " ") {
      event.preventDefault();
      if (menu.hidden) open();
      else move(event.key === "ArrowUp" ? -1 : 1);
    } else if (event.key === "Enter" && !menu.hidden) {
      event.preventDefault();
      commit(highlighted);
    }
  });

  menu.addEventListener("mousedown", (event) => {
    const option = event.target.closest(".select-option");
    if (!option) return;
    event.preventDefault();
    commit(options.findIndex((entry) => entry.value === option.dataset.value));
    trigger.focus();
  });

  return controller;
}

/** Ferme la liste déroulante ouverte, s'il y en a une. Retourne vrai si c'était le cas. */
function closeOpenSelect() {
  if (!openSelect) return false;
  openSelect.close();
  return true;
}

document.addEventListener("mousedown", (event) => {
  if (!openSelect) return;
  if (event.target.closest(".select-menu, .select-trigger")) return;
  closeOpenSelect();
});

/* ------------------------------------------------------------- recherche */

/** Filtre et classe les fiches selon la recherche courante. */
function filterCustomers(query) {
  const terms = fold(query).split(/\s+/).filter(Boolean);
  if (terms.length === 0) return state.customers.slice();

  const scored = [];
  for (const customer of state.customers) {
    const name = fold(customer.name);
    const tenant = fold(customer.tenant);
    const keywords = (customer.keywords || []).map(fold);

    let score = 0;
    let matchesAll = true;
    for (const term of terms) {
      if (name.startsWith(term)) score += 0;
      else if (name.includes(term)) score += 2;
      else if (tenant.startsWith(term)) score += 3;
      else if (tenant.includes(term)) score += 4;
      else if (keywords.some((keyword) => keyword.startsWith(term))) score += 5;
      else if (keywords.some((keyword) => keyword.includes(term))) score += 6;
      else {
        matchesAll = false;
        break;
      }
    }
    if (matchesAll) scored.push({ customer, score });
  }

  scored.sort((left, right) => left.score - right.score);
  return scored.map((entry) => entry.customer);
}

/** Insère le nom en surlignant la première occurrence du premier terme. */
function renderName(customer, query) {
  const target = el("div", { class: "card-name" });
  const term = fold(query).split(/\s+/).filter(Boolean)[0];
  const index = term ? fold(customer.name).indexOf(term) : -1;
  if (index < 0) {
    target.textContent = customer.name;
    return target;
  }
  target.appendChild(document.createTextNode(customer.name.slice(0, index)));
  target.appendChild(el("mark", { text: customer.name.slice(index, index + term.length) }));
  target.appendChild(document.createTextNode(customer.name.slice(index + term.length)));
  return target;
}

/* --------------------------------------------------------------- rendu liste */

/** Reconstruit la liste des fiches. */
function renderList() {
  const query = dom.search.value;
  state.filtered = filterCustomers(query);
  state.pendingDelete = null;

  if (state.activeIndex >= state.filtered.length) {
    state.activeIndex = Math.max(0, state.filtered.length - 1);
  }

  const fragment = document.createDocumentFragment();
  const visible = state.filtered.slice(0, RENDER_LIMIT);
  visible.forEach((customer, index) => {
    fragment.appendChild(buildCard(customer, index, query));
  });
  if (state.filtered.length > visible.length) {
    fragment.appendChild(
      el("p", {
        class: "empty-hint",
        text: `+ ${state.filtered.length - visible.length} autres — affine ta recherche.`,
      })
    );
  }

  dom.list.replaceChildren(fragment);

  // La liste vide est retirée du flux pour que le message occupe toute la
  // hauteur restante et s'y centre réellement.
  const isEmpty = state.filtered.length === 0;
  dom.list.hidden = isEmpty;
  dom.empty.hidden = !isEmpty;
  dom.clear.hidden = query.length === 0;

  const total = state.customers.length;
  dom.subtitle.textContent =
    query.length > 0
      ? `${state.filtered.length} sur ${total} fiche${total > 1 ? "s" : ""}`
      : `${total} fiche${total > 1 ? "s" : ""}`;

  scrollActiveIntoView();
}

/** Construit la carte d'une fiche client. */
function buildCard(customer, index, query) {
  const card = el("article", {
    class: "card" + (index === state.activeIndex ? " is-active" : ""),
    attrs: {
      "data-id": customer.id,
      role: "option",
      "aria-selected": String(index === state.activeIndex),
    },
  });

  const top = [renderName(customer, query)];
  if (customer.tenant) {
    top.push(
      el("span", { class: "card-tenant", text: customer.tenant, attrs: { title: customer.tenant } })
    );
  }
  card.appendChild(el("div", { class: "card-top" }, top));

  if (customer.keywords && customer.keywords.length > 0) {
    card.appendChild(
      el(
        "div",
        { class: "chips" },
        customer.keywords.map((keyword) => el("span", { class: "chip", text: keyword }))
      )
    );
  }

  const workspaces = customer.workspaces || [];
  const links = customer.links || [];

  // Une rangée pour ce qui part au navigateur, une autre pour ce qui s'ouvre en
  // local. Chacune défile pour elle-même.
  const webRow = el("div", { class: "card-actions" });
  for (const descriptor of LINK_KINDS) {
    const url = descriptor.urlField ? customer[descriptor.urlField] : null;
    if (!url) continue;
    webRow.appendChild(
      linkButton(descriptor.label, descriptor.icon(), url, {
        "data-action": "open",
        "data-kind": descriptor.kind,
      })
    );
  }
  for (const link of links) {
    webRow.appendChild(
      linkButton(link.name, iconLink(), link.url, {
        "data-action": "link",
        "data-name": link.name,
      })
    );
  }

  const localRow = el("div", { class: "card-actions" });
  // Seules les cibles renseignées ont un bouton : une fiche n'affiche que ce
  // qu'elle sait ouvrir.
  if (customer.folderPath) {
    localRow.appendChild(
      linkButton("Dossier", iconFolder(), customer.folderPath, {
        "data-action": "open",
        "data-kind": "folder",
      })
    );
  }
  for (const workspace of workspaces) {
    localRow.appendChild(
      linkButton(workspace.name, iconCode(), workspace.path, {
        "data-action": "workspace",
        "data-name": workspace.name,
      })
    );
  }

  const tools = el("div", { class: "card-tools" }, [
    el("button", {
      class: "tool-btn",
      html: iconEdit(),
      attrs: { type: "button", "data-action": "edit", title: "Modifier", "aria-label": "Modifier" },
    }),
    el("button", {
      class: "tool-btn",
      html: iconTrash(),
      attrs: {
        type: "button",
        "data-action": "delete",
        title: "Supprimer",
        "aria-label": "Supprimer",
      },
    }),
  ]);

  const rows = [webRow, localRow].filter((row) => row.childElementCount > 0);
  // Les outils restent ancrés à droite, hors des bandes défilantes.
  card.appendChild(el("div", { class: "card-foot" }, [el("div", { class: "card-rows" }, rows), tools]));

  return card;
}

/** Construit un bouton d'ouverture ; la cible est rappelée en infobulle. */
function linkButton(label, icon, target, attrs) {
  const button = el("button", {
    class: "link-btn",
    attrs: {
      type: "button",
      title: target,
      ...attrs,
    },
  });
  button.appendChild(el("span", { class: "ico", html: icon }));
  button.appendChild(document.createTextNode(label));
  return button;
}

/** Garde la fiche sélectionnée visible pendant la navigation au clavier. */
function scrollActiveIntoView() {
  const active = dom.list.querySelector(".card.is-active");
  if (active) active.scrollIntoView({ block: "nearest" });
}

/** Déplace la sélection sans reconstruire toute la liste. */
function moveSelection(delta) {
  if (state.filtered.length === 0) return;
  const limit = Math.min(state.filtered.length, RENDER_LIMIT);
  const next = (state.activeIndex + delta + limit) % limit;
  const cards = dom.list.querySelectorAll(".card");
  cards[state.activeIndex]?.classList.remove("is-active");
  cards[state.activeIndex]?.setAttribute("aria-selected", "false");
  state.activeIndex = next;
  cards[next]?.classList.add("is-active");
  cards[next]?.setAttribute("aria-selected", "true");
  scrollActiveIntoView();
}

/* ------------------------------------------------------------------- vues */

/** Bascule vers une vue et y place le focus. */
function showView(name) {
  closeOpenSelect();
  state.view = name;
  for (const [key, node] of Object.entries(dom.views)) {
    node.hidden = key !== name;
  }
  if (name === "list") {
    dom.search.focus();
    dom.search.select();
  } else {
    dom.views[name].querySelector("input, .select-trigger")?.focus();
  }
}

/** Prépare le formulaire pour une création ou une modification. */
function openForm(customer) {
  state.editingId = customer ? customer.id : null;
  const fields = dom.form.elements;
  fields.name.value = customer ? customer.name : "";
  fields.tenant.value = customer ? customer.tenant : "";
  fields.keywords.value = customer ? (customer.keywords || []).join(", ") : "";
  fields.folderPath.value = customer ? customer.folderPath || "" : "";
  fields.devopsId.value = customer ? customer.devopsId || "" : "";
  fields.githubId.value = customer ? customer.githubId || "" : "";
  dom.workspaces.replaceChildren();
  for (const workspace of (customer && customer.workspaces) || []) addWorkspaceRow(workspace);
  dom.links.replaceChildren();
  for (const link of (customer && customer.links) || []) addLinkRow(link);
  renderFormPreview();
  showView("form");
  fields.name.focus();
  fields.name.select();
}

/** Ajoute une ligne « nom + dossier » au formulaire. */
function addWorkspaceRow(workspace) {
  const name = el("input", {
    class: "ws-name",
    attrs: { type: "text", maxlength: 512, placeholder: "Extension" },
  });
  const path = el("input", {
    class: "ws-path",
    attrs: {
      type: "text",
      maxlength: 512,
      placeholder: "dossier, ou fichier .code-workspace",
    },
  });
  name.value = workspace ? workspace.name : "";
  path.value = workspace ? workspace.path : "";

  const row = el("div", { class: "workspace-row" }, [
    name,
    path,
    el("button", {
      class: "tool-btn",
      html: iconFolder(),
      attrs: {
        type: "button",
        "data-ws": "folder",
        title: "Choisir un dossier",
        "aria-label": "Choisir un dossier",
      },
    }),
    el("button", {
      class: "tool-btn",
      html: iconCode(),
      attrs: {
        type: "button",
        "data-ws": "file",
        title: "Choisir un fichier .code-workspace",
        "aria-label": "Choisir un fichier .code-workspace",
      },
    }),
    el("button", {
      class: "tool-btn",
      html: iconTrash(),
      attrs: { type: "button", "data-ws": "remove", title: "Retirer", "aria-label": "Retirer" },
    }),
  ]);
  dom.workspaces.appendChild(row);
  return row;
}

/** Ajoute une ligne « nom + URL » au formulaire. */
function addLinkRow(link) {
  const name = el("input", {
    class: "ws-name",
    attrs: { type: "text", maxlength: 512, placeholder: "Extranet" },
  });
  const url = el("input", {
    class: "ws-path",
    attrs: { type: "text", maxlength: 512, placeholder: "https://…" },
  });
  name.value = link ? link.name : "";
  url.value = link ? link.url : "";

  const row = el("div", { class: "workspace-row" }, [
    name,
    url,
    el("button", {
      class: "tool-btn",
      html: iconTrash(),
      attrs: { type: "button", "data-ws": "remove", title: "Retirer", "aria-label": "Retirer" },
    }),
  ]);
  dom.links.appendChild(row);
  return row;
}

/** Relit les lignes de liens, en ignorant celles restées vides. */
function collectLinks() {
  return Array.from(dom.links.querySelectorAll(".workspace-row"))
    .map((row) => ({
      name: row.querySelector(".ws-name").value.trim(),
      url: row.querySelector(".ws-path").value.trim(),
    }))
    .filter((link) => link.name || link.url);
}

/** Relit les lignes du formulaire, en ignorant celles restées vides. */
function collectWorkspaces() {
  return Array.from(dom.workspaces.querySelectorAll(".workspace-row"))
    .map((row) => ({
      name: row.querySelector(".ws-name").value.trim(),
      path: row.querySelector(".ws-path").value.trim(),
    }))
    .filter((workspace) => workspace.name || workspace.path);
}

/** Affiche en direct les URL qui seront générées par la fiche en cours d'édition. */
function renderFormPreview() {
  if (!state.settings) return;
  const fields = dom.form.elements;
  const draft = {
    tenant: fields.tenant.value,
    devopsId: fields.devopsId.value,
    githubId: fields.githubId.value,
  };
  const lines = [
    ["DevOps", applyTemplate(state.settings.devopsUrlTemplate, draft, "devopsId")],
    ["GitHub", applyTemplate(state.settings.githubUrlTemplate, draft, "githubId")],
    ["Admin Center", applyTemplate(state.settings.adminCenterUrlTemplate, draft, "tenant")],
  ];
  dom.formPreview.replaceChildren(
    ...lines.map(([label, url]) =>
      el("div", { class: "preview-line" + (url ? "" : " muted") }, [
        el("b", { text: label }),
        el("span", { text: url || "identifiant manquant" }),
      ])
    )
  );
}

/** Remplit la vue réglages avec l'état courant. */
function renderSettings(autostartEnabled) {
  const fields = dom.settingsForm.elements;
  fields.devopsUrlTemplate.value = state.settings.devopsUrlTemplate;
  fields.githubUrlTemplate.value = state.settings.githubUrlTemplate;
  fields.adminCenterUrlTemplate.value = state.settings.adminCenterUrlTemplate;
  dom.hotkeyInput.value = formatAccelerator(state.settings.hotkey);
  for (const render of tokenRenderers) render();
  dom.dataPath.textContent = state.dataPath;
  if (autostartEnabled !== undefined) dom.autostart.checked = autostartEnabled;

  renderBrowserProfiles(state.settings.browserProfiles);
  renderThemeList();
  renderSettingsSummaries();
}

/* ------------------------------------------------------ jetons des gabarits */

/**
 * Colore les jetons d'un champ de gabarit : un calque, aligné au pixel sur le
 * champ, redessine son texte par-dessus lui. Le texte du champ est rendu
 * transparent, mais son curseur et sa sélection restent visibles au travers.
 *
 * Seule la couleur change sur un jeton, jamais la graisse : la moindre différence
 * de chasse décalerait le calque du curseur.
 */
function enhanceTokenInput(input) {
  const content = el("span", { class: "token-mirror-content" });
  const mirror = el("div", { class: "token-mirror", attrs: { "aria-hidden": "true" } }, [content]);
  const wrapper = el("div", { class: "token-input" });
  input.replaceWith(wrapper);
  wrapper.append(input, mirror);

  const sync = () => {
    content.style.transform = `translateX(${-input.scrollLeft}px)`;
  };
  const render = () => {
    content.replaceChildren(...tokenize(input.value));
    sync();
  };

  input.addEventListener("input", render);
  // Le défilement horizontal du champ suit le curseur : le calque doit le suivre.
  for (const type of ["scroll", "keydown", "keyup", "mousedown", "mouseup", "select", "focus", "blur"]) {
    input.addEventListener(type, () => requestAnimationFrame(sync));
  }
  return render;
}

/** Découpe un gabarit en texte brut et jetons, reconnus ou non. */
function tokenize(value) {
  const nodes = [];
  let last = 0;
  for (const match of value.matchAll(/\{(?<name>[A-Za-z]+)\}/g)) {
    if (match.index > last) {
      nodes.push(document.createTextNode(value.slice(last, match.index)));
    }

    const known = TEMPLATE_TOKENS.includes(match.groups.name);
    nodes.push(el("span", { class: known ? "token" : "token is-unknown", text: match[0] }));
    last = match.index + match[0].length;
  }
  if (last < value.length) {
    nodes.push(document.createTextNode(value.slice(last)));
  }

  return nodes;
}

/** Redessine les calques après un remplissage par programme, qui n'émet pas `input`. */
const tokenRenderers = Array.from(dom.settingsForm.querySelectorAll("input[data-tokens]"), enhanceTokenInput);

/* ------------------------------------------------------------ navigateurs */

/** Logo d'un navigateur ; une pastille à l'initiale le remplace si le fichier manque. */
function browserLogo(kind) {
  const descriptor = BROWSERS.find((browser) => browser.kind === kind);
  const file = descriptor ? descriptor.logo : `${kind}.png`;
  const image = el("img", {
    class: "browser-logo",
    attrs: { src: `icons/browsers/${file}`, alt: "", draggable: "false" },
  });
  image.addEventListener("error", () => {
    const label = descriptor ? descriptor.label : kind;
    image.replaceWith(el("span", { class: "avatar-initial", text: label.charAt(0) }));
  });
  return image;
}

/** Vignette d'un profil : sa photo, sinon son initiale sur sa couleur. */
function profileAvatar(profile) {
  if (profile.avatar) {
    return el("img", { class: "profile-avatar", attrs: { src: profile.avatar, alt: "", draggable: "false" } });
  }

  const initial = el("span", { class: "avatar-initial", text: (profile.name || "?").charAt(0).toUpperCase() });
  if (profile.color) {
    initial.style.background = profile.color;
    initial.style.color = "#fff";
  }

  return initial;
}

/** État détecté d'un navigateur sur ce poste. */
function detectedBrowser(kind) {
  return state.browsers.find((browser) => browser.kind === kind) || { kind, installed: false, profiles: [] };
}

/** Options de la liste « navigateur ». */
function browserOptions() {
  const options = BROWSERS.map((descriptor) => {
    const detected = detectedBrowser(descriptor.kind);
    return {
      value: descriptor.kind,
      main: descriptor.label,
      sub: detected.installed ? "" : "non détecté sur ce poste",
      icon: () => browserLogo(descriptor.kind),
      rank: browserRank(descriptor.kind, detected.installed),
    };
  });
  // Le tri est stable : chaque groupe garde l'ordre de `BROWSERS`.
  return options.sort((left, right) => left.rank - right.rank);
}

/** Groupe d'affichage : détectés, puis non détectés, puis navigateur par défaut. */
function browserRank(kind, installed) {
  if (kind === "system") {
    return 2;
  }

  return installed ? 0 : 1;
}

/** Options de la liste « profil » pour un navigateur donné. */
function profileOptions(kind) {
  const descriptor = BROWSERS.find((browser) => browser.kind === kind);
  if (!descriptor || !descriptor.profiles) {
    return [{ value: "", main: "Profil unique", sub: "ce navigateur n'expose pas ses profils" }];
  }

  const detected = detectedBrowser(kind).profiles.map((profile) => {
    const details = profile.account ? `${profile.id} — ${profile.account}` : profile.id;
    return {
      value: profile.id,
      main: profile.name,
      sub: profile.isDefault ? `${details} · par défaut` : details,
      icon: () => profileAvatar(profile),
    };
  });
  return [...detected, { value: "", main: "Profil par défaut du navigateur", sub: "aucun profil imposé" }];
}

/** Profil à présélectionner quand on choisit un navigateur : le défaut, sinon le seul. */
function defaultProfileOf(kind) {
  const profiles = detectedBrowser(kind).profiles;
  const preferred = profiles.find((profile) => profile.isDefault) || profiles[0];
  return preferred ? preferred.id : "";
}

/** Compteur servant à nommer les listes créées à la volée. */
let selectSequence = 0;

/** Contrôleurs des listes de chaque carte de profil. */
const profileCards = new WeakMap();

/** Reconstruit l'éditeur des profils de navigation. */
function renderBrowserProfiles(profiles) {
  dom.browserProfiles.replaceChildren();
  for (const profile of profiles) {
    addBrowserProfileCard(profile);
  }

  refreshProfileCards();
}

/** Ajoute une carte d'édition de profil de navigation. */
function addBrowserProfileCard(profile) {
  selectSequence += 1;
  const browserLabelId = `bp-browser-label-${selectSequence}`;
  const profileLabelId = `bp-profile-label-${selectSequence}`;

  const name = el("input", {
    class: "bp-name",
    attrs: { type: "text", maxlength: 512, placeholder: "DevOps et GitHub", "aria-label": "Nom du profil" },
  });
  const hosts = el("input", {
    class: "bp-hosts",
    attrs: { type: "text", maxlength: 4096, spellcheck: "false", placeholder: "dev.azure.com, github.com" },
  });
  name.value = profile.name;
  hosts.value = profile.hosts.join(", ");

  const browserSlot = el("div", { class: "select", attrs: { "data-labelled-by": browserLabelId } });
  const profileSlot = el("div", { class: "select", attrs: { "data-labelled-by": profileLabelId } });

  const card = el("div", { class: "profile-card" }, [
    el("div", { class: "profile-card-head" }, [
      el("span", { class: "profile-rank" }),
      name,
      toolButton(iconUp(), "up", "Monter"),
      toolButton(iconDown(), "down", "Descendre"),
      toolButton(iconTrash(), "remove", "Supprimer le profil"),
    ]),
    // Une ligne chacun : côte à côte, les noms de profil et de compte débordaient.
    el("div", { class: "field" }, [
      el("span", { class: "label", text: "Navigateur", attrs: { id: browserLabelId } }),
      browserSlot,
    ]),
    el("div", { class: "field" }, [
      el("span", { class: "label", text: "Profil", attrs: { id: profileLabelId } }),
      profileSlot,
    ]),
    el("label", { class: "field" }, [
      el("span", { class: "label", html: "Hôtes <em>séparés par des virgules</em>" }),
      hosts,
    ]),
  ]);
  dom.browserProfiles.appendChild(card);

  const profileSelect = createSelect(profileSlot, () => {});
  const browserSelect = createSelect(browserSlot, (kind) => applyBrowser(kind, defaultProfileOf(kind)));

  /** Aligne la liste des profils sur le navigateur choisi. */
  function applyBrowser(kind, selectedProfile) {
    const descriptor = BROWSERS.find((browser) => browser.kind === kind);
    profileSelect.setOptions(profileOptions(kind), selectedProfile);
    profileSelect.setDisabled(!descriptor || !descriptor.profiles);
  }

  browserSelect.setOptions(browserOptions(), profile.browser);
  applyBrowser(profile.browser, profile.profile);
  profileCards.set(card, { name, hosts, browserSelect, profileSelect });
  return card;
}

/** Bouton d'outil d'une carte de profil. */
function toolButton(icon, action, label) {
  return el("button", {
    class: "tool-btn",
    html: icon,
    attrs: { type: "button", "data-bp": action, title: label, "aria-label": label },
  });
}

/** Renumérote les cartes et grise les déplacements impossibles. */
function refreshProfileCards() {
  const cards = Array.from(dom.browserProfiles.children);
  cards.forEach((card, index) => {
    card.querySelector(".profile-rank").textContent = String(index + 1);
    card.querySelector('[data-bp="up"]').disabled = index === 0;
    card.querySelector('[data-bp="down"]').disabled = index === cards.length - 1;
    // Il faut toujours un profil vers lequel envoyer les liens.
    card.querySelector('[data-bp="remove"]').disabled = cards.length === 1;
  });
}

/** Relit les profils de navigation saisis. */
function collectBrowserProfiles() {
  return Array.from(dom.browserProfiles.children).map((card) => {
    const controls = profileCards.get(card);
    return {
      name: controls.name.value.trim(),
      browser: controls.browserSelect.getValue(),
      profile: controls.profileSelect.getValue() || "",
      hosts: controls.hosts.value
        .split(",")
        .map((host) => host.trim())
        .filter(Boolean),
    };
  });
}

dom.browserProfiles.addEventListener("click", (event) => {
  const button = event.target.closest("[data-bp]");
  if (!button) return;
  const card = button.closest(".profile-card");
  closeOpenSelect();

  switch (button.dataset.bp) {
    case "up":
      card.previousElementSibling?.before(card);
      break;
    case "down":
      card.nextElementSibling?.after(card);
      break;
    case "remove":
      card.remove();
      break;
    default:
      break;
  }

  refreshProfileCards();
});

document.getElementById("btn-add-profile").addEventListener("click", () => {
  const installed = BROWSERS.find((browser) => detectedBrowser(browser.kind).installed) || BROWSERS[0];
  const card = addBrowserProfileCard({
    name: "",
    browser: installed.kind,
    profile: defaultProfileOf(installed.kind),
    hosts: [],
  });
  refreshProfileCards();
  card.querySelector(".bp-name").focus();
});

/* ---------------------------------------------------------- mise à jour */

/** Retient la mise à jour disponible et affiche, ou non, son bouton. */
function setUpdate(update) {
  state.update = update || null;
  dom.updateButton.hidden = !state.update;
  if (state.update) {
    dom.updateButton.title = `Mise à jour ${state.update.version} disponible`;
  }
}

/** Formate une taille en mégaoctets. */
function formatSize(bytes) {
  const megabytes = bytes / (1024 * 1024);
  return `${megabytes.toLocaleString("fr-FR", { maximumFractionDigits: 1 })} Mo`;
}

/** Ouvre la fenêtre de mise à jour. */
function openUpdateDialog() {
  const update = state.update;
  if (!update) return;
  closeOpenSelect();

  const published = update.publishedAt ? new Date(update.publishedAt) : null;
  const flavor = update.flavor === "installer" ? "installateur" : "portable";
  dom.updateVersion.textContent = `${state.version} → ${update.version}`;
  dom.updateDate.textContent = published
    ? published.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" })
    : "—";
  dom.updateSize.textContent = formatSize(update.size);
  dom.updateFlavor.textContent = flavor;
  dom.updateNotes.textContent = update.notes.trim();
  dom.updateNotes.hidden = !update.notes.trim();
  if (!state.installing) {
    dom.updateProgress.hidden = true;
    dom.updateStatus.hidden = true;
  }

  dom.updateDialog.hidden = false;
  playEntrance(dom.updateDialog, "is-entering", 200);
  dom.updateInstall.focus();
}

/** Ferme la fenêtre de mise à jour, sauf pendant l'installation. */
function closeUpdateDialog() {
  if (state.installing) return;
  dom.updateDialog.hidden = true;
}

/** Affiche l'avancement de l'installation. */
function showUpdateProgress(downloaded, total) {
  const ratio = total > 0 ? Math.min(1, downloaded / total) : 0;
  dom.updateProgress.hidden = false;
  dom.updateProgress.firstElementChild.style.width = `${(ratio * 100).toFixed(1)}%`;
  dom.updateStatus.hidden = false;
  dom.updateStatus.textContent =
    ratio < 1
      ? `Téléchargement… ${formatSize(downloaded)} sur ${formatSize(total)}`
      : "Installation, l'application va redémarrer…";
}

/** Télécharge et installe la mise à jour ; l'application redémarre d'elle-même. */
async function installUpdate() {
  if (state.installing || !state.update) return;
  state.installing = true;
  dom.updateInstall.disabled = true;
  dom.updateLater.disabled = true;
  showUpdateProgress(0, state.update.size);

  try {
    await call("install_update");
  }
  catch {
    // L'erreur est déjà affichée : on rend la main pour permettre un nouvel essai.
    state.installing = false;
    dom.updateInstall.disabled = false;
    dom.updateLater.disabled = false;
    dom.updateProgress.hidden = true;
    dom.updateStatus.hidden = true;
  }
}

dom.updateButton.addEventListener("click", openUpdateDialog);
dom.updateLater.addEventListener("click", closeUpdateDialog);
dom.updateInstall.addEventListener("click", installUpdate);
dom.updateDialog.addEventListener("mousedown", (event) => {
  if (event.target === dom.updateDialog) closeUpdateDialog();
});

listen("update://available", (event) => setUpdate(event.payload));
listen("update://progress", (event) => showUpdateProgress(event.payload.downloaded, event.payload.total));

/* ------------------------------------------------------- sections des réglages */

/** Ouvre les réglages sur leur menu. */
function openSettings() {
  renderSettings();
  showView("settings");
  showSettingsMenu();
}

/** Affiche le menu des sections. */
function showSettingsMenu() {
  closeOpenSelect();
  state.settingsSection = null;
  dom.settingsTitle.textContent = "Réglages";
  dom.settingsForm.hidden = true;
  dom.settingsMenu.hidden = false;
  renderSettingsSummaries();
  dom.settingsMenu.querySelector(".menu-item")?.focus();
}

/** Affiche une section de réglages, seule. */
function showSettingsSection(name) {
  closeOpenSelect();
  state.settingsSection = name;
  let savable = false;
  for (const section of dom.settingsSections) {
    const isCurrent = section.dataset.section === name;
    section.hidden = !isCurrent;
    if (isCurrent) {
      dom.settingsTitle.textContent = section.dataset.title;
      savable = section.hasAttribute("data-savable");
    }
  }

  // Apparence, système et données s'appliquent immédiatement : rien à enregistrer.
  dom.settingsActions.hidden = !savable;
  dom.settingsMenu.hidden = true;
  dom.settingsForm.hidden = false;
  dom.settingsForm.scrollTop = 0;
  dom.settingsForm.querySelector(`[data-section="${name}"] input, [data-section="${name}"] button`)?.focus();
}

/** Revient d'un cran : de la section au menu, du menu à la liste. */
function settingsBack() {
  if (state.settingsSection) {
    // Abandonner une section non enregistrée remet ses champs à l'état stocké.
    renderSettings();
    showSettingsMenu();
  }
  else {
    showView("list");
  }
}

/** Résumés affichés sous les entrées du menu. */
function renderSettingsSummaries() {
  if (!state.settings) return;
  const count = state.settings.browserProfiles.length;
  const theme = THEMES.find((entry) => entry.value === state.settings.theme);
  document.getElementById("summary-browsers").textContent = `${count} profil${count > 1 ? "s" : ""}`;
  document.getElementById("summary-appearance").textContent = theme ? `Thème ${theme.main.toLowerCase()}` : "Thème";
  document.getElementById("summary-system").textContent = `Raccourci ${formatAccelerator(state.settings.hotkey)}`;
}

dom.settingsMenu.addEventListener("click", (event) => {
  const item = event.target.closest(".menu-item");
  if (item) showSettingsSection(item.dataset.section);
});

document.getElementById("settings-back").addEventListener("click", settingsBack);
document.getElementById("settings-cancel").addEventListener("click", settingsBack);

/**
 * Enregistre tout de suite une préférence (thème, raccourci), sans toucher aux
 * champs des autres sections : une saisie en cours ailleurs n'est pas perdue.
 */
async function persistSettings(patch) {
  const data = await call("save_settings", { settings: { ...state.settings, ...patch } });
  state.settings = data.settings;
  state.customers = data.customers;
  applyTheme(state.settings.theme);
  renderThemeList();
  dom.hotkeyInput.value = formatAccelerator(state.settings.hotkey);
  renderSettingsSummaries();
  renderList();
}

/* ------------------------------------------------------------------ thème */

/** Liste à plat des apparences, l'active cochée. */
function renderThemeList() {
  const current = state.settings ? state.settings.theme : "system";
  dom.themeList.replaceChildren(
    ...THEMES.map((theme) =>
      el(
        "button",
        {
          class: "choice",
          attrs: { type: "button", role: "radio", "aria-checked": String(theme.value === current), "data-theme": theme.value },
        },
        [
          el("span", { class: "select-texts" }, [
            el("span", { class: "select-main", text: theme.main }),
            ...(theme.sub ? [el("span", { class: "choice-sub", text: theme.sub })] : []),
          ]),
          el("span", { class: "choice-check", html: iconCheck() }),
        ]
      )
    )
  );
}

dom.themeList.addEventListener("click", async (event) => {
  const choice = event.target.closest(".choice");
  if (!choice || !state.settings || choice.dataset.theme === state.settings.theme) return;
  applyTheme(choice.dataset.theme);
  await persistSettings({ theme: choice.dataset.theme });
});

/* ------------------------------------------------------- raccourci global */

/** Touches de modification, reconnues à leur code physique. */
const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

/** Libellés affichés des modificateurs ; l'accélérateur garde les noms anglais. */
const MODIFIER_LABELS = { Ctrl: "Ctrl", Alt: "Alt", Shift: "Maj", Super: "Win" };

/** Capture en cours : le dernier accélérateur complet saisi. */
let hotkeyCapture = null;

/** Modificateurs enfoncés, dans l'ordre canonique de l'accélérateur. */
function modifiersOf(event) {
  const modifiers = [];
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Super");
  return modifiers;
}

/**
 * Nom de touche compris par le parseur d'accélérateurs : le code physique
 * (`KeyD`, `Digit1`, `F5`, `Space`…), raccourci en `D` ou `1` pour la lisibilité.
 * Le code physique, contrairement au caractère, ne dépend pas de la disposition.
 */
function keyName(code) {
  const match = /^(?:Key|Digit)(?<key>[A-Z0-9])$/.exec(code);
  return match ? match.groups.key : code;
}

/** Accélérateur lisible : `Ctrl + Maj + D`. */
function formatAccelerator(accelerator) {
  return accelerator
    .split("+")
    .map((part) => MODIFIER_LABELS[part] || part)
    .join(" + ");
}

/** Affiche une combinaison en touches. */
function renderHotkeyPreview(parts) {
  if (parts.length === 0) {
    dom.hotkeyPreview.replaceChildren(el("span", { class: "hotkey-placeholder", text: "En attente…" }));
    return;
  }

  dom.hotkeyPreview.replaceChildren(
    ...parts.map((part) => el("kbd", { text: MODIFIER_LABELS[part] || part }))
  );
}

/** Ouvre la capture : le raccourci global est suspendu le temps de la saisie. */
async function openHotkeyCapture() {
  closeOpenSelect();
  hotkeyCapture = { accelerator: null };
  dom.hotkeyMessage.textContent = "";
  renderHotkeyPreview([]);
  dom.hotkeyDialog.hidden = false;
  playEntrance(dom.hotkeyDialog, "is-entering", 200);
  window.addEventListener("keydown", onCaptureKeydown, true);
  window.addEventListener("keyup", onCaptureKeyup, true);
  // Sans cela, taper le raccourci actuel fermerait l'overlay au lieu d'être capturé.
  await call("pause_shortcut");
}

/** Ferme la capture et rend la main au raccourci enregistré. */
async function closeHotkeyCapture() {
  if (!hotkeyCapture) return;
  hotkeyCapture = null;
  window.removeEventListener("keydown", onCaptureKeydown, true);
  window.removeEventListener("keyup", onCaptureKeyup, true);
  dom.hotkeyDialog.hidden = true;
  dom.hotkeyButton.focus();
  await call("resume_shortcut");
}

/** Valide la combinaison saisie et l'applique aussitôt. */
async function confirmHotkeyCapture() {
  if (!hotkeyCapture) return;
  const accelerator = hotkeyCapture.accelerator;
  if (!accelerator) {
    dom.hotkeyMessage.textContent = "Aucune combinaison saisie.";
    return;
  }

  try {
    if (accelerator !== state.settings.hotkey) {
      await persistSettings({ hotkey: accelerator });
      toast(`Raccourci ${formatAccelerator(accelerator)} enregistré.`);
    }
  }
  finally {
    // Refusé par le système ou accepté, le raccourci en vigueur doit être réarmé.
    await closeHotkeyCapture();
  }
}

/** Toute frappe est capturée : rien ne doit atteindre les raccourcis de l'overlay. */
function onCaptureKeydown(event) {
  event.preventDefault();
  event.stopPropagation();
  if (event.repeat) return;

  const modifiers = modifiersOf(event);
  if (modifiers.length === 0 && event.code === "Escape") {
    closeHotkeyCapture();
    return;
  }

  if (modifiers.length === 0 && (event.code === "Enter" || event.code === "NumpadEnter")) {
    confirmHotkeyCapture();
    return;
  }

  if (MODIFIER_CODES.has(event.code)) {
    if (!hotkeyCapture.accelerator) renderHotkeyPreview(modifiers);
    return;
  }

  if (modifiers.length === 0) {
    dom.hotkeyMessage.textContent = "Ajoute au moins un modificateur : Ctrl, Alt, Maj ou Win.";
    renderHotkeyPreview([keyName(event.code)]);
    return;
  }

  const parts = [...modifiers, keyName(event.code)];
  hotkeyCapture.accelerator = parts.join("+");
  dom.hotkeyMessage.textContent = "Entrée pour valider, ou tape une autre combinaison.";
  renderHotkeyPreview(parts);
}

/** Tant qu'aucune combinaison n'est complète, l'aperçu suit les modificateurs tenus. */
function onCaptureKeyup(event) {
  event.preventDefault();
  event.stopPropagation();
  if (hotkeyCapture && !hotkeyCapture.accelerator) renderHotkeyPreview(modifiersOf(event));
}

dom.hotkeyButton.addEventListener("click", openHotkeyCapture);
document.getElementById("btn-hotkey-cancel").addEventListener("click", closeHotkeyCapture);
document.getElementById("btn-hotkey-confirm").addEventListener("click", confirmHotkeyCapture);
dom.hotkeyDialog.addEventListener("mousedown", (event) => {
  if (event.target === dom.hotkeyDialog) closeHotkeyCapture();
});

// L'overlay se referme (clic ailleurs) : la capture est abandonnée, faute de quoi
// le raccourci resterait suspendu et l'overlay injoignable au clavier.
listen("overlay://hidden", () => closeHotkeyCapture());

/* ----------------------------------------------------------------- actions */

/** Applique un instantané renvoyé par le backend. */
function applyBootstrap(data) {
  state.customers = data.customers;
  state.settings = data.settings;
  state.dataPath = data.dataPath;
  state.browsers = data.browsers;
  state.version = data.version;
  setUpdate(data.update);
  applyTheme(data.settings.theme);
  renderSettings(data.autostartEnabled);
  renderList();
}

/** Remplace la liste après une écriture, en conservant recherche et sélection. */
function applyCustomers(customers) {
  state.customers = customers;
  renderList();
}

/** Ouvre une cible pour la fiche sélectionnée. */
async function openLink(id, kind) {
  await call("open_link", { id, kind });
}

/** Ouvre la cible correspondant aux modificateurs du clavier. */
function openFromKeyboard(event) {
  const customer = state.filtered[state.activeIndex];
  if (!customer) return;
  if (event.ctrlKey) return openLink(customer.id, "devops");
  if (event.shiftKey) return openLink(customer.id, "github");
  if (event.altKey) return openLink(customer.id, "adminCenter");
  const workspaces = customer.workspaces || [];
  if (!customer.folderPath && workspaces.length > 0) {
    return call("open_workspace", { id: customer.id, name: workspaces[0].name });
  }
  return openLink(customer.id, "folder");
}

/** Ferme l'overlay. */
async function hideOverlay() {
  closeOpenSelect();
  await invoke("hide_overlay").catch(() => {});
}

/* -------------------------------------------------------------- évènements */

dom.search.addEventListener("input", () => {
  state.activeIndex = 0;
  renderList();
});

dom.clear.addEventListener("click", () => {
  dom.search.value = "";
  state.activeIndex = 0;
  renderList();
  dom.search.focus();
});

dom.list.addEventListener("click", async (event) => {
  const card = event.target.closest(".card");
  if (!card) return;
  const id = card.dataset.id;
  const index = Array.from(dom.list.querySelectorAll(".card")).indexOf(card);
  if (index >= 0 && index !== state.activeIndex) {
    moveSelection(index - state.activeIndex);
  }

  const button = event.target.closest("[data-action]");
  if (!button) return;

  switch (button.dataset.action) {
    case "open":
      await openLink(id, button.dataset.kind);
      break;
    case "workspace":
      await call("open_workspace", { id, name: button.dataset.name });
      break;
    case "link":
      await call("open_custom_link", { id, name: button.dataset.name });
      break;
    case "edit": {
      const customer = state.customers.find((entry) => entry.id === id);
      if (customer) openForm(customer);
      break;
    }
    case "delete":
      if (state.pendingDelete === id) {
        state.pendingDelete = null;
        applyCustomers(await call("delete_customer", { id }));
        toast("Fiche supprimée.");
      } else {
        state.pendingDelete = id;
        button.classList.add("confirm");
        button.textContent = "Confirmer";
        window.setTimeout(() => {
          if (state.pendingDelete === id) {
            state.pendingDelete = null;
            renderList();
          }
        }, 4000);
      }
      break;
    default:
      break;
  }
});

dom.backdrop.addEventListener("mousedown", hideOverlay);

document.getElementById("btn-close").addEventListener("click", hideOverlay);
document.getElementById("btn-new").addEventListener("click", () => openForm(null));
document.getElementById("btn-settings").addEventListener("click", openSettings);

for (const button of document.querySelectorAll("[data-back]")) {
  button.addEventListener("click", () => showView("list"));
}

for (const field of ["tenant", "devopsId", "githubId"]) {
  dom.form.elements[field].addEventListener("input", renderFormPreview);
}

document.getElementById("btn-add-link").addEventListener("click", () => {
  const row = addLinkRow(null);
  row.querySelector(".ws-name").focus();
});

dom.links.addEventListener("click", (event) => {
  const button = event.target.closest('[data-ws="remove"]');
  if (button) button.closest(".workspace-row").remove();
});

document.getElementById("btn-add-workspace").addEventListener("click", () => {
  const row = addWorkspaceRow(null);
  row.querySelector(".ws-name").focus();
});

dom.workspaces.addEventListener("click", async (event) => {
  const button = event.target.closest("[data-ws]");
  if (!button) return;
  const row = button.closest(".workspace-row");
  if (button.dataset.ws === "remove") {
    row.remove();
    return;
  }
  const command = button.dataset.ws === "file" ? "pick_workspace_file" : "pick_folder";
  const selected = await call(command);
  if (selected) row.querySelector(".ws-path").value = selected;
});

document.getElementById("btn-browse").addEventListener("click", async () => {
  const selected = await call("pick_folder");
  if (selected) dom.form.elements.folderPath.value = selected;
});

dom.form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const fields = dom.form.elements;
  const customer = {
    tenant: fields.tenant.value,
    name: fields.name.value,
    keywords: fields.keywords.value
      .split(",")
      .map((keyword) => keyword.trim())
      .filter(Boolean),
    folderPath: fields.folderPath.value,
    devopsId: fields.devopsId.value,
    githubId: fields.githubId.value,
    workspaces: collectWorkspaces(),
    links: collectLinks(),
  };

  const customers = state.editingId
    ? await call("update_customer", { id: state.editingId, customer })
    : await call("create_customer", { customer });

  applyCustomers(customers);
  toast(state.editingId ? "Fiche mise à jour." : "Fiche créée.");
  state.editingId = null;
  showView("list");
});

dom.settingsForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const fields = dom.settingsForm.elements;
  const settings = {
    devopsUrlTemplate: fields.devopsUrlTemplate.value,
    githubUrlTemplate: fields.githubUrlTemplate.value,
    adminCenterUrlTemplate: fields.adminCenterUrlTemplate.value,
    browserProfiles: collectBrowserProfiles(),
    hotkey: state.settings.hotkey,
    theme: state.settings.theme,
  };
  applyBootstrap(await call("save_settings", { settings }));
  toast("Réglages enregistrés.");
  showSettingsMenu();
});

dom.autostart.addEventListener("change", async () => {
  const enabled = await call("set_autostart", { enabled: dom.autostart.checked });
  dom.autostart.checked = enabled;
  toast(enabled ? "Lancement au démarrage activé." : "Lancement au démarrage désactivé.");
});

document.getElementById("btn-reveal").addEventListener("click", async () => {
  await call("reveal_data_folder");
});

document.getElementById("btn-export").addEventListener("click", async () => {
  const path = await call("export_data");
  if (path) toast("Exporté (chiffré) vers " + path);
});

document.getElementById("btn-export-plain").addEventListener("click", async () => {
  const path = await call("export_plain_data");
  if (path) toast("Exporté en clair vers " + path + " — à ne pas laisser traîner.");
});

for (const [id, mode] of [
  ["btn-import-merge", "merge"],
  ["btn-import-replace", "replace"],
]) {
  document.getElementById(id).addEventListener("click", async () => {
    const report = await call("import_data", { mode });
    if (!report) return;
    applyBootstrap(await call("bootstrap"));
    toast(
      `Import terminé : ${report.added} ajoutée(s), ${report.updated} mise(s) à jour, ` +
        `${report.skipped} inchangée(s). Sauvegarde : ${report.backupPath}`
    );
  });
}

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    // Échap ferme d'abord ce qui est ouvert par-dessus la vue.
    if (closeOpenSelect()) return;
    if (!dom.updateDialog.hidden) {
      closeUpdateDialog();
      return;
    }
    if (state.view === "list") hideOverlay();
    else if (state.view === "settings") settingsBack();
    else showView("list");
    return;
  }

  if (event.ctrlKey && event.key.toLowerCase() === "n") {
    event.preventDefault();
    openForm(null);
    return;
  }

  if (event.ctrlKey && event.key === ",") {
    event.preventDefault();
    openSettings();
    return;
  }

  if (state.view !== "list" || !dom.updateDialog.hidden) return;

  switch (event.key) {
    case "ArrowDown":
      event.preventDefault();
      moveSelection(1);
      break;
    case "ArrowUp":
      event.preventDefault();
      moveSelection(-1);
      break;
    case "Enter":
      event.preventDefault();
      openFromKeyboard(event);
      break;
    case "F2": {
      event.preventDefault();
      const customer = state.filtered[state.activeIndex];
      if (customer) openForm(customer);
      break;
    }
    default:
      // Toute autre touche alimente la recherche : l'overlay reste pilotable
      // sans jamais quitter le clavier.
      if (event.key.length === 1 && !event.ctrlKey && !event.altKey) dom.search.focus();
      break;
  }
});

/** Rejoue l'animation d'ouverture du panneau. */
function playOpenAnimation() {
  playEntrance(document.body, "is-opening", 240);
}

/** Horodatage de la dernière ouverture traitée. */
let lastShownAt = 0;

/* À chaque ouverture : recherche précédente intacte et sélectionnée — une frappe
 * la remplace, Entrée la réutilise.
 *
 * Un formulaire ou des réglages ouverts sont un travail en cours, et l'overlay se
 * referme dès qu'on va chercher quelque chose ailleurs (un dossier, un fichier à
 * importer). Le ramener de force à l'accueil jetterait la saisie : on reste donc
 * sur la vue en cours, et seule la liste se rafraîchit. */
function onOverlayShown() {
  // L'évènement du backend et le retour de focus arrivent tous les deux : sans
  // ce garde-fou l'animation d'ouverture serait rejouée deux fois de suite.
  const now = performance.now();
  const isNewOpening = now - lastShownAt > 400;
  lastShownAt = now;

  state.pendingDelete = null;
  closeOpenSelect();
  // Le panneau quitte son état d'attente et l'animation prend le relais dans la
  // même tâche : la première image affichée est donc celle du départ du
  // glissement, jamais l'état final.
  document.body.classList.remove("is-closed");
  if (isNewOpening) playOpenAnimation();
  if (state.view !== "list") return;
  renderList();
  dom.search.focus();
  dom.search.select();
}

listen("overlay://shown", onOverlayShown);

// Émis juste avant le masquage : le panneau reprend son état de départ pendant
// que la webview compose encore, pour que l'image conservée soit la bonne.
listen("overlay://hidden", () => {
  document.body.classList.remove("is-opening");
  document.body.classList.add("is-closed");
});

// Filet de sécurité : la webview est suspendue tant que l'overlay est masqué, et
// l'évènement peut arriver avant sa reprise. Le retour du focus, lui, est certain.
window.addEventListener("focus", onOverlayShown);

/* ------------------------------------------------------------------ icônes */

function iconDevops() {
  // Marque Azure DevOps redessinee sur notre grille : deux rubans imbriques.
  // Le premier part de la pointe haute et descend en bande vers le bas a gauche,
  // le second longe le bord droit puis le bas, encoche comprise. Tracee pleine,
  // comme l'originale : a 13 px un contour se refermerait sur lui-meme.
  return (
    '<svg viewBox="0 0 24 24">' +
    '<path fill="currentColor" stroke="none" d="M10.4 1.4 18.2 5.6 4.5 8.9v7.7l-3.1-1.5V8.6l2.2-2.7 6.8-1.9z"/>' +
    '<path fill="currentColor" stroke="none" d="M22.6 5.2v13.3l-5.3 4.2-8.2-3v3l-4.6-6 13.7 1.1V5.7z"/>' +
    '</svg>'
  );
}


function iconGithub() {
  return '<svg viewBox="0 0 24 24"><path d="M9 19c-4 1.2-4-2.2-5.5-2.8M15 21v-3.3c0-1 .1-1.4-.5-2 2.4-.3 4.5-1.2 4.5-5.1a4 4 0 0 0-1.1-2.7 3.7 3.7 0 0 0-.1-2.8s-.9-.3-3 1.1a10.2 10.2 0 0 0-5.4 0C7.3 4 6.4 4.3 6.4 4.3a3.7 3.7 0 0 0-.1 2.8A4 4 0 0 0 5.2 9.8c0 3.9 2.1 4.8 4.5 5.1-.5.5-.5 1-.5 1.7V21"/></svg>';
}

function iconAdmin() {
  return '<svg viewBox="0 0 24 24"><path d="M12 3l7.5 3v5.3c0 4.3-3 7.7-7.5 9.2-4.5-1.5-7.5-4.9-7.5-9.2V6z"/><path d="M9.3 12.2l1.9 1.9 3.6-3.8"/></svg>';
}

function iconFolder() {
  return '<svg viewBox="0 0 24 24"><path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4l2 2.4h7A1.5 1.5 0 0 1 19 9.9v7.6a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 3 17.5z"/></svg>';
}

function iconEdit() {
  return '<svg viewBox="0 0 24 24"><path d="M4 20h4l10.5-10.5a2.1 2.1 0 0 0-3-3L5 17v3z"/></svg>';
}

function iconTrash() {
  return '<svg viewBox="0 0 24 24"><path d="M4 7h16M10 7V5h4v2M6 7l1 12h10l1-12"/></svg>';
}

function iconCode() {
  return '<svg viewBox="0 0 24 24"><path d="M9 7l-5 5 5 5M15 7l5 5-5 5"/></svg>';
}

function iconLink() {
  return '<svg viewBox="0 0 24 24"><path d="M10.3 13.7a4 4 0 0 0 5.7 0l2.8-2.8a4 4 0 0 0-5.7-5.7l-1.6 1.6"/><path d="M13.7 10.3a4 4 0 0 0-5.7 0l-2.8 2.8a4 4 0 0 0 5.7 5.7l1.6-1.6"/></svg>';
}

function iconUp() {
  return '<svg viewBox="0 0 24 24"><path d="M7 14l5-5 5 5"/></svg>';
}

function iconDown() {
  return '<svg viewBox="0 0 24 24"><path d="M7 10l5 5 5-5"/></svg>';
}

function iconCaret() {
  return '<svg viewBox="0 0 24 24"><path d="M7 10l5 5 5-5"/></svg>';
}

function iconCheck() {
  return '<svg viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>';
}

/* --------------------------------------------------------------- démarrage */

invoke("bootstrap")
  .then((data) => {
    applyBootstrap(data);
    dom.search.focus();
  })
  .catch((error) => {
    dom.subtitle.textContent = "Erreur de chargement";
    toast(String(error), "error");
  });
