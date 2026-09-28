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

/* Liste déroulante maison : le `<select>` natif de la webview ouvre un menu dessiné
 * par le système, ni thémable ni accordé au reste de l'overlay. */

import type { Maybe } from "../types/backend.types.js";
import { MENU_MAX_HEIGHT } from "../core/constants.js";
import { closestTarget, el, playEntrance } from "../core/dom.helper.js";
import { iconCaret, iconCheck } from "../core/icons.helper.js";

/** Une entrée de liste. `icon` fabrique le nœud de son icône. */
export interface SelectOption<T extends string> {
    value: T;
    main: string;
    sub?: string;
    icon?: () => Node;
}

/** Pilotage d'une liste déroulante. */
export interface SelectController<T extends string> {
    /** Remplace les entrées et la valeur sélectionnée. */
    setOptions(options: SelectOption<T>[], value: T): void;
    /** Sélectionne une valeur, en la créant à la volée si elle est inconnue. */
    setValue(value: T): void;
    /** Grise la liste quand elle n'offre aucun choix réel. */
    setDisabled(disabled: boolean): void;
    getValue(): Maybe<T>;
    close(): void;
}

/** Liste ouverte, s'il y en a une : une seule à la fois. */
let openSelect: Maybe<{ close(): void }> = null;

/**
 * @description Construit une liste déroulante thémable dans `container`.
 *
 * Le menu flotte par-dessus le contenu, sous le champ, ou au-dessus quand la place
 * manque en bas. Il est positionné en absolu dans le formulaire qui défile, et non
 * en fixe sur la fenêtre : il suit ainsi le défilement sans être recalculé. Sa
 * hauteur est bornée par la place visible de ce formulaire, ce qui le garde à
 * l'abri du rognage.
 */
