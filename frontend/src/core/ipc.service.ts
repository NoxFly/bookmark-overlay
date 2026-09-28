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

/* Frontière avec le backend : commandes et évènements typés. Le nom d'une commande
 * détermine ses arguments et son résultat, si bien qu'un appel mal formé ne compile
 * pas. */

import type {
    Bootstrap,
    Customer,
    CustomerInput,
    CustomerView,
    ImportMode,
    ImportReport,
    LinkKind,
    Maybe,
    Settings,
    UpdateInfo,
    UpdateProgress,
} from "../types/backend.types.js";
import { toast } from "../components/toast.component.js";

/** Commandes Rust exposées à la webview : arguments et résultat de chacune. */
interface Commands {
    bootstrap: { args: undefined; result: Bootstrap };
    create_customer: { args: { customer: CustomerInput }; result: CustomerView[] };
    update_customer: { args: { id: string; customer: CustomerInput }; result: CustomerView[] };
    delete_customer: { args: { id: string }; result: CustomerView[] };
    save_settings: { args: { settings: Settings }; result: Bootstrap };
    open_link: { args: { id: Customer["id"]; kind: LinkKind }; result: null };
    open_workspace: { args: { id: string; name: string }; result: null };
    open_custom_link: { args: { id: string; name: string }; result: null };
    hide_overlay: { args: undefined; result: null };
    reveal_data_folder: { args: undefined; result: null };
    set_autostart: { args: { enabled: boolean }; result: boolean };
    pick_folder: { args: undefined; result: Maybe<string> };
    pick_workspace_file: { args: undefined; result: Maybe<string> };
    export_data: { args: undefined; result: Maybe<string> };
    export_plain_data: { args: undefined; result: Maybe<string> };
    import_data: { args: { mode: ImportMode }; result: Maybe<ImportReport> };
    install_update: { args: undefined; result: null };
    check_update: { args: undefined; result: Maybe<UpdateInfo> };
    open_update_page: { args: undefined; result: null };
    pause_shortcut: { args: undefined; result: null };
    resume_shortcut: { args: undefined; result: null };
}

/** Évènements émis par le backend, et leur charge utile. */
interface Events {
    "overlay://shown": null;
    "overlay://hidden": null;
    "update://available": UpdateInfo;
    "update://progress": UpdateProgress;
}

/** Nom d'une commande. */
export type CommandName = keyof Commands;

/** Arguments d'une commande : aucun quand elle n'en attend pas. */
type CommandArgs<K extends CommandName> = Commands[K]["args"] extends undefined ? [] : [Commands[K]["args"]];

/**
 * @description Appelle une commande sans rien afficher en cas d'échec.
 */
export async function invokeQuietly<K extends CommandName>(
    command: K,
    ...args: CommandArgs<K>
): Promise<Commands[K]["result"]> {
    const [payload] = args;
    return await window.__TAURI__.core.invoke<Commands[K]["result"]>(command, payload);
}

/**
 * @description Appelle une commande et affiche son erreur, lisible, avant de la relancer.
 */
export async function call<K extends CommandName>(command: K, ...args: CommandArgs<K>): Promise<Commands[K]["result"]> {
    try {
        return await invokeQuietly(command, ...args);
    }
    catch (error) {
        toast(String(error), "error");
        throw error;
    }
}

/**
 * @description S'abonne à un évènement du backend.
 */
export function listen<K extends keyof Events>(event: K, handler: (payload: Events[K]) => void): void {
    // L'abonnement dure autant que la page : la fonction de désabonnement est
    // volontairement ignorée, et un échec d'abonnement n'a pas de recours.
    void window.__TAURI__.event.listen<Events[K]>(event, received => handler(received.payload));
}
