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

/* Navigation entre les vues du panneau, et fermeture de l'overlay. */

import { closeOpenSelect } from "../components/select.component.js";
import { dom } from "../core/dom-refs.js";
import { invokeQuietly } from "../core/ipc.service.js";
import { state, type ViewName } from "../core/state.js";

/**
 * @description Bascule vers une vue et y place le focus.
 */
export function showView(name: ViewName): void {
    closeOpenSelect();
    state.view = name;
    for (const [key, node] of Object.entries(dom.views)) {
        node.hidden = key !== name;
    }

    if (name === "list") {
        dom.search.focus();
        dom.search.select();
        return;
    }

    const first = dom.views[name].querySelector<HTMLElement>("input, .select-trigger");
    first?.focus();
}

/**
 * @description Ferme l'overlay.
 */
export async function hideOverlay(): Promise<void> {
    closeOpenSelect();
    try {
        await invokeQuietly("hide_overlay");
    }
    catch {
        // Un masquage refusé laisse simplement l'overlay affiché : rien à signaler.
    }
}
