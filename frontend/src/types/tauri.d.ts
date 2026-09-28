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

/* API Tauri exposée sur `window.__TAURI__` (`withGlobalTauri`). Elle est utilisée
 * telle quelle : sans bundler, importer `@tauri-apps/api` n'est pas possible, et le
 * paquet n'apporterait rien de plus que ces deux fonctions. */

/** Évènement reçu du backend. */
interface TauriEvent<T> {
    payload: T;
}

interface TauriGlobal {
    core: {
        invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
    };
    event: {
        listen<T>(event: string, handler: (event: TauriEvent<T>) => void): Promise<() => void>;
    };
}

interface Window {
    __TAURI__: TauriGlobal;
}
