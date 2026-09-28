/*
 * Bookmark Overlay
 * Copyright (C) 2026 NoxFly
 *
 * FR : Ce programme est un logiciel libre ; vous pouvez le redistribuer ou le
 * modifier selon les termes de la GNU Affero General Public License, version 3,
 * telle que publiée par la Free Software Foundation. Il est distribué dans
 * l'espoir d'être utile, mais SANS AUCUNE GARANTIE. Voir le fichier LICENSE.
 *
 * EN : This program is free software: you can redistribute it and/or modify it
 * under the terms of the GNU Affero General Public License, version 3, as
 * published by the Free Software Foundation. It is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY. See the LICENSE file.
 *
 * SPDX-License-Identifier: AGPL-3.0-only
 */

/* Liste des fiches : recherche, rendu, sélection au clavier et actions des cartes. */

import type { CustomerView, LinkKind, Maybe } from "../types/backend.types.js";
import { toast } from "../components/toast.component.js";
import { COMPUTED_LINKS, RENDER_LIMIT } from "../core/constants.js";
import { dom } from "../core/dom-refs.js";
import { closestTarget, el } from "../core/dom.helper.js";
import { iconCode, iconEdit, iconFolder, iconLink, iconTrash } from "../core/icons.helper.js";
import { call } from "../core/ipc.service.js";
import { state } from "../core/state.js";
import { fold, plural, searchTerms } from "../core/text.helper.js";
import { openForm } from "./customer-form.component.js";

/** Cibles calculées connues, pour relire sans risque celle portée par un bouton. */
const LINK_KINDS: readonly LinkKind[] = ["devops", "github", "adminCenter", "folder"];

/** Relit la cible d'un bouton ; `null` pour une valeur inconnue. */
function asLinkKind(value: string | undefined): Maybe<LinkKind> {
    return LINK_KINDS.find(kind => kind === value) ?? null;
}

/** Délai pendant lequel une suppression attend sa confirmation. */
const DELETE_CONFIRM_DELAY = 4000;

/**
 * @description Filtre et classe les fiches selon la recherche : le nom d'abord, puis le
 * tenant, puis les mots-clés ; un début de mot compte plus qu'une occurrence.
 */
export function filterCustomers(customers: CustomerView[], query: string): CustomerView[] {
    const terms = searchTerms(query);
    if (terms.length === 0) {
        return customers.slice();
    }

    const scored: { customer: CustomerView; score: number }[] = [];
    for (const customer of customers) {
        const score = matchScore(customer, terms);
        if (score !== null) {
            scored.push({ customer, score });
        }
    }

    scored.sort((left, right) => left.score - right.score);
    return scored.map(entry => entry.customer);
}

/** Score d'une fiche, plus faible pour une meilleure correspondance ; `null` si un terme manque. */
function matchScore(customer: CustomerView, terms: string[]): Maybe<number> {
    const name = fold(customer.name);
    const tenant = fold(customer.tenant);
    const keywords = customer.keywords.map(fold);

    let score = 0;
    for (const term of terms) {
        if (name.startsWith(term)) {
            score += 0;
        }
        else if (name.includes(term)) {
            score += 2;
        }
        else if (tenant.startsWith(term)) {
            score += 3;
        }
        else if (tenant.includes(term)) {
            score += 4;
        }
        else if (keywords.some(keyword => keyword.startsWith(term))) {
            score += 5;
        }
        else if (keywords.some(keyword => keyword.includes(term))) {
            score += 6;
        }
        else {
            return null;
        }
    }

    return score;
}

/** Nom de la fiche, la première occurrence du premier terme surlignée. */
function renderName(customer: CustomerView, query: string): HTMLDivElement {
    const target = el("div", { class: "card-name" });
    const term = searchTerms(query)[0];
    const index = term ? fold(customer.name).indexOf(term) : -1;
    if (!term || index < 0) {
        target.textContent = customer.name;
        return target;
    }

    const end = index + term.length;
    target.append(
        document.createTextNode(customer.name.slice(0, index)),
        el("mark", { text: customer.name.slice(index, end) }),
        document.createTextNode(customer.name.slice(end)),
    );
    return target;
}

