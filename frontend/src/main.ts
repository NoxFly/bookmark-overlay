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

/* Point d'entrée de l'interface.
 *
 * Tout l'état tient en mémoire : la recherche ne déclenche aucun appel au backend,
 * seules les écritures (fiches, réglages, import/export) traversent l'IPC.
 *
 * Les modules ne font rien à leur chargement : chacun expose une fonction `init…`,
 * appelée ici dans un ordre explicite. Les dépendances circulaires entre modules
 * (réglages ↔ mise à jour, par exemple) restent ainsi sans effet. */

import { initSelects } from "./components/select.component.js";
import { toast } from "./components/toast.component.js";
import { dom } from "./core/dom-refs.js";
import { byId } from "./core/dom.helper.js";
import { invokeQuietly } from "./core/ipc.service.js";
import { applyBootstrap } from "./features/bootstrap.service.js";
import { initBrowserProfiles } from "./features/browser-profiles.component.js";
import { initCustomerForm, openForm } from "./features/customer-form.component.js";
import { initCustomerList } from "./features/customer-list.component.js";
import { initHotkeyCapture } from "./features/hotkey-capture.component.js";
import { initKeyboard } from "./features/keyboard.service.js";
import { initOverlay } from "./features/overlay.service.js";
import { initSettings } from "./features/settings.component.js";
import { initTheme } from "./features/theme.service.js";
import { initUpdates } from "./features/update.component.js";

/** Branche toute l'interface, puis charge l'état initial. */
async function start(): Promise<void> {
    initTheme();
    initSelects();
    initCustomerList();
    initCustomerForm();
    initBrowserProfiles();
    initSettings();
    initHotkeyCapture();
    initUpdates();
    initKeyboard();
    initOverlay();
    byId("btn-new", HTMLButtonElement).addEventListener("click", () => openForm(null));

    try {
        applyBootstrap(await invokeQuietly("bootstrap"));
        dom.search.focus();
    }
    catch (error) {
        dom.subtitle.textContent = "Erreur de chargement";
        toast(String(error), "error");
    }
}

void start();
