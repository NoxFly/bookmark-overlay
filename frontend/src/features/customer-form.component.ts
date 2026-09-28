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

/* Formulaire de création et de modification d'une fiche. */

import type { CustomerInput, CustomerView, CustomLink, Maybe, Workspace } from "../types/backend.types.js";
import { toast } from "../components/toast.component.js";
import { dom } from "../core/dom-refs.js";
import { byId, closestTarget, el, field, query } from "../core/dom.helper.js";
import { iconCode, iconFolder, iconTrash } from "../core/icons.helper.js";
import { call } from "../core/ipc.service.js";
import { state } from "../core/state.js";
import { applyTemplate, type TemplateSource } from "../core/text.helper.js";
import { applyCustomers } from "./customer-list.component.js";
import { showView } from "./views.service.js";

/** Champs simples du formulaire, par nom. */
const FIELD_NAMES = ["name", "tenant", "keywords", "folderPath", "devopsId", "githubId"] as const;

type FieldName = (typeof FIELD_NAMES)[number];

/** Champ du formulaire de fiche. */
function input(name: FieldName): HTMLInputElement {
    return field(dom.form, name);
}

/**
 * @description Prépare le formulaire pour une création (`null`) ou une modification.
 */
export function openForm(customer: Maybe<CustomerView>): void {
    state.editingId = customer ? customer.id : null;
    input("name").value = customer?.name ?? "";
    input("tenant").value = customer?.tenant ?? "";
    input("keywords").value = customer ? customer.keywords.join(", ") : "";
    input("folderPath").value = customer?.folderPath ?? "";
    input("devopsId").value = customer?.devopsId ?? "";
    input("githubId").value = customer?.githubId ?? "";

    dom.workspaces.replaceChildren();
    for (const workspace of customer?.workspaces ?? []) {
        addWorkspaceRow(workspace);
    }

    dom.links.replaceChildren();
    for (const link of customer?.links ?? []) {
        addLinkRow(link);
    }

    renderFormPreview();
    showView("form");
    input("name").focus();
    input("name").select();
}

/** Bouton d'outil d'une ligne du formulaire. */
function rowButton(icon: string, action: string, label: string): HTMLButtonElement {
    return el("button", {
        class: "tool-btn",
        html: icon,
        attrs: { type: "button", "data-ws": action, title: label, "aria-label": label },
    });
}

/** Champ texte d'une ligne du formulaire. */
function rowInput(className: string, placeholder: string, value: string): HTMLInputElement {
    const node = el("input", { class: className, attrs: { type: "text", maxlength: 512, placeholder } });
    node.value = value;
    return node;
}

/** Ajoute une ligne « nom + dossier ». */
function addWorkspaceRow(workspace: Maybe<Workspace>): HTMLDivElement {
    const row = el("div", { class: "workspace-row" }, [
        rowInput("ws-name", "Extension", workspace?.name ?? ""),
        rowInput("ws-path", "dossier, ou fichier .code-workspace", workspace?.path ?? ""),
        rowButton(iconFolder(), "folder", "Choisir un dossier"),
        rowButton(iconCode(), "file", "Choisir un fichier .code-workspace"),
        rowButton(iconTrash(), "remove", "Retirer"),
    ]);
    dom.workspaces.appendChild(row);
    return row;
}

/** Ajoute une ligne « nom + URL ». */
function addLinkRow(link: Maybe<CustomLink>): HTMLDivElement {
    const row = el("div", { class: "workspace-row" }, [
        rowInput("ws-name", "Extranet", link?.name ?? ""),
        rowInput("ws-path", "https://…", link?.url ?? ""),
        rowButton(iconTrash(), "remove", "Retirer"),
    ]);
    dom.links.appendChild(row);
    return row;
}

