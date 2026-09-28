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

/* Éditeur des profils de navigation : un navigateur, un de ses profils, des hôtes. */

import type { BrowserKind, BrowserProfile, DetectedProfile, InstalledBrowser, Maybe } from "../types/backend.types.js";
import { closeOpenSelect, createSelect, type SelectController, type SelectOption } from "../components/select.component.js";
import { BROWSERS, type BrowserDescriptor } from "../core/constants.js";
import { dom } from "../core/dom-refs.js";
import { byId, closestTarget, el, query } from "../core/dom.helper.js";
import { iconDown, iconTrash, iconUp } from "../core/icons.helper.js";
import { state } from "../core/state.js";

/** Contrôles d'une carte de profil. */
interface ProfileCardControls {
    name: HTMLInputElement;
    hosts: HTMLInputElement;
    browserSelect: SelectController<BrowserKind>;
    profileSelect: SelectController<string>;
}

/** Contrôles de chaque carte, retrouvés à partir de son élément. */
const profileCards = new WeakMap<Element, ProfileCardControls>();

/** Compteur servant à nommer les étiquettes des listes créées à la volée. */
let labelSequence = 0;

/** Description d'un navigateur. */
function descriptorOf(kind: BrowserKind): Maybe<BrowserDescriptor> {
    return BROWSERS.find(browser => browser.kind === kind) ?? null;
}

/** État détecté d'un navigateur sur ce poste. */
function detectedBrowser(kind: BrowserKind): InstalledBrowser {
    return state.browsers.find(browser => browser.kind === kind) ?? { kind, installed: false, profiles: [] };
}

/**
 * @description Logo d'un navigateur ; une pastille à l'initiale le remplace si le fichier manque.
 */
export function browserLogo(kind: BrowserKind): HTMLElement {
    const descriptor = descriptorOf(kind);
    const file = descriptor ? descriptor.logo : `${kind}.png`;
    const image = el("img", { class: "browser-logo", attrs: { src: `icons/browsers/${file}`, alt: "", draggable: "false" } });
    image.addEventListener("error", () => {
        const label = descriptor ? descriptor.label : kind;
        image.replaceWith(el("span", { class: "avatar-initial", text: label.charAt(0) }));
    });
    return image;
}

/** Vignette d'un profil : sa photo, sinon son initiale sur sa couleur. */
function profileAvatar(profile: DetectedProfile): HTMLElement {
    if (profile.avatar) {
        return el("img", { class: "profile-avatar", attrs: { src: profile.avatar, alt: "", draggable: "false" } });
    }

    const initial = (profile.name || "?").charAt(0).toUpperCase();
    const node = el("span", { class: "avatar-initial", text: initial });
    if (profile.color) {
        node.style.background = profile.color;
        node.style.color = "#fff";
    }

    return node;
}

/** Groupe d'affichage : détectés, puis non détectés, puis navigateur par défaut. */
function browserRank(kind: BrowserKind, installed: boolean): number {
    if (kind === "system") {
        return 2;
    }

    return installed ? 0 : 1;
}

/** Options de la liste « navigateur », triées par groupe. */
function browserOptions(): SelectOption<BrowserKind>[] {
    const ranked = BROWSERS.map(descriptor => {
        const detected = detectedBrowser(descriptor.kind);
        const option: SelectOption<BrowserKind> = {
            value: descriptor.kind,
            main: descriptor.label,
            sub: detected.installed ? "" : "non détecté sur ce poste",
            icon: () => browserLogo(descriptor.kind),
        };
        return { option, rank: browserRank(descriptor.kind, detected.installed) };
    });
    // Le tri est stable : chaque groupe garde l'ordre de `BROWSERS`.
    ranked.sort((left, right) => left.rank - right.rank);
    return ranked.map(entry => entry.option);
}

/** Options de la liste « profil » pour un navigateur donné. */
function profileOptions(kind: BrowserKind): SelectOption<string>[] {
    if (!descriptorOf(kind)?.profiles) {
        return [{ value: "", main: "Profil unique", sub: "ce navigateur n'expose pas ses profils" }];
    }

    const detected = detectedBrowser(kind).profiles.map(profile => {
        const details = profile.account ? `${profile.id} — ${profile.account}` : profile.id;
        const option: SelectOption<string> = {
            value: profile.id,
            main: profile.name,
            sub: profile.isDefault ? `${details} · par défaut` : details,
            icon: () => profileAvatar(profile),
        };
        return option;
    });
    return [...detected, { value: "", main: "Profil par défaut du navigateur", sub: "aucun profil imposé" }];
}

