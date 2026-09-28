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

/* Traitements de texte partagés : recherche tolérante, gabarits d'URL, formats. */

import type { Maybe } from "../types/backend.types.js";

/** Champs d'une fiche utilisés par les gabarits d'URL. */
export interface TemplateSource {
    tenant: string;
    devopsId: string;
    githubId: string;
}

/**
 * @description Supprime accents et casse, pour une recherche tolérante.
 */
export function fold(value: string): string {
    return value
        .normalize("NFD")
        .replace(/[̀-ͯ]/g, "")
        .toLowerCase();
}

/**
 * @description Découpe une recherche en termes normalisés.
 */
export function searchTerms(query: string): string[] {
    return fold(query).split(/\s+/).filter(Boolean);
}

/**
 * @description Encode une valeur d'URL comme le fait le backend, pour l'aperçu du formulaire.
 */
export function encodeSegment(value: string): string {
    return encodeURIComponent(value.trim()).replace(
        /[!'()*]/g,
        character => `%${character.charCodeAt(0).toString(16).toUpperCase()}`,
    );
}

/**
 * @description Applique un gabarit d'URL à une fiche ; `null` si le lien n'est pas calculable.
 */
export function applyTemplate(template: string, source: TemplateSource, required: keyof TemplateSource): Maybe<string> {
    if (!template || !source[required].trim()) {
        return null;
    }

    const url = template
        .replace(/\{tenant\}/g, encodeSegment(source.tenant))
        .replace(/\{devopsId\}/g, encodeSegment(source.devopsId))
        .replace(/\{githubId\}/g, encodeSegment(source.githubId));
    return /^https?:\/\//i.test(url) ? url : null;
}

/**
 * @description Formate une taille en mégaoctets.
 */
export function formatSize(bytes: number): string {
    const megabytes = bytes / (1024 * 1024);
    const formatted = megabytes.toLocaleString("fr-FR", { maximumFractionDigits: 1 });
    return `${formatted} Mo`;
}

/**
 * @description Accorde un nom au pluriel au-delà d'une unité.
 */
export function plural(count: number, word: string): string {
    const suffix = count > 1 ? "s" : "";
    return `${word}${suffix}`;
}
