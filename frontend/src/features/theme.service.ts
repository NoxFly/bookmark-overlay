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

/* Apparence : traduit le réglage en thème effectif et suit Windows en mode système. */

import type { ThemeMode } from "../types/backend.types.js";
import { state } from "../core/state.js";

const lightQuery = window.matchMedia("(prefers-color-scheme: light)");

/**
 * @description Applique au document le thème correspondant au réglage.
 */
export function applyTheme(mode: ThemeMode): void {
    let resolved: "light" | "dark";
    if (mode === "system") {
        resolved = lightQuery.matches ? "light" : "dark";
    }
    else {
        resolved = mode;
    }

    document.documentElement.dataset["theme"] = resolved;
}

/**
 * @description En mode système, l'interface suit Windows en direct, sans rouvrir l'overlay.
 */
export function initTheme(): void {
    lightQuery.addEventListener("change", () => {
        if (state.settings?.theme === "system") {
            applyTheme("system");
        }
    });
}
