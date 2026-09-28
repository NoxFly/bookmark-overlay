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

/* Outils DOM : fabrication d'éléments et accès typés. */

import type { Maybe } from "../types/backend.types.js";

/** Valeur d'attribut : `null` et `false` l'omettent, `true` le pose vide. */
type AttributeValue = string | number | boolean | null;

/** Options de fabrication d'un élément. */
export interface ElementOptions {
    class?: string;
    text?: string;
    html?: string;
    attrs?: Record<string, AttributeValue>;
}

/**
 * @description Crée un élément avec ses classes, attributs et enfants.
 */
export function el<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    options: ElementOptions = {},
    children: Node[] = [],
): HTMLElementTagNameMap[K] {
    const node = document.createElement(tag);
    if (options.class !== undefined) {
        node.className = options.class;
    }

    if (options.text !== undefined) {
        node.textContent = options.text;
    }

    if (options.html !== undefined) {
        node.innerHTML = options.html;
    }

    for (const [name, value] of Object.entries(options.attrs ?? {})) {
        if (value === null || value === false) {
            continue;
        }

        const serialized = value === true ? "" : String(value);
        node.setAttribute(name, serialized);
    }

    node.append(...children);
    return node;
}

/**
 * @description Élément de la page, de type attendu ; une absence est une erreur de
 * câblage entre le HTML et le script, signalée immédiatement.
 */
export function byId<T extends HTMLElement>(id: string, type: new () => T): T {
    const node = document.getElementById(id);
    if (!(node instanceof type)) {
        throw new Error(`Élément #${id} introuvable ou de type inattendu.`);
    }

    return node;
}

/**
 * @description Descendant de type attendu, sous la même garantie que `byId`.
 */
export function query<T extends Element>(root: ParentNode, selector: string, type: new () => T): T {
    const node = root.querySelector(selector);
    if (!(node instanceof type)) {
        throw new Error(`Élément « ${selector} » introuvable ou de type inattendu.`);
    }

    return node;
}

/**
 * @description Champ nommé d'un formulaire.
 */
export function field(form: HTMLFormElement, name: string): HTMLInputElement {
    const node = form.elements.namedItem(name);
    if (!(node instanceof HTMLInputElement)) {
        throw new Error(`Champ « ${name} » introuvable dans le formulaire.`);
    }

    return node;
}

/**
 * @description Ancêtre de la cible d'un évènement correspondant au sélecteur.
 */
export function closestTarget(event: Event, selector: string): Maybe<HTMLElement> {
    const target = event.target;
    if (!(target instanceof Element)) {
        return null;
    }

    const found = target.closest(selector);
    return found instanceof HTMLElement ? found : null;
}

/**
 * @description Applique une classe d'animation d'entrée, puis la retire au bout de `duration`.
 *
 * Le retrait est confié à un minuteur et non à `animationend` : une animation
 * partant de `opacity: 0` laisserait l'élément invisible pour toujours si le
 * compositeur n'avance pas — ce qui arrive juste après la reprise d'une webview
 * suspendue. Un minuteur, lui, se déclenche toujours.
 */
export function playEntrance(node: HTMLElement, className: string, duration: number): void {
    node.classList.remove(className);
    // Force un recalcul : sans lui, le retrait puis l'ajout seraient fusionnés en un
    // non-évènement et l'animation ne repartirait pas.
    void node.offsetWidth;
    node.classList.add(className);
    window.setTimeout(() => node.classList.remove(className), duration);
}
