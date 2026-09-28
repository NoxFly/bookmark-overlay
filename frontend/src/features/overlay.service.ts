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

/* Ouverture et masquage de l'overlay, pilotés par le backend. */

import { closeOpenSelect } from "../components/select.component.js";
import { dom } from "../core/dom-refs.js";
import { playEntrance } from "../core/dom.helper.js";
import { listen } from "../core/ipc.service.js";
import { state } from "../core/state.js";
import { renderList } from "./customer-list.component.js";
import { hideOverlay } from "./views.service.js";

/** Délai en deçà duquel deux signaux d'ouverture sont une seule et même ouverture. */
const SAME_OPENING_DELAY = 400;

/** Horodatage de la dernière ouverture traitée. */
let lastShownAt = 0;

/**
 * @description À chaque ouverture, la recherche précédente est intacte et sélectionnée :
 * une frappe la remplace, Entrée la réutilise.
 *
 * Un formulaire ou des réglages ouverts sont un travail en cours, et l'overlay se
 * referme dès qu'on va chercher quelque chose ailleurs (un dossier, un fichier à
 * importer). Le ramener de force à l'accueil jetterait la saisie : on reste donc
 * sur la vue en cours, et seule la liste se rafraîchit.
 */
function onOverlayShown(): void {
    // L'évènement du backend et le retour de focus arrivent tous les deux : sans
    // ce garde-fou, l'animation d'ouverture serait rejouée deux fois de suite.
    const now = performance.now();
    const isNewOpening = now - lastShownAt > SAME_OPENING_DELAY;
    lastShownAt = now;

    state.pendingDelete = null;
    closeOpenSelect();
    // Le panneau quitte son état d'attente et l'animation prend le relais dans la
    // même tâche : la première image affichée est celle du départ du glissement.
    document.body.classList.remove("is-closed");
    if (isNewOpening) {
        playEntrance(document.body, "is-opening", 240);
    }

    if (state.view !== "list") {
        return;
    }

    renderList();
    dom.search.focus();
    dom.search.select();
}

/**
 * @description Branche l'ouverture, le masquage et la fermeture par le voile.
 */
export function initOverlay(): void {
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

    dom.backdrop.addEventListener("mousedown", () => void hideOverlay());
    document.getElementById("btn-close")?.addEventListener("click", () => void hideOverlay());
}
