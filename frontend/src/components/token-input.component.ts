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

/* Champs de gabarit d'URL aux jetons colorés. */

import { TEMPLATE_TOKENS } from "../core/constants.js";
import { el } from "../core/dom.helper.js";

/** Évènements après lesquels le défilement horizontal du champ a pu changer. */
const SCROLL_EVENTS = ["scroll", "keydown", "keyup", "mousedown", "mouseup", "select", "focus", "blur"] as const;

/**
 * @description Colore les jetons d'un champ de gabarit, et retourne la fonction qui
 * redessine le calque : un remplissage par programme n'émet pas `input`.
 *
 * Un calque, aligné au pixel sur le champ, redessine son texte par-dessus lui. Le
 * texte du champ est rendu transparent, mais son curseur et sa sélection restent
 * visibles au travers. Seule la couleur change sur un jeton, jamais la graisse : la
 * moindre différence de chasse décalerait le calque du curseur.
 */
export function enhanceTokenInput(input: HTMLInputElement): () => void {
    const content = el("span", { class: "token-mirror-content" });
    const mirror = el("div", { class: "token-mirror", attrs: { "aria-hidden": "true" } }, [content]);
    const wrapper = el("div", { class: "token-input" });
    input.replaceWith(wrapper);
    wrapper.append(input, mirror);

    const sync = (): void => {
        content.style.transform = `translateX(${-input.scrollLeft}px)`;
    };
    const render = (): void => {
        content.replaceChildren(...tokenize(input.value));
        sync();
    };

    input.addEventListener("input", render);
    for (const type of SCROLL_EVENTS) {
        input.addEventListener(type, () => requestAnimationFrame(sync));
    }

    return render;
}

/**
 * @description Découpe un gabarit en texte brut et jetons, reconnus ou non.
 */
export function tokenize(value: string): Node[] {
    const nodes: Node[] = [];
    let last = 0;
    for (const match of value.matchAll(/\{(?<name>[A-Za-z]+)\}/g)) {
        if (match.index > last) {
            nodes.push(document.createTextNode(value.slice(last, match.index)));
        }

        const known = TEMPLATE_TOKENS.includes(match.groups?.["name"] ?? "");
        const className = known ? "token" : "token is-unknown";
        nodes.push(el("span", { class: className, text: match[0] }));
        last = match.index + match[0].length;
    }

    if (last < value.length) {
        nodes.push(document.createTextNode(value.slice(last)));
    }

    return nodes;
}
