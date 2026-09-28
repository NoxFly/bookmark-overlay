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

/* Réglages : menu des sections, sections, préférences appliquées immédiatement. */

import type { ImportMode, Settings } from "../types/backend.types.js";
import { closeOpenSelect } from "../components/select.component.js";
import { toast } from "../components/toast.component.js";
import { enhanceTokenInput } from "../components/token-input.component.js";
import { THEMES } from "../core/constants.js";
import { dom } from "../core/dom-refs.js";
import { byId, closestTarget, el, field } from "../core/dom.helper.js";
import { iconCheck } from "../core/icons.helper.js";
import { call } from "../core/ipc.service.js";
import { currentSettings, state } from "../core/state.js";
import { plural } from "../core/text.helper.js";
import { applyBootstrap } from "./bootstrap.service.js";
import { collectBrowserProfiles, renderBrowserProfiles } from "./browser-profiles.component.js";
import { renderList } from "./customer-list.component.js";
import { formatAccelerator } from "./hotkey-capture.component.js";
import { applyTheme } from "./theme.service.js";
import { renderVersion } from "./update.component.js";
import { showView } from "./views.service.js";

/** Champs des gabarits d'URL, par nom. */
type TemplateField = "devopsUrlTemplate" | "githubUrlTemplate" | "adminCenterUrlTemplate";

/** Redessine les calques des gabarits après un remplissage par programme. */
let tokenRenderers: (() => void)[] = [];

/** Champ de gabarit. */
function templateInput(name: TemplateField): HTMLInputElement {
    return field(dom.settingsForm, name);
}

/**
 * @description Remplit les réglages avec l'état enregistré ; une saisie non enregistrée
 * est abandonnée.
 */
export function renderSettings(autostartEnabled?: boolean): void {
    const settings = currentSettings();
    templateInput("devopsUrlTemplate").value = settings.devopsUrlTemplate;
    templateInput("githubUrlTemplate").value = settings.githubUrlTemplate;
    templateInput("adminCenterUrlTemplate").value = settings.adminCenterUrlTemplate;
    dom.hotkeyInput.value = formatAccelerator(settings.hotkey);
    for (const render of tokenRenderers) {
        render();
    }

    dom.dataPath.textContent = state.dataPath;
    renderVersion();
    if (autostartEnabled !== undefined) {
        dom.autostart.checked = autostartEnabled;
    }

    renderBrowserProfiles(settings.browserProfiles);
    renderThemeList();
    renderSettingsSummaries();
}

/**
 * @description Ouvre les réglages sur leur menu.
 */
export function openSettings(): void {
    renderSettings();
    showView("settings");
    showSettingsMenu();
}

/** Affiche le menu des sections. */
function showSettingsMenu(): void {
    closeOpenSelect();
    state.settingsSection = null;
    dom.settingsTitle.textContent = "Réglages";
    dom.settingsForm.hidden = true;
    dom.settingsMenu.hidden = false;
    renderSettingsSummaries();
    dom.settingsMenu.querySelector<HTMLElement>(".menu-item")?.focus();
}

/** Affiche une section, seule. */
function showSettingsSection(name: string): void {
    closeOpenSelect();
    state.settingsSection = name;
    let savable = false;
    for (const section of dom.settingsForm.querySelectorAll<HTMLElement>(".settings-section")) {
        const isCurrent = section.dataset["section"] === name;
        section.hidden = !isCurrent;
        if (isCurrent) {
            dom.settingsTitle.textContent = section.dataset["title"] ?? "Réglages";
            savable = section.hasAttribute("data-savable");
        }
    }

    // Apparence, système et données s'appliquent immédiatement : rien à enregistrer.
    dom.settingsActions.hidden = !savable;
    dom.settingsMenu.hidden = true;
    dom.settingsForm.hidden = false;
    dom.settingsForm.scrollTop = 0;
    const selector = `[data-section="${name}"] input, [data-section="${name}"] button`;
    dom.settingsForm.querySelector<HTMLElement>(selector)?.focus();
}

/**
 * @description Revient d'un cran : de la section au menu, du menu à la liste.
 */
export function settingsBack(): void {
    if (state.settingsSection) {
        // Abandonner une section non enregistrée remet ses champs à l'état stocké.
        renderSettings();
        showSettingsMenu();
        return;
    }

    showView("list");
}

/** Résumés affichés sous les entrées du menu. */
function renderSettingsSummaries(): void {
    const settings = state.settings;
    if (!settings) {
        return;
    }

    const count = settings.browserProfiles.length;
    const theme = THEMES.find(entry => entry.value === settings.theme);
    byId("summary-browsers", HTMLSpanElement).textContent = `${count} ${plural(count, "profil")}`;
    byId("summary-appearance", HTMLSpanElement).textContent = theme ? `Thème ${theme.main.toLowerCase()}` : "Thème";
    byId("summary-system", HTMLSpanElement).textContent = `Raccourci ${formatAccelerator(settings.hotkey)}`;
}

