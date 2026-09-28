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

/* Structures échangées avec le backend Rust. Elles sont générées depuis le code
 * Rust par ts-rs dans `generated/` (`cargo test`) : ce module ne fait que les
 * réexporter, et nommer les quelques formes propres à l'interface. */

import type { Customer } from "./generated/Customer.js";
import type { Flavor } from "./generated/Flavor.js";
import type { Settings } from "./generated/Settings.js";

export type { Bootstrap } from "./generated/Bootstrap.js";
export type { BrowserKind } from "./generated/BrowserKind.js";
export type { BrowserProfile } from "./generated/BrowserProfile.js";
export type { CustomLink } from "./generated/CustomLink.js";
export type { Customer } from "./generated/Customer.js";
export type { CustomerView } from "./generated/CustomerView.js";
export type { DetectedProfile } from "./generated/DetectedProfile.js";
export type { ImportMode } from "./generated/ImportMode.js";
export type { ImportReport } from "./generated/ImportReport.js";
export type { InstalledBrowser } from "./generated/InstalledBrowser.js";
export type { LinkKind } from "./generated/LinkKind.js";
export type { Settings } from "./generated/Settings.js";
export type { UpdateInfo } from "./generated/UpdateInfo.js";
export type { UpdateProgress } from "./generated/UpdateProgress.js";
export type { Workspace } from "./generated/Workspace.js";

/** Valeur éventuellement absente. */
export type Maybe<T> = T | null;

/** Apparence de l'interface. */
export type ThemeMode = Settings["theme"];

/** Livrable en cours d'exécution, qui détermine celui à télécharger. */
export type UpdateFlavor = Flavor;

/** Fiche saisie dans le formulaire : l'identifiant est attribué par le backend. */
export type CustomerInput = Omit<Customer, "id">;
