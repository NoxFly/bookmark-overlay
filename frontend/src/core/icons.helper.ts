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

/* Icônes SVG en ligne, sous forme de chaînes constantes : jamais construites à
 * partir d'une donnée, elles peuvent être injectées en `innerHTML`. */

/**
 * @description Marque Azure DevOps redessinée sur notre grille : deux rubans imbriqués,
 * tracés pleins comme l'original — à 13 px, un contour se refermerait sur lui-même.
 */
export function iconDevops(): string {
    return (
        '<svg viewBox="0 0 24 24">'
        + '<path fill="currentColor" stroke="none" d="M10.4 1.4 18.2 5.6 4.5 8.9v7.7l-3.1-1.5V8.6l2.2-2.7 6.8-1.9z"/>'
        + '<path fill="currentColor" stroke="none" d="M22.6 5.2v13.3l-5.3 4.2-8.2-3v3l-4.6-6 13.7 1.1V5.7z"/>'
        + "</svg>"
    );
}

/** @description Icône GitHub. */
export function iconGithub(): string {
    return '<svg viewBox="0 0 24 24"><path d="M9 19c-4 1.2-4-2.2-5.5-2.8M15 21v-3.3c0-1 .1-1.4-.5-2 2.4-.3 4.5-1.2 4.5-5.1a4 4 0 0 0-1.1-2.7 3.7 3.7 0 0 0-.1-2.8s-.9-.3-3 1.1a10.2 10.2 0 0 0-5.4 0C7.3 4 6.4 4.3 6.4 4.3a3.7 3.7 0 0 0-.1 2.8A4 4 0 0 0 5.2 9.8c0 3.9 2.1 4.8 4.5 5.1-.5.5-.5 1-.5 1.7V21"/></svg>';
}

/** @description Icône de l'Admin Center. */
export function iconAdmin(): string {
    return '<svg viewBox="0 0 24 24"><path d="M12 3l7.5 3v5.3c0 4.3-3 7.7-7.5 9.2-4.5-1.5-7.5-4.9-7.5-9.2V6z"/><path d="M9.3 12.2l1.9 1.9 3.6-3.8"/></svg>';
}

/** @description Icône de dossier. */
export function iconFolder(): string {
    return '<svg viewBox="0 0 24 24"><path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4l2 2.4h7A1.5 1.5 0 0 1 19 9.9v7.6a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 3 17.5z"/></svg>';
}

/** @description Icône de modification. */
export function iconEdit(): string {
    return '<svg viewBox="0 0 24 24"><path d="M4 20h4l10.5-10.5a2.1 2.1 0 0 0-3-3L5 17v3z"/></svg>';
}

/** @description Icône de suppression. */
export function iconTrash(): string {
    return '<svg viewBox="0 0 24 24"><path d="M4 7h16M10 7V5h4v2M6 7l1 12h10l1-12"/></svg>';
}

/** @description Icône de code, pour les espaces de travail. */
export function iconCode(): string {
    return '<svg viewBox="0 0 24 24"><path d="M9 7l-5 5 5 5M15 7l5 5-5 5"/></svg>';
}

/** @description Icône de lien. */
export function iconLink(): string {
    return '<svg viewBox="0 0 24 24"><path d="M10.3 13.7a4 4 0 0 0 5.7 0l2.8-2.8a4 4 0 0 0-5.7-5.7l-1.6 1.6"/><path d="M13.7 10.3a4 4 0 0 0-5.7 0l-2.8 2.8a4 4 0 0 0 5.7 5.7l1.6-1.6"/></svg>';
}

/** @description Flèche vers le haut. */
export function iconUp(): string {
    return '<svg viewBox="0 0 24 24"><path d="M7 14l5-5 5 5"/></svg>';
}

/** @description Flèche vers le bas. */
export function iconDown(): string {
    return '<svg viewBox="0 0 24 24"><path d="M7 10l5 5 5-5"/></svg>';
}

/** @description Chevron d'une liste déroulante. */
export function iconCaret(): string {
    return '<svg viewBox="0 0 24 24"><path d="M7 10l5 5 5-5"/></svg>';
}

/** @description Coche de sélection. */
export function iconCheck(): string {
    return '<svg viewBox="0 0 24 24"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>';
}