/**
 * @description Enregistre tout de suite une préférence (thème, raccourci, mise à jour),
 * sans toucher aux champs des autres sections : une saisie en cours n'est pas perdue.
 */
export async function persistSettings(patch: Partial<Settings>): Promise<void> {
    const data = await call("save_settings", { settings: { ...currentSettings(), ...patch } });
    state.settings = data.settings;
    state.customers = data.customers;
    applyTheme(data.settings.theme);
    renderThemeList();
    dom.hotkeyInput.value = formatAccelerator(data.settings.hotkey);
    renderSettingsSummaries();
    renderList();
}

/** Liste à plat des apparences, l'active cochée. */
function renderThemeList(): void {
    const current = state.settings?.theme ?? "system";
    dom.themeList.replaceChildren(
        ...THEMES.map(theme => {
            const texts = [el("span", { class: "select-main", text: theme.main })];
            if (theme.sub) {
                texts.push(el("span", { class: "choice-sub", text: theme.sub }));
            }

            return el(
                "button",
                {
                    class: "choice",
                    attrs: {
                        type: "button",
                        role: "radio",
                        "aria-checked": String(theme.value === current),
                        "data-theme": theme.value,
                    },
                },
                [el("span", { class: "select-texts" }, texts), el("span", { class: "choice-check", html: iconCheck() })],
            );
        }),
    );
}

/** Applique l'apparence choisie et l'enregistre aussitôt. */
async function chooseTheme(event: MouseEvent): Promise<void> {
    const choice = closestTarget(event, ".choice");
    const theme = THEMES.find(entry => entry.value === choice?.dataset["theme"]);
    if (!theme || theme.value === state.settings?.theme) {
        return;
    }

    applyTheme(theme.value);
    await persistSettings({ theme: theme.value });
}

/** Enregistre les sections à bouton (gabarits, profils de navigation). */
async function submitSettings(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const settings: Settings = {
        ...currentSettings(),
        devopsUrlTemplate: templateInput("devopsUrlTemplate").value,
        githubUrlTemplate: templateInput("githubUrlTemplate").value,
        adminCenterUrlTemplate: templateInput("adminCenterUrlTemplate").value,
        browserProfiles: collectBrowserProfiles(),
    };
    applyBootstrap(await call("save_settings", { settings }));
    toast("Réglages enregistrés.");
    showSettingsMenu();
}

/** Importe un fichier, puis recharge tout l'état. */
async function importData(mode: ImportMode): Promise<void> {
    const report = await call("import_data", { mode });
    if (!report) {
        return;
    }

    applyBootstrap(await call("bootstrap"));
    toast(
        `Import terminé : ${report.added} ajoutée(s), ${report.updated} mise(s) à jour, `
            + `${report.skipped} inchangée(s). Sauvegarde : ${report.backupPath}`,
    );
}

/**
 * @description Branche le menu, les sections, les préférences et les actions sur les données.
 */
export function initSettings(): void {
    tokenRenderers = Array.from(dom.settingsForm.querySelectorAll<HTMLInputElement>("input[data-tokens]"), enhanceTokenInput);

    dom.settingsMenu.addEventListener("click", event => {
        const section = closestTarget(event, ".menu-item")?.dataset["section"];
        if (section) {
            showSettingsSection(section);
        }
    });

    byId("btn-settings", HTMLButtonElement).addEventListener("click", openSettings);
    byId("settings-back", HTMLButtonElement).addEventListener("click", settingsBack);
    byId("settings-cancel", HTMLButtonElement).addEventListener("click", settingsBack);
    dom.themeList.addEventListener("click", chooseTheme);
    dom.settingsForm.addEventListener("submit", submitSettings);

    dom.autostart.addEventListener("change", async () => {
        const enabled = await call("set_autostart", { enabled: dom.autostart.checked });
        dom.autostart.checked = enabled;
        toast(enabled ? "Lancement au démarrage activé." : "Lancement au démarrage désactivé.");
    });

    byId("btn-reveal", HTMLButtonElement).addEventListener("click", async () => {
        await call("reveal_data_folder");
    });

    byId("btn-export", HTMLButtonElement).addEventListener("click", async () => {
        const path = await call("export_data");
        if (path) {
            toast(`Exporté (chiffré) vers ${path}`);
        }
    });

    byId("btn-export-plain", HTMLButtonElement).addEventListener("click", async () => {
        const path = await call("export_plain_data");
        if (path) {
            toast(`Exporté en clair vers ${path} — à ne pas laisser traîner.`);
        }
    });

    byId("btn-import-merge", HTMLButtonElement).addEventListener("click", () => importData("merge"));
    byId("btn-import-replace", HTMLButtonElement).addEventListener("click", () => importData("replace"));
}
