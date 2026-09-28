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

/* Constantes de l'interface. */

import type { BrowserKind, LinkKind, ThemeMode } from "../types/backend.types.js";
import { iconAdmin, iconDevops, iconGithub } from "./icons.helper.js";

/** Nombre maximal de fiches rendues simultanément, pour garder un affichage instantané. */
export const RENDER_LIMIT = 200;

/** Hauteur maximale d'un menu déroulant, bornée en plus par la place disponible. */
export const MENU_MAX_HEIGHT = 500;

/** Jetons reconnus dans les gabarits d'URL. */
export const TEMPLATE_TOKENS: readonly string[] = ["tenant", "devopsId", "githubId"];

/** Un lien calculé d'une fiche, dans l'ordre d'affichage des boutons. */
export interface ComputedLink {
    readonly kind: Exclude<LinkKind, "folder">;
    readonly label: string;
    readonly urlField: "devopsUrl" | "githubUrl" | "adminCenterUrl";
    readonly icon: () => string;
}

/** Liens calculés, dans l'ordre d'affichage des boutons d'une fiche. */
export const COMPUTED_LINKS: readonly ComputedLink[] = [
    { kind: "devops", label: "DevOps", urlField: "devopsUrl", icon: iconDevops },
    { kind: "github", label: "GitHub", urlField: "githubUrl", icon: iconGithub },
    { kind: "adminCenter", label: "Admin", urlField: "adminCenterUrl", icon: iconAdmin },
];

/** Description d'un navigateur pris en charge. */
export interface BrowserDescriptor {
    readonly kind: BrowserKind;
    readonly label: string;
    /** Vrai si le navigateur sait ouvrir un lien dans un profil désigné. */
    readonly profiles: boolean;
    /** Fichier du logo, dans `icons/browsers/`. */
    readonly logo: string;
}

/** Navigateurs pris en charge, dans l'ordre de présentation. Les logos sont lus dans
 * `icons/browsers/` : il suffit de remplacer ces fichiers pour les changer. */
export const BROWSERS: readonly BrowserDescriptor[] = [
    { kind: "edge", label: "Microsoft Edge", profiles: true, logo: "edge.png" },
    { kind: "chrome", label: "Google Chrome", profiles: true, logo: "chrome.png" },
    { kind: "firefox", label: "Mozilla Firefox", profiles: true, logo: "firefox.png" },
    { kind: "firefoxDev", label: "Firefox Developer Edition", profiles: true, logo: "firefox-dev.png" },
    { kind: "brave", label: "Brave", profiles: true, logo: "brave.png" },
    { kind: "vivaldi", label: "Vivaldi", profiles: true, logo: "vivaldi.png" },
    { kind: "opera", label: "Opera", profiles: false, logo: "opera.png" },
    { kind: "system", label: "Navigateur par défaut", profiles: false, logo: "system.svg" },
];

/** Une apparence proposée dans les réglages. */
export interface ThemeChoice {
    readonly value: ThemeMode;
    readonly main: string;
    readonly sub: string;
}

/** Apparences proposées dans les réglages. */
export const THEMES: readonly ThemeChoice[] = [
    { value: "system", main: "Système", sub: "suit le thème de Windows" },
    { value: "light", main: "Clair", sub: "" },
    { value: "dark", main: "Sombre", sub: "" },
];
