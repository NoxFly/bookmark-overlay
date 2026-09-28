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

/* État applicatif, tenu en mémoire : la recherche ne déclenche aucun appel au
 * backend, seules les écritures traversent l'IPC. */

import type { CustomerView, InstalledBrowser, Maybe, Settings, UpdateInfo } from "../types/backend.types.js";

/** Vues principales du panneau. */
export type ViewName = "list" | "form" | "settings";

/** État partagé de l'interface. */
export interface AppState {
    customers: CustomerView[];
    settings: Maybe<Settings>;
    dataPath: string;
    browsers: InstalledBrowser[];
    version: string;
    update: Maybe<UpdateInfo>;
    installing: boolean;
    updatesEnabled: boolean;
    settingsSection: Maybe<string>;
    filtered: CustomerView[];
    activeIndex: number;
    view: ViewName;
    editingId: Maybe<string>;
    pendingDelete: Maybe<string>;
}

/** L'unique instance de l'état. */
export const state: AppState = {
    customers: [],
    settings: null,
    dataPath: "",
    browsers: [],
    version: "",
    update: null,
    installing: false,
    updatesEnabled: false,
    settingsSection: null,
    filtered: [],
    activeIndex: 0,
    view: "list",
    editingId: null,
    pendingDelete: null,
};

/**
 * @description Réglages chargés ; leur absence avant l'amorçage est une erreur de séquence.
 */
export function currentSettings(): Settings {
    if (!state.settings) {
        throw new Error("Réglages lus avant l'amorçage.");
    }

    return state.settings;
}
