import {DirectAccessPage, Profile} from "../types/profile.ts";
import {create} from "zustand/react";
import {ProfileId, StationId} from "../types/generic.ts";

type ProfileState = {
    profile: Profile | undefined;
    page: DirectAccessPage | undefined;
    setProfile: (profile: Profile | undefined) => void;
    setPage: (page: DirectAccessPage | undefined) => void;
};

export const useProfileStore = create<ProfileState>()(set => ({
    profile: undefined,
    page: undefined,
    setProfile: profile => set({profile}),
    setPage: page => set({page: page}),
}));

export const INVALID_PROFILE: Profile = {
    id: "INVALID" as ProfileId,
};

export const TABBED_PROFILE: Profile = {
    id: "TABBED" as ProfileId,
    tabbed: {},
};

export const LOVV_PROFILE: Profile = {
    id: "LOVV" as ProfileId,
    geo: {
        direction: "row",
        padding: 0.5,
        gap: 1.5,
        justifyContent: "space-between",
        children: [
            {
                direction: "col",
                height: "100%",
                justifyContent: "space-between",
                gap: 0.75,
                children: [
                    {label: ["KAR_MUN", "N"], size: 6.25},
                    {label: ["KAR_MUN", "S"], size: 6.25},
                    {label: ["ZUR"], size: 6.25},
                ],
            },
            {
                direction: "col",
                height: "100%",
                width: "100%",
                justifyContent: "space-between",
                gap: 0.75,
                children: [
                    {
                        direction: "row",
                        justifyContent: "space-between",
                        children: [
                            {label: ["FIC"], size: 6.25},
                            {label: ["Cont"], size: 6.25},
                            {label: ["FDU"], size: 6.25},
                        ],
                    },
                    {
                        direction: "col",
                        alignItems: "center",
                        gap: 0.25,
                        children: [
                            {
                                direction: "row",
                                gap: 0.5,
                                children: [
                                    {label: ["B", "LOWS"], size: 9},
                                    {
                                        label: ["N", "LOWL"],
                                        size: 9,
                                        page: {
                                            keys: [
                                                {
                                                    label: ["380", "N6", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["370", "N5", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["360", "N4", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["350", "N3", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "N2", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["310-", "N1", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["390+", "N7", "EC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: ["380", "N6", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["370", "N5", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["360", "N4", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["350", "N3", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "N2", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["310-", "N1", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: ["390+", "N7", "PLC"],
                                                    stationId: "N6" as StationId,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: ["LOWL", "APP", "PLC"],
                                                    stationId: "LOWL_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWL", "TWR", "PLC"],
                                                    stationId: "LOWL_TWR" as StationId,
                                                },
                                            ],
                                            rows: 6,
                                        },
                                    },
                                    {label: ["E", "APP"], size: 9},
                                ],
                            },
                            {
                                direction: "row",
                                gap: 0.5,
                                children: [
                                    {label: ["W", "WI_WK"], size: 9},
                                    {label: ["S", "LOWG"], size: 9},
                                ],
                            },
                        ],
                    },
                    {
                        direction: "row",
                        justifyContent: "space-between",
                        paddingLeft: 3,
                        paddingRight: 3,
                        children: [
                            {label: ["PAD"], size: 6.25},
                            {label: ["LJU"], size: 6.25},
                        ],
                    },
                ],
            },
            {
                direction: "row",
                height: "100%",
                gap: 1,
                children: [
                    {
                        direction: "col",
                        height: "100%",
                        justifyContent: "space-between",
                        gap: 0.75,
                        children: [
                            {label: ["PRA"], size: 6.25},
                            {label: ["BRA"], size: 6.25},
                            {label: ["BUD"], size: 6.25},
                            {label: ["ZAG"], size: 6.25},
                        ],
                    },
                    {orientation: "vertical", thickness: 2, color: "#364153"},
                    {
                        direction: "col",
                        height: "100%",
                        gap: 0.75,
                        justifyContent: "space-between",
                        children: [
                            {label: ["MIL"], size: 6.25},
                            {label: ["FMP"], size: 6.25},
                            {label: ["SUP"], size: 6.25},
                            {label: ["CWP"], size: 6.25},
                        ],
                    },
                ],
            },
        ],
    },
};
