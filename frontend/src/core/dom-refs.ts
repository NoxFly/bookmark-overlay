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

/* Éléments de la page utilisés par plusieurs modules, résolus une seule fois. Le
 * script est un module ES, exécuté après l'analyse du document : tous existent. */

import { byId } from "./dom.helper.js";

export const dom = {
    backdrop: byId("backdrop", HTMLDivElement),
    subtitle: byId("subtitle", HTMLParagraphElement),
    search: byId("search", HTMLInputElement),
    clear: byId("btn-clear", HTMLButtonElement),
    list: byId("list", HTMLDivElement),
    empty: byId("empty", HTMLDivElement),
    views: {
        list: byId("view-list", HTMLElement),
        form: byId("view-form", HTMLElement),
        settings: byId("view-settings", HTMLElement),
    },
    form: byId("form", HTMLFormElement),
    formPreview: byId("form-preview", HTMLDivElement),
    workspaces: byId("workspaces", HTMLDivElement),
    links: byId("links", HTMLDivElement),
    settingsForm: byId("settings-form", HTMLFormElement),
    settingsMenu: byId("settings-menu", HTMLElement),
    settingsTitle: byId("settings-title", HTMLHeadingElement),
    settingsActions: byId("settings-actions", HTMLDivElement),
    autostart: byId("autostart", HTMLInputElement),
    dataPath: byId("data-path", HTMLParagraphElement),
    browserProfiles: byId("browser-profiles", HTMLDivElement),
    themeList: byId("theme-list", HTMLDivElement),
    hotkeyInput: byId("hotkey", HTMLInputElement),
    hotkeyButton: byId("btn-record-hotkey", HTMLButtonElement),
    hotkeyDialog: byId("hotkey-dialog", HTMLDivElement),
    hotkeyPreview: byId("hotkey-preview", HTMLDivElement),
    hotkeyMessage: byId("hotkey-message", HTMLParagraphElement),
    updateButton: byId("btn-update", HTMLButtonElement),
    updateDialog: byId("update-dialog", HTMLDivElement),
    updateVersion: byId("update-version", HTMLElement),
    updateDate: byId("update-date", HTMLElement),
    updateSize: byId("update-size", HTMLElement),
    updateFlavor: byId("update-flavor", HTMLElement),
    updateProgress: byId("update-progress", HTMLDivElement),
    updateStatus: byId("update-status", HTMLParagraphElement),
    updateInstall: byId("btn-update-install", HTMLButtonElement),
    updateLater: byId("btn-update-later", HTMLButtonElement),
    checkUpdate: byId("btn-check-update", HTMLButtonElement),
    autoUpdate: byId("auto-update", HTMLInputElement),
} as const;
