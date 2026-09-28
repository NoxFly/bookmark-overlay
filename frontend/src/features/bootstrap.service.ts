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

/* Application d'un instantané complet renvoyé par le backend. */

import type { Bootstrap } from "../types/backend.types.js";
import { state } from "../core/state.js";
import { renderList } from "./customer-list.component.js";
import { renderSettings } from "./settings.component.js";
import { applyTheme } from "./theme.service.js";
import { setUpdate } from "./update.component.js";

/**
 * @description Remplace tout l'état par l'instantané et redessine l'interface.
 */
export function applyBootstrap(data: Bootstrap): void {
    state.customers = data.customers;
    state.settings = data.settings;
    state.dataPath = data.dataPath;
    state.browsers = data.browsers;
    state.version = data.version;
    state.updatesEnabled = data.updatesEnabled;
    setUpdate(data.update);
    applyTheme(data.settings.theme);
    renderSettings(data.autostartEnabled);
    renderList();
}