/** Relit les paires nom + valeur d'une liste de lignes, en ignorant celles restées vides. */
function collectRows(container: HTMLElement): { name: string; value: string }[] {
    return Array.from(container.querySelectorAll(".workspace-row"))
        .map(row => ({
            name: query(row, ".ws-name", HTMLInputElement).value.trim(),
            value: query(row, ".ws-path", HTMLInputElement).value.trim(),
        }))
        .filter(entry => entry.name || entry.value);
}

/** Relit la saisie complète. */
function collectCustomer(): CustomerInput {
    return {
        tenant: input("tenant").value,
        name: input("name").value,
        keywords: input("keywords")
            .value.split(",")
            .map(keyword => keyword.trim())
            .filter(Boolean),
        folderPath: input("folderPath").value,
        devopsId: input("devopsId").value,
        githubId: input("githubId").value,
        workspaces: collectRows(dom.workspaces).map(entry => ({ name: entry.name, path: entry.value })),
        links: collectRows(dom.links).map(entry => ({ name: entry.name, url: entry.value })),
    };
}

/**
 * @description Affiche en direct les URL que produira la fiche en cours d'édition.
 */
export function renderFormPreview(): void {
    const settings = state.settings;
    if (!settings) {
        return;
    }

    const draft: TemplateSource = {
        tenant: input("tenant").value,
        devopsId: input("devopsId").value,
        githubId: input("githubId").value,
    };
    const lines: [string, Maybe<string>][] = [
        ["DevOps", applyTemplate(settings.devopsUrlTemplate, draft, "devopsId")],
        ["GitHub", applyTemplate(settings.githubUrlTemplate, draft, "githubId")],
        ["Admin Center", applyTemplate(settings.adminCenterUrlTemplate, draft, "tenant")],
    ];
    dom.formPreview.replaceChildren(
        ...lines.map(([label, url]) => {
            const className = url ? "preview-line" : "preview-line muted";
            return el("div", { class: className }, [
                el("b", { text: label }),
                el("span", { text: url ?? "identifiant manquant" }),
            ]);
        }),
    );
}

/** Enregistre la fiche et revient à la liste. */
async function submitForm(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const customer = collectCustomer();
    const editingId = state.editingId;
    const customers = editingId
        ? await call("update_customer", { id: editingId, customer })
        : await call("create_customer", { customer });

    applyCustomers(customers);
    toast(editingId ? "Fiche mise à jour." : "Fiche créée.");
    state.editingId = null;
    showView("list");
}

/**
 * @description Branche le formulaire : aperçu, lignes dynamiques, sélecteurs, enregistrement.
 */
export function initCustomerForm(): void {
    for (const name of ["tenant", "devopsId", "githubId"] as const) {
        input(name).addEventListener("input", renderFormPreview);
    }

    byId("btn-add-link", HTMLButtonElement).addEventListener("click", () => {
        query(addLinkRow(null), ".ws-name", HTMLInputElement).focus();
    });

    byId("btn-add-workspace", HTMLButtonElement).addEventListener("click", () => {
        query(addWorkspaceRow(null), ".ws-name", HTMLInputElement).focus();
    });

    dom.links.addEventListener("click", event => {
        closestTarget(event, '[data-ws="remove"]')?.closest(".workspace-row")?.remove();
    });

    dom.workspaces.addEventListener("click", async event => {
        const button = closestTarget(event, "[data-ws]");
        const row = button?.closest(".workspace-row");
        if (!button || !row) {
            return;
        }

        if (button.dataset["ws"] === "remove") {
            row.remove();
            return;
        }

        const selected = button.dataset["ws"] === "file" ? await call("pick_workspace_file") : await call("pick_folder");
        if (selected) {
            query(row, ".ws-path", HTMLInputElement).value = selected;
        }
    });

    byId("btn-browse", HTMLButtonElement).addEventListener("click", async () => {
        const selected = await call("pick_folder");
        if (selected) {
            input("folderPath").value = selected;
        }
    });

    for (const button of document.querySelectorAll("[data-back]")) {
        button.addEventListener("click", () => showView("list"));
    }

    dom.form.addEventListener("submit", submitForm);
}