/**
 * @description Reconstruit la liste des fiches à partir de la recherche courante.
 */
export function renderList(): void {
    const query = dom.search.value;
    state.filtered = filterCustomers(state.customers, query);
    state.pendingDelete = null;

    if (state.activeIndex >= state.filtered.length) {
        state.activeIndex = Math.max(0, state.filtered.length - 1);
    }

    const fragment = document.createDocumentFragment();
    const visible = state.filtered.slice(0, RENDER_LIMIT);
    visible.forEach((customer, index) => fragment.appendChild(buildCard(customer, index, query)));
    const hidden = state.filtered.length - visible.length;
    if (hidden > 0) {
        fragment.appendChild(el("p", { class: "empty-hint", text: `+ ${hidden} autres — affine ta recherche.` }));
    }

    dom.list.replaceChildren(fragment);

    // La liste vide est retirée du flux pour que le message occupe toute la
    // hauteur restante et s'y centre réellement.
    const isEmpty = state.filtered.length === 0;
    dom.list.hidden = isEmpty;
    dom.empty.hidden = !isEmpty;
    dom.clear.hidden = query.length === 0;

    const total = state.customers.length;
    const count = `${total} ${plural(total, "fiche")}`;
    dom.subtitle.textContent = query.length > 0 ? `${state.filtered.length} sur ${count}` : count;

    scrollActiveIntoView();
}

/** Carte d'une fiche. */
function buildCard(customer: CustomerView, index: number, query: string): HTMLElement {
    const isActive = index === state.activeIndex;
    const card = el("article", {
        class: isActive ? "card is-active" : "card",
        attrs: { "data-id": customer.id, role: "option", "aria-selected": String(isActive) },
    });

    const top: HTMLElement[] = [renderName(customer, query)];
    if (customer.tenant) {
        top.push(el("span", { class: "card-tenant", text: customer.tenant, attrs: { title: customer.tenant } }));
    }

    card.appendChild(el("div", { class: "card-top" }, top));

    if (customer.keywords.length > 0) {
        const chips = customer.keywords.map(keyword => el("span", { class: "chip", text: keyword }));
        card.appendChild(el("div", { class: "chips" }, chips));
    }

    // Une rangée pour ce qui part au navigateur, une autre pour ce qui s'ouvre en
    // local, chacune défilant pour elle-même. Seules les cibles renseignées ont un
    // bouton : une fiche n'affiche que ce qu'elle sait ouvrir.
    const webRow = el("div", { class: "card-actions" });
    for (const link of COMPUTED_LINKS) {
        const url = customer[link.urlField];
        if (url) {
            webRow.appendChild(linkButton(link.label, link.icon(), url, { "data-action": "open", "data-kind": link.kind }));
        }
    }

    for (const link of customer.links) {
        webRow.appendChild(linkButton(link.name, iconLink(), link.url, { "data-action": "link", "data-name": link.name }));
    }

    const localRow = el("div", { class: "card-actions" });
    if (customer.folderPath) {
        const attrs = { "data-action": "open", "data-kind": "folder" };
        localRow.appendChild(linkButton("Dossier", iconFolder(), customer.folderPath, attrs));
    }

    for (const workspace of customer.workspaces) {
        const attrs = { "data-action": "workspace", "data-name": workspace.name };
        localRow.appendChild(linkButton(workspace.name, iconCode(), workspace.path, attrs));
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
            attrs: { type: "button", "data-action": "delete", title: "Supprimer", "aria-label": "Supprimer" },
        }),
    ]);

    const rows = [webRow, localRow].filter(row => row.childElementCount > 0);
    // Les outils restent ancrés à droite, hors des bandes défilantes.
    card.appendChild(el("div", { class: "card-foot" }, [el("div", { class: "card-rows" }, rows), tools]));
    return card;
}

