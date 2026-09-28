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

/* Pilotage au clavier : l'overlay s'utilise sans jamais quitter le clavier. */

import { closeOpenSelect } from "../components/select.component.js";
import { dom } from "../core/dom-refs.js";
import { state } from "../core/state.js";
import { openForm } from "./customer-form.component.js";
import { moveSelection, openFromKeyboard } from "./customer-list.component.js";
import { openSettings, settingsBack } from "./settings.component.js";
import { closeUpdateDialog, isUpdateDialogOpen } from "./update.component.js";
import { hideOverlay, showView } from "./views.service.js";

/** Échap ferme d'abord ce qui est ouvert par-dessus la vue, puis remonte d'un cran. */
function handleEscape(): void {
    if (closeOpenSelect()) {
        return;
    }

    if (isUpdateDialogOpen()) {
        closeUpdateDialog();
        return;
    }

    switch (state.view) {
        case "list":
            void hideOverlay();
            break;
        case "settings":
            settingsBack();
            break;
        case "form":
            showView("list");
            break;
    }
}

/** Touches de la vue liste. */
function handleListKey(event: KeyboardEvent): void {
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
            void openFromKeyboard(event);
            break;
        case "F2": {
            event.preventDefault();
            const customer = state.filtered[state.activeIndex];
            if (customer) {
                openForm(customer);
            }

            break;
        }
        default:
            // Toute autre touche alimente la recherche.
            if (event.key.length === 1 && !event.ctrlKey && !event.altKey) {
                dom.search.focus();
            }

            break;
    }
}

/**
 * @description Branche les raccourcis de l'overlay.
 */
export function initKeyboard(): void {
    document.addEventListener("keydown", event => {
        if (event.key === "Escape") {
            event.preventDefault();
            handleEscape();
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

        if (state.view === "list" && !isUpdateDialogOpen()) {
            handleListKey(event);
        }
    });
}
