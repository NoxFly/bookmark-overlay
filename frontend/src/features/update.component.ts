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

/* Mise à jour : icône d'en-tête, fenêtre, recherche manuelle, mode automatique. */

import type { Maybe, UpdateInfo } from "../types/backend.types.js";
import { closeOpenSelect } from "../components/select.component.js";
import { toast } from "../components/toast.component.js";
import { dom } from "../core/dom-refs.js";
import { byId, playEntrance } from "../core/dom.helper.js";
import { call, listen } from "../core/ipc.service.js";
import { state } from "../core/state.js";
import { formatSize } from "../core/text.helper.js";
import { persistSettings } from "./settings.component.js";

/** Libellé du bouton de recherche au repos. */
const CHECK_LABEL = "Rechercher une mise à jour";

/**
 * @description Retient la mise à jour disponible et affiche, ou non, son icône.
 */
export function setUpdate(update: Maybe<UpdateInfo>): void {
    state.update = update;
    dom.updateButton.hidden = !update;
    if (update) {
        dom.updateButton.title = `Mise à jour ${update.version} disponible`;
    }
}

/** Date de publication lisible. */
function formatPublication(publishedAt: string): string {
    if (!publishedAt) {
        return "—";
    }

    return new Date(publishedAt).toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}

/**
 * @description Ouvre la fenêtre de mise à jour.
 */
export function openUpdateDialog(): void {
    const update = state.update;
    if (!update) {
        return;
    }

    closeOpenSelect();
    dom.updateVersion.textContent = `${state.version} → ${update.version}`;
    dom.updateDate.textContent = formatPublication(update.publishedAt);
    dom.updateSize.textContent = formatSize(update.size);
    dom.updateFlavor.textContent = update.flavor === "installer" ? "installateur" : "portable";
    if (!state.installing) {
        dom.updateProgress.hidden = true;
        dom.updateStatus.hidden = true;
    }

    dom.updateDialog.hidden = false;
    playEntrance(dom.updateDialog, "is-entering", 200);
    dom.updateInstall.focus();
}

/**
 * @description Ferme la fenêtre de mise à jour, sauf pendant l'installation.
 */
export function closeUpdateDialog(): void {
    if (!state.installing) {
        dom.updateDialog.hidden = true;
    }
}

/**
 * @description Vrai si la fenêtre de mise à jour est affichée.
 */
export function isUpdateDialogOpen(): boolean {
    return !dom.updateDialog.hidden;
}

/** Affiche l'avancement de l'installation. */
function showProgress(downloaded: number, total: number): void {
    const ratio = total > 0 ? Math.min(1, downloaded / total) : 0;
    const bar = dom.updateProgress.firstElementChild;
    if (bar instanceof HTMLElement) {
        bar.style.width = `${(ratio * 100).toFixed(1)}%`;
    }

    dom.updateProgress.hidden = false;
    dom.updateStatus.hidden = false;
    dom.updateStatus.textContent =
        ratio < 1
            ? `Téléchargement… ${formatSize(downloaded)} sur ${formatSize(total)}`
            : "Installation, l'application va redémarrer…";
}

/** Télécharge et installe la mise à jour ; l'application redémarre d'elle-même. */
async function installUpdate(): Promise<void> {
    if (state.installing || !state.update) {
        return;
    }

    state.installing = true;
    dom.updateInstall.disabled = true;
    dom.updateLater.disabled = true;
    showProgress(0, state.update.size);

    try {
        await call("install_update");
    }
    catch {
        // L'erreur est déjà affichée : on rend la main pour permettre un nouvel essai.
        state.installing = false;
        dom.updateInstall.disabled = false;
        dom.updateLater.disabled = false;
        dom.updateProgress.hidden = true;
        dom.updateStatus.hidden = true;
    }
}

/**
 * @description Version courante, et disponibilité de la recherche et du mode automatique.
 */
export function renderVersion(): void {
    byId("app-version", HTMLElement).textContent = state.version;
    byId("settings-version", HTMLParagraphElement).textContent = `Bookmark Overlay ${state.version}`;
    dom.checkUpdate.disabled = !state.updatesEnabled;
    dom.autoUpdate.checked = state.settings?.autoUpdate ?? false;
    dom.autoUpdate.disabled = !state.updatesEnabled;
    byId("update-hint", HTMLElement).textContent = state.updatesEnabled ? "" : "build local : aucune mise à jour suivie";
}

/** Recherche une mise à jour à la demande, et ouvre la fenêtre si elle en trouve une. */
async function checkForUpdate(): Promise<void> {
    dom.checkUpdate.disabled = true;
    dom.checkUpdate.textContent = "Recherche…";
    try {
        const update = await call("check_update");
        if (update) {
            setUpdate(update);
            openUpdateDialog();
        }
        else {
            toast(`Bookmark Overlay est à jour (${state.version}).`);
        }
    }
    catch {
        // Le message d'erreur est déjà affiché par `call`.
    }
    finally {
        dom.checkUpdate.textContent = CHECK_LABEL;
        dom.checkUpdate.disabled = !state.updatesEnabled;
    }
}

/** Active ou coupe l'installation automatique. */
async function toggleAutoUpdate(): Promise<void> {
    const enabled = dom.autoUpdate.checked;
    try {
        await persistSettings({ autoUpdate: enabled });
        toast(enabled ? "Mises à jour automatiques activées." : "Mises à jour automatiques désactivées.");
    }
    catch {
        // Le message d'erreur est déjà affiché : la bascule reprend l'état enregistré.
        dom.autoUpdate.checked = !enabled;
    }
}

/**
 * @description Branche l'icône, la fenêtre, la recherche manuelle et les évènements.
 */
export function initUpdates(): void {
    dom.checkUpdate.addEventListener("click", checkForUpdate);
    dom.autoUpdate.addEventListener("change", toggleAutoUpdate);
    dom.updateButton.addEventListener("click", openUpdateDialog);
    byId("btn-update-page", HTMLButtonElement).addEventListener("click", async () => {
        await call("open_update_page");
    });
    dom.updateLater.addEventListener("click", closeUpdateDialog);
    dom.updateInstall.addEventListener("click", installUpdate);
    dom.updateDialog.addEventListener("mousedown", event => {
        if (event.target === dom.updateDialog) {
            closeUpdateDialog();
        }
    });

    listen("update://available", setUpdate);
    listen("update://progress", progress => showProgress(progress.downloaded, progress.total));
}
