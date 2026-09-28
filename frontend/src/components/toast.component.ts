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

/* Message éphémère en bas de l'écran. Ce module ne dépend d'aucun autre : le
 * service IPC s'en sert pour afficher ses erreurs. */

/** Tonalité d'un message. */
export type ToastTone = "ok" | "error";

let timer = 0;

/**
 * @description Affiche un message éphémère ; une erreur reste affichée plus longtemps.
 */
export function toast(message: string, tone: ToastTone = "ok"): void {
    const node = document.getElementById("toast");
    if (!node) {
        return;
    }

    window.clearTimeout(timer);
    node.textContent = message;
    node.className = `toast is-${tone}`;
    node.hidden = false;
    // Un cycle de rendu est nécessaire pour que la transition d'opacité s'applique.
    requestAnimationFrame(() => node.classList.add("is-visible"));

    const duration = tone === "error" ? 5200 : 2600;
    timer = window.setTimeout(() => {
        node.classList.remove("is-visible");
        timer = window.setTimeout(() => {
            node.hidden = true;
        }, 200);
    }, duration);
}