export function createSelect<T extends string>(
    container: HTMLElement,
    onChange: (value: T) => void,
): SelectController<T> {
    const iconSlot = el("span", { class: "select-icon", attrs: { hidden: true } });
    const main = el("span", { class: "select-main" });
    const sub = el("span", { class: "select-sub" });
    const trigger = el(
        "button",
        {
            class: "select-trigger",
            attrs: {
                type: "button",
                "aria-haspopup": "listbox",
                "aria-expanded": "false",
                "aria-labelledby": container.dataset["labelledBy"] ?? null,
            },
        },
        [
            iconSlot,
            el("span", { class: "select-texts" }, [main, sub]),
            el("span", { class: "select-caret", html: iconCaret() }),
        ],
    );
    const menu = el("div", { class: "select-menu", attrs: { role: "listbox", hidden: true } });
    container.replaceChildren(trigger, menu);

    let options: SelectOption<T>[] = [];
    let value: Maybe<T> = null;
    let highlighted = 0;

    const renderOptions = (): void => {
        menu.replaceChildren(...options.map((option, index) => renderOption(option, index === highlighted, option.value === value)));
    };

    /** Place le menu sous le champ, ou au-dessus s'il y tient mieux. */
    const placeMenu = (): void => {
        const scroller = container.closest(".form");
        const bounds = scroller ? scroller.getBoundingClientRect() : { top: 0, bottom: window.innerHeight };
        const anchor = trigger.getBoundingClientRect();
        const gap = 10;
        const below = Math.min(bounds.bottom, window.innerHeight) - anchor.bottom - gap;
        const above = anchor.top - Math.max(bounds.top, 0) - gap;

        menu.style.maxHeight = "";
        const natural = Math.min(menu.scrollHeight, MENU_MAX_HEIGHT);
        const opensUp = natural > below && above > below;
        const room = opensUp ? above : below;
        const height = Math.max(96, Math.min(MENU_MAX_HEIGHT, room));
        menu.style.maxHeight = `${height}px`;
        container.classList.toggle("opens-up", opensUp);
    };

    const close = (): void => {
        if (menu.hidden) {
            return;
        }

        menu.hidden = true;
        container.classList.remove("is-open", "opens-up");
        trigger.setAttribute("aria-expanded", "false");
        if (openSelect === controller) {
            openSelect = null;
        }
    };

    const open = (): void => {
        if (trigger.disabled) {
            return;
        }

        if (openSelect && openSelect !== controller) {
            openSelect.close();
        }

        highlighted = Math.max(0, options.findIndex(option => option.value === value));
        renderOptions();
        trigger.scrollIntoView({ block: "nearest" });
        menu.hidden = false;
        placeMenu();
        playEntrance(menu, "is-entering", 150);
        container.classList.add("is-open");
        trigger.setAttribute("aria-expanded", "true");
        openSelect = controller;
        menu.querySelector(".is-highlighted")?.scrollIntoView({ block: "nearest" });
    };

    const commit = (index: number): void => {
        const option = options[index];
        if (!option) {
            return;
        }

        close();
        if (option.value === value) {
            return;
        }

        controller.setValue(option.value);
        onChange(option.value);
    };

    const move = (delta: number): void => {
        highlighted = (highlighted + delta + options.length) % options.length;
        renderOptions();
        menu.querySelector(".is-highlighted")?.scrollIntoView({ block: "nearest" });
    };

    const controller: SelectController<T> = {
        setOptions(nextOptions, nextValue) {
            options = nextOptions;
            controller.setValue(nextValue);
        },
        setValue(nextValue) {
            value = nextValue;
            if (!options.some(option => option.value === nextValue)) {
                options = [{ value: nextValue, main: nextValue, sub: "inconnu sur cette machine" }, ...options];
            }

            const current = options.find(option => option.value === nextValue);
            main.textContent = current ? current.main : "—";
            sub.textContent = current?.sub ?? "";
            sub.hidden = !current?.sub;
            const icon = current?.icon;
            iconSlot.replaceChildren(...(icon ? [icon()] : []));
            iconSlot.hidden = !icon;
            if (!menu.hidden) {
                renderOptions();
            }
        },
        setDisabled(disabled) {
            trigger.disabled = disabled;
            if (disabled) {
                close();
            }
        },
        getValue: () => value,
        close,
    };

    trigger.addEventListener("click", () => {
        if (menu.hidden) {
            open();
        }
        else {
            close();
        }
    });

    trigger.addEventListener("keydown", event => {
        if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === " ") {
            event.preventDefault();
            if (menu.hidden) {
                open();
            }
            else {
                const delta = event.key === "ArrowUp" ? -1 : 1;
                move(delta);
            }
        }
        else if (event.key === "Enter" && !menu.hidden) {
            event.preventDefault();
            commit(highlighted);
        }
    });

    menu.addEventListener("mousedown", event => {
        const option = closestTarget(event, ".select-option");
        if (!option) {
            return;
        }

        event.preventDefault();
        const index = options.findIndex(entry => entry.value === option.dataset["value"]);
        commit(index);
        trigger.focus();
    });

    return controller;
}

/** Nœud d'une entrée du menu. */
function renderOption<T extends string>(option: SelectOption<T>, highlighted: boolean, selected: boolean): HTMLButtonElement {
    const className = highlighted ? "select-option is-highlighted" : "select-option";
    const icon = option.icon;
    const texts = [el("span", { class: "select-main", text: option.main })];
    if (option.sub) {
        texts.push(el("span", { class: "select-sub", text: option.sub }));
    }

    return el(
        "button",
        {
            class: className,
            attrs: { type: "button", role: "option", "aria-selected": String(selected), "data-value": option.value },
        },
        [
            ...(icon ? [el("span", { class: "select-icon" }, [icon()])] : []),
            el("span", { class: "select-texts" }, texts),
            el("span", { class: "select-check", html: iconCheck() }),
        ],
    );
}

/**
 * @description Ferme la liste ouverte, s'il y en a une ; vrai si c'était le cas.
 */
export function closeOpenSelect(): boolean {
    if (!openSelect) {
        return false;
    }

    openSelect.close();
    return true;
}

/**
 * @description Un clic ailleurs que dans une liste ferme celle qui est ouverte.
 */
export function initSelects(): void {
    document.addEventListener("mousedown", event => {
        if (openSelect && !closestTarget(event, ".select-menu, .select-trigger")) {
            closeOpenSelect();
        }
    });
}
