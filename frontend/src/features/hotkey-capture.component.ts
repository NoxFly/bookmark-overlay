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

/* Capture du raccourci global au clavier, à la manière de Visual Studio Code. */

import type { Maybe } from "../types/backend.types.js";
import { closeOpenSelect } from "../components/select.component.js";
import { toast } from "../components/toast.component.js";
import { dom } from "../core/dom-refs.js";
import { byId, el, playEntrance } from "../core/dom.helper.js";
import { call, listen } from "../core/ipc.service.js";
import { currentSettings } from "../core/state.js";
import { persistSettings } from "./settings.component.js";

/** Modificateurs de l'accélérateur, dans son ordre canonique. */
type Modifier = "Ctrl" | "Alt" | "Shift" | "Super";

/** Touches de modification, reconnues à leur code physique. */
const MODIFIER_CODES: ReadonlySet<string> = new Set([
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "ShiftLeft",
    "ShiftRight",
    "MetaLeft",
    "MetaRight",
]);

/** Libellés affichés des modificateurs ; l'accélérateur garde les noms anglais. */
const MODIFIER_LABELS: Readonly<Record<string, string>> = { Ctrl: "Ctrl", Alt: "Alt", Shift: "Maj", Super: "Win" };

/** Capture en cours : le dernier accélérateur complet saisi. */
let capture: Maybe<{ accelerator: Maybe<string> }> = null;

/** Modificateurs enfoncés, dans l'ordre canonique de l'accélérateur. */
function modifiersOf(event: KeyboardEvent): Modifier[] {
    const modifiers: Modifier[] = [];
    if (event.ctrlKey) {
        modifiers.push("Ctrl");
    }

    if (event.altKey) {
        modifiers.push("Alt");
    }

    if (event.shiftKey) {
        modifiers.push("Shift");
    }

    if (event.metaKey) {
        modifiers.push("Super");
    }

    return modifiers;
}

/**
 * @description Nom de touche compris par le parseur d'accélérateurs : le code physique
 * (`KeyD`, `Digit1`, `F5`, `Space`…), raccourci en `D` ou `1` pour la lisibilité.
 * Le code physique, contrairement au caractère, ne dépend pas de la disposition.
 */
export function keyName(code: string): string {
    const match = /^(?:Key|Digit)(?<key>[A-Z0-9])$/.exec(code);
    return match?.groups?.["key"] ?? code;
}

/**
 * @description Accélérateur lisible : `Ctrl + Maj + D`.
 */
export function formatAccelerator(accelerator: string): string {
    return accelerator
        .split("+")
        .map(part => MODIFIER_LABELS[part] ?? part)
        .join(" + ");
}

/** Affiche une combinaison en touches. */
function renderPreview(parts: string[]): void {
    if (parts.length === 0) {
        dom.hotkeyPreview.replaceChildren(el("span", { class: "hotkey-placeholder", text: "En attente…" }));
        return;
    }

    dom.hotkeyPreview.replaceChildren(...parts.map(part => el("kbd", { text: MODIFIER_LABELS[part] ?? part })));
}

/** Ouvre la capture : le raccourci global est suspendu le temps de la saisie. */
async function openCapture(): Promise<void> {
    closeOpenSelect();
    capture = { accelerator: null };
    dom.hotkeyMessage.textContent = "";
    renderPreview([]);
    dom.hotkeyDialog.hidden = false;
    playEntrance(dom.hotkeyDialog, "is-entering", 200);
    window.addEventListener("keydown", onCaptureKeydown, true);
    window.addEventListener("keyup", onCaptureKeyup, true);
    // Sans cela, taper le raccourci actuel fermerait l'overlay au lieu d'être capturé.
    await call("pause_shortcut");
}

/** Ferme la capture et rend la main au raccourci enregistré. */
async function closeCapture(): Promise<void> {
    if (!capture) {
        return;
    }

    capture = null;
    window.removeEventListener("keydown", onCaptureKeydown, true);
    window.removeEventListener("keyup", onCaptureKeyup, true);
    dom.hotkeyDialog.hidden = true;
    dom.hotkeyButton.focus();
    await call("resume_shortcut");
}

/** Valide la combinaison saisie et l'applique aussitôt. */
async function confirmCapture(): Promise<void> {
    const accelerator = capture?.accelerator;
    if (!capture) {
        return;
    }

    if (!accelerator) {
        dom.hotkeyMessage.textContent = "Aucune combinaison saisie.";
        return;
    }

    try {
        if (accelerator !== currentSettings().hotkey) {
            await persistSettings({ hotkey: accelerator });
            toast(`Raccourci ${formatAccelerator(accelerator)} enregistré.`);
        }
    }
    finally {
        // Refusé par le système ou accepté, le raccourci en vigueur doit être réarmé.
        await closeCapture();
    }
}

/** Toute frappe est capturée : rien ne doit atteindre les raccourcis de l'overlay. */
function onCaptureKeydown(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();
    if (event.repeat || !capture) {
        return;
    }

    const modifiers = modifiersOf(event);
    if (modifiers.length === 0 && event.code === "Escape") {
        void closeCapture();
        return;
    }

    if (modifiers.length === 0 && (event.code === "Enter" || event.code === "NumpadEnter")) {
        void confirmCapture();
        return;
    }

    if (MODIFIER_CODES.has(event.code)) {
        if (!capture.accelerator) {
            renderPreview(modifiers);
        }

        return;
    }

    if (modifiers.length === 0) {
        dom.hotkeyMessage.textContent = "Ajoute au moins un modificateur : Ctrl, Alt, Maj ou Win.";
        renderPreview([keyName(event.code)]);
        return;
    }

    const parts = [...modifiers, keyName(event.code)];
    capture.accelerator = parts.join("+");
    dom.hotkeyMessage.textContent = "Entrée pour valider, ou tape une autre combinaison.";
    renderPreview(parts);
}

/** Tant qu'aucune combinaison n'est complète, l'aperçu suit les modificateurs tenus. */
function onCaptureKeyup(event: KeyboardEvent): void {
    event.preventDefault();
    event.stopPropagation();
    if (capture && !capture.accelerator) {
        renderPreview(modifiersOf(event));
    }
}

/**
 * @description Branche la capture du raccourci.
 */
export function initHotkeyCapture(): void {
    dom.hotkeyButton.addEventListener("click", openCapture);
    byId("btn-hotkey-cancel", HTMLButtonElement).addEventListener("click", closeCapture);
    byId("btn-hotkey-confirm", HTMLButtonElement).addEventListener("click", confirmCapture);
    dom.hotkeyDialog.addEventListener("mousedown", event => {
        if (event.target === dom.hotkeyDialog) {
            void closeCapture();
        }
    });

    // L'overlay se referme (clic ailleurs) : la capture est abandonnée, faute de quoi
    // le raccourci resterait suspendu et l'overlay injoignable au clavier.
    listen("overlay://hidden", () => void closeCapture());
}