/** Bouton d'ouverture ; la cible est rappelée en infobulle. */
function linkButton(label: string, icon: string, target: string, attrs: Record<string, string>): HTMLButtonElement {
    const button = el("button", { class: "link-btn", attrs: { type: "button", title: target, ...attrs } });
    button.append(el("span", { class: "ico", html: icon }), document.createTextNode(label));
    return button;
}

/** Garde la fiche sélectionnée visible pendant la navigation au clavier. */
function scrollActiveIntoView(): void {
    dom.list.querySelector(".card.is-active")?.scrollIntoView({ block: "nearest" });
}

/**
 * @description Déplace la sélection sans reconstruire toute la liste.
 */
export function moveSelection(delta: number): void {
    if (state.filtered.length === 0) {
        return;
    }

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

/**
 * @description Remplace les fiches après une écriture, en conservant recherche et sélection.
 */
export function applyCustomers(customers: CustomerView[]): void {
    state.customers = customers;
    renderList();
}

/**
 * @description Ouvre une cible calculée d'une fiche.
 */
export async function openLink(id: string, kind: LinkKind): Promise<void> {
    await call("open_link", { id, kind });
}

/**
 * @description Ouvre la cible de la fiche sélectionnée correspondant aux modificateurs :
 * Ctrl → DevOps, Maj → GitHub, Alt → Admin Center, sinon le dossier ou le premier
 * espace de travail.
 */
export async function openFromKeyboard(event: KeyboardEvent): Promise<void> {
    const customer = state.filtered[state.activeIndex];
    if (!customer) {
        return;
    }

    if (event.ctrlKey) {
        await openLink(customer.id, "devops");
        return;
    }

    if (event.shiftKey) {
        await openLink(customer.id, "github");
        return;
    }

    if (event.altKey) {
        await openLink(customer.id, "adminCenter");
        return;
    }

    const firstWorkspace = customer.workspaces[0];
    if (!customer.folderPath && firstWorkspace) {
        await call("open_workspace", { id: customer.id, name: firstWorkspace.name });
        return;
    }

    await openLink(customer.id, "folder");
}

/** Supprime au second clic : le premier transforme le bouton en demande de confirmation. */
async function handleDelete(id: string, button: HTMLElement): Promise<void> {
    if (state.pendingDelete === id) {
        state.pendingDelete = null;
        applyCustomers(await call("delete_customer", { id }));
        toast("Fiche supprimée.");
        return;
    }

    state.pendingDelete = id;
    button.classList.add("confirm");
    button.textContent = "Confirmer";
    window.setTimeout(() => {
        if (state.pendingDelete === id) {
            state.pendingDelete = null;
            renderList();
        }
    }, DELETE_CONFIRM_DELAY);
}

/** Exécute l'action d'un bouton de carte. */
async function handleCardAction(id: string, button: HTMLElement): Promise<void> {
    const name = button.dataset["name"] ?? "";
    switch (button.dataset["action"]) {
        case "open": {
            const kind = asLinkKind(button.dataset["kind"]);
            if (kind) {
                await openLink(id, kind);
            }

            break;
        }
        case "workspace":
            await call("open_workspace", { id, name });
            break;
        case "link":
            await call("open_custom_link", { id, name });
            break;
        case "edit": {
            const customer = state.customers.find(entry => entry.id === id);
            if (customer) {
                openForm(customer);
            }

            break;
        }
        case "delete":
            await handleDelete(id, button);
            break;
        default:
            break;
    }
}

/**
 * @description Branche la recherche et les actions des cartes.
 */
export function initCustomerList(): void {
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

    dom.list.addEventListener("click", async event => {
        const card = closestTarget(event, ".card");
        const id = card?.dataset["id"];
        if (!card || !id) {
            return;
        }

        const index = Array.from(dom.list.querySelectorAll(".card")).indexOf(card);
        if (index >= 0 && index !== state.activeIndex) {
            moveSelection(index - state.activeIndex);
        }

        const button = closestTarget(event, "[data-action]");
        if (button) {
            await handleCardAction(id, button);
        }
    });
}
