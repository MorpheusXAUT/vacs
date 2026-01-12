import {ProfileId, StationId} from "./generic.ts";

export type Profile = {
    id: ProfileId;
    geo?: GeoPageContainerDTO;
    tabbed?: Record<string, DirectAccessPageDTO>;
};

export type GeoPageContainerDTO = {
    height?: string; // "<number>['%' | 'em']"
    width?: string; // "<number>['%' | 'em']"
    padding?: number; // in rem
    paddingLeft?: number; // in rem
    paddingRight?: number; // in rem
    paddingTop?: number; // in rem
    paddingBottom?: number; // in rem
    gap?: number; // in rem
    justifyContent?: "start" | "end" | "space-between" | "space-around" | "space-evenly" | "center";
    alignItems?: "start" | "end" | "between" | "around" | "evenly" | "center";
    direction: "row" | "col";
    children: (GeoPageContainerDTO | GeoPageButtonDTO | GeoPageDividerDTO)[];
};

export type GeoPageButtonDTO = {
    label: string[];
    size: number; // size in rem, > 0
    page?: DirectAccessPageDTO;
};

export type GeoPageDividerDTO = {
    orientation: "horizontal" | "vertical";
    thickness: number; // size in px, > 0
    color: string;
};

export type DirectAccessPageDTO = {
    keys: DirectAccessKey[];
    rows: number; // > 0
};

export type DirectAccessKey = {
    label: string[];
    stationId?: StationId;
};

export function isGeoPageContainer(container: unknown): container is GeoPageContainerDTO {
    if (typeof container !== "object" || container === null) {
        return false;
    }

    const maybeContainer = container as Record<string, unknown>;

    return typeof maybeContainer.direction === "string" && Array.isArray(maybeContainer.children);
}

export function isGeoPageDivider(divider: unknown): divider is GeoPageDividerDTO {
    if (typeof divider !== "object" || divider === null) {
        return false;
    }

    const maybeDivider = divider as Record<string, unknown>;

    return (
        typeof maybeDivider.orientation === "string" &&
        typeof maybeDivider.thickness === "number" &&
        (maybeDivider.color === undefined || typeof maybeDivider.color === "string")
    );
}

export function isGeoPageButton(button: unknown): button is GeoPageButtonDTO {
    if (typeof button !== "object" || button === null) {
        return false;
    }

    const maybeButton = button as Record<string, unknown>;

    return (
        Array.isArray(maybeButton.label) &&
        typeof maybeButton.size === "number" &&
        (maybeButton.page === undefined || typeof maybeButton.page === "object")
    );
}