/** Profil à présélectionner quand on choisit un navigateur : le défaut, sinon le seul. */
function defaultProfileOf(kind: BrowserKind): string {
    const profiles = detectedBrowser(kind).profiles;
    const preferred = profiles.find(profile => profile.isDefault) ?? profiles[0];
    return preferred ? preferred.id : "";
}

/** Bouton d'outil d'une carte. */
function toolButton(icon: string, action: string, label: string): HTMLButtonElement {
    return el("button", {
        class: "tool-btn",
        html: icon,
        attrs: { type: "button", "data-bp": action, title: label, "aria-label": label },
    });
}

/** Ajoute une carte d'édition de profil. */
function addBrowserProfileCard(profile: BrowserProfile): HTMLDivElement {
    labelSequence += 1;
    const browserLabelId = `bp-browser-label-${labelSequence}`;
    const profileLabelId = `bp-profile-label-${labelSequence}`;

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

    const profileSelect = createSelect<string>(profileSlot, () => {});

    /** Aligne la liste des profils sur le navigateur choisi. */
    const applyBrowser = (kind: BrowserKind, selectedProfile: string): void => {
        profileSelect.setOptions(profileOptions(kind), selectedProfile);
        profileSelect.setDisabled(!descriptorOf(kind)?.profiles);
    };

    const browserSelect = createSelect<BrowserKind>(browserSlot, kind => applyBrowser(kind, defaultProfileOf(kind)));
    browserSelect.setOptions(browserOptions(), profile.browser);
    applyBrowser(profile.browser, profile.profile);
    profileCards.set(card, { name, hosts, browserSelect, profileSelect });
    return card;
}

/** Renumérote les cartes et grise les déplacements impossibles. */
function refreshProfileCards(): void {
    const cards = Array.from(dom.browserProfiles.children);
    cards.forEach((card, index) => {
        query(card, ".profile-rank", HTMLSpanElement).textContent = String(index + 1);
        query(card, '[data-bp="up"]', HTMLButtonElement).disabled = index === 0;
        query(card, '[data-bp="down"]', HTMLButtonElement).disabled = index === cards.length - 1;
        // Il faut toujours un profil vers lequel envoyer les liens.
        query(card, '[data-bp="remove"]', HTMLButtonElement).disabled = cards.length === 1;
    });
}

/**
 * @description Reconstruit l'éditeur à partir des profils enregistrés.
 */
export function renderBrowserProfiles(profiles: BrowserProfile[]): void {
    dom.browserProfiles.replaceChildren();
    for (const profile of profiles) {
        addBrowserProfileCard(profile);
    }

    refreshProfileCards();
}

/**
 * @description Relit les profils saisis, dans l'ordre des cartes.
 */
export function collectBrowserProfiles(): BrowserProfile[] {
    const profiles: BrowserProfile[] = [];
    for (const card of dom.browserProfiles.children) {
        const controls = profileCards.get(card);
        const browser = controls?.browserSelect.getValue();
        if (!controls || !browser) {
            continue;
        }

        profiles.push({
            name: controls.name.value.trim(),
            browser,
            profile: controls.profileSelect.getValue() ?? "",
            hosts: controls.hosts.value
                .split(",")
                .map(host => host.trim())
                .filter(Boolean),
        });
    }

    return profiles;
}

/**
 * @description Branche les déplacements, suppressions et ajouts de cartes.
 */
export function initBrowserProfiles(): void {
    dom.browserProfiles.addEventListener("click", event => {
        const button = closestTarget(event, "[data-bp]");
        const card = button?.closest(".profile-card");
        if (!button || !card) {
            return;
        }

        closeOpenSelect();
        switch (button.dataset["bp"]) {
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

    byId("btn-add-profile", HTMLButtonElement).addEventListener("click", () => {
        const installed = BROWSERS.find(browser => detectedBrowser(browser.kind).installed);
        // À défaut de navigateur détecté, le premier de la liste, comme au départ.
        const kind = installed ? installed.kind : "edge";
        const card = addBrowserProfileCard({ name: "", browser: kind, profile: defaultProfileOf(kind), hosts: [] });
        refreshProfileCards();
        query(card, ".bp-name", HTMLInputElement).focus();
    });
}
