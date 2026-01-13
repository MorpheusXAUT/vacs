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

export const useProfileType = (): "geo" | "tabbed" | "unknown" | undefined => {
    return useProfileStore(state => {
        if (state.profile === undefined) return undefined;
        if (state.profile.geo !== undefined) return "geo";
        if (state.profile.tabbed !== undefined) return "tabbed";
        return "unknown";
    });
};

export const INVALID_PROFILE: Profile = {
    id: "INVALID" as ProfileId,
};

export const TABBED_PROFILE: Profile = {
    id: "TABBED" as ProfileId,
    tabbed: {
        "DA 1": {} as DirectAccessPage,
        "DA 2": {} as DirectAccessPage,
    },
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
                                    {
                                        label: ["B", "LOWS"],
                                        size: 9,
                                        page: {
                                            keys: [
                                                {
                                                    label: ["380", "B6", "PLC"],
                                                    stationId: "LOVV_B6" as StationId,
                                                },
                                                {
                                                    label: ["370", "B5", "PLC"],
                                                    stationId: "LOVV_B5" as StationId,
                                                },
                                                {
                                                    label: ["360", "B4", "PLC"],
                                                    stationId: "LOVV_B4" as StationId,
                                                },
                                                {
                                                    label: ["350", "B3", "PLC"],
                                                    stationId: "LOVV_B3" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "B2", "PLC"],
                                                    stationId: "LOVV_B2" as StationId,
                                                },
                                                {
                                                    label: ["310-", "B1", "PLC"],
                                                    stationId: "LOVV_B1" as StationId,
                                                },
                                                {
                                                    label: ["390+", "B7", "PLC"],
                                                    stationId: "LOVV_B7" as StationId,
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
                                                    label: ["LOWS", "APP", "PLC"],
                                                    stationId: "LOWS_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWS", "TWR", "PLC"],
                                                    stationId: "LOWS_TWR" as StationId,
                                                },
                                                {
                                                    label: ["LOWS", "DEL", "PLC"],
                                                    stationId: "LOWS_DEL" as StationId,
                                                },
                                            ],
                                            rows: 6,
                                        },
                                    },
                                    {
                                        label: ["N", "LOWL"],
                                        size: 9,
                                        page: {
                                            keys: [
                                                {
                                                    label: ["380", "N6", "PLC"],
                                                    stationId: "LOVV_N6" as StationId,
                                                },
                                                {
                                                    label: ["370", "N5", "PLC"],
                                                    stationId: "LOVV_N5" as StationId,
                                                },
                                                {
                                                    label: ["360", "N4", "PLC"],
                                                    stationId: "LOVV_N4" as StationId,
                                                },
                                                {
                                                    label: ["350", "N3", "PLC"],
                                                    stationId: "LOVV_N3" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "N2", "PLC"],
                                                    stationId: "LOVV_N2" as StationId,
                                                },
                                                {
                                                    label: ["310-", "N1", "PLC"],
                                                    stationId: "LOVV_N1" as StationId,
                                                },
                                                {
                                                    label: ["390+", "N7", "PLC"],
                                                    stationId: "LOVV_N7" as StationId,
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
                                    {
                                        label: ["E", "APP"],
                                        size: 9,
                                        page: {
                                            keys: [
                                                {
                                                    label: ["380", "E6", "PLC"],
                                                    stationId: "LOVV_E6" as StationId,
                                                },
                                                {
                                                    label: ["370", "E5", "PLC"],
                                                    stationId: "LOVV_E5" as StationId,
                                                },
                                                {
                                                    label: ["360", "E4", "PLC"],
                                                    stationId: "LOVV_E4" as StationId,
                                                },
                                                {
                                                    label: ["350", "E3", "PLC"],
                                                    stationId: "LOVV_E3" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "E2", "PLC"],
                                                    stationId: "LOVV_E2" as StationId,
                                                },
                                                {
                                                    label: ["310-", "E1", "PLC"],
                                                    stationId: "LOVV_E1" as StationId,
                                                },
                                                {
                                                    label: ["390+", "E7", "PLC"],
                                                    stationId: "LOVV_E7" as StationId,
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
                                                    label: ["APP", "VB", "PLN"],
                                                    stationId: "LOWW_APP" as StationId,
                                                },
                                                {
                                                    label: ["APP", "VN", "PLN"],
                                                    stationId: "LOWW_N_APP" as StationId,
                                                },
                                                {
                                                    label: ["APP", "VM", "PLN"],
                                                    stationId: "LOWW_M_APP" as StationId,
                                                },
                                                {
                                                    label: ["APP", "VP", "PLN"],
                                                    stationId: "LOWW_P_APP" as StationId,
                                                },
                                                {
                                                    label: ["APP", "VD1", "PLN"],
                                                    stationId: "LOWW_F_APP" as StationId,
                                                },
                                                {
                                                    label: ["APP", "VD2", "PLN"],
                                                    stationId: "LOWW_D_APP" as StationId,
                                                },
                                                {
                                                    label: [],
                                                    stationId: undefined,
                                                },
                                                {
                                                    label: ["LOWW", "TWR", "PLN"],
                                                    stationId: "LOWW_TWR" as StationId,
                                                },
                                                {
                                                    label: ["LOWW", "E TWR", "PLN"],
                                                    stationId: "LOWW_E_TWR" as StationId,
                                                },
                                                {
                                                    label: ["LOWW", "GND", "PLN"],
                                                    stationId: "LOWW_GMD" as StationId,
                                                },
                                                {
                                                    label: ["LOWW", "W GND", "PLN"],
                                                    stationId: "LOWW_W_GMD" as StationId,
                                                },
                                                {
                                                    label: ["LOWW", "DEL", "PLN"],
                                                    stationId: "LOWW_DEL" as StationId,
                                                },
                                            ],
                                            rows: 6,
                                        },
                                    },
                                ],
                            },
                            {
                                direction: "row",
                                gap: 0.5,
                                children: [
                                    {
                                        label: ["W", "WI_WK"],
                                        size: 9,
                                        page: {
                                            keys: [
                                                {
                                                    label: ["380", "W6", "PLC"],
                                                    stationId: "LOVV_W6" as StationId,
                                                },
                                                {
                                                    label: ["370", "W5", "PLC"],
                                                    stationId: "LOVV_W5" as StationId,
                                                },
                                                {
                                                    label: ["360", "W4", "PLC"],
                                                    stationId: "LOVV_W4" as StationId,
                                                },
                                                {
                                                    label: ["350", "W3", "PLC"],
                                                    stationId: "LOVV_W3" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "W2", "PLC"],
                                                    stationId: "LOVV_W2" as StationId,
                                                },
                                                {
                                                    label: ["310-", "W1", "PLC"],
                                                    stationId: "LOVV_W1" as StationId,
                                                },
                                                {
                                                    label: ["390+", "W7", "PLC"],
                                                    stationId: "LOVV_W7" as StationId,
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
                                                    label: ["LOWK", "APP", "PLC"],
                                                    stationId: "LOWK_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWK", "TWR", "PLC"],
                                                    stationId: "LOWK_TWR" as StationId,
                                                },
                                                {
                                                    label: ["LOWI", "APP", "PLC"],
                                                    stationId: "LOWI_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWI", "DIR", "PLC"],
                                                    stationId: "LOWI_F_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWI", "E APP", "PLC"],
                                                    stationId: "LOWI_E_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWI", "S APP", "PLC"],
                                                    stationId: "LOWI_S_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWI", "TWR", "PLC"],
                                                    stationId: "LOWI_TWR" as StationId,
                                                },
                                                {
                                                    label: ["LOWI", "DEL", "PLC"],
                                                    stationId: "LOWI_DEL" as StationId,
                                                },
                                            ],
                                            rows: 6,
                                        },
                                    },
                                    {
                                        label: ["S", "LOWG"],
                                        size: 9,
                                        page: {
                                            keys: [
                                                {
                                                    label: ["380", "S6", "PLC"],
                                                    stationId: "LOVV_S6" as StationId,
                                                },
                                                {
                                                    label: ["370", "S5", "PLC"],
                                                    stationId: "LOVV_S5" as StationId,
                                                },
                                                {
                                                    label: ["360", "S4", "PLC"],
                                                    stationId: "LOVV_S4" as StationId,
                                                },
                                                {
                                                    label: ["350", "S3", "PLC"],
                                                    stationId: "LOVV_S3" as StationId,
                                                },
                                                {
                                                    label: ["320-340", "S2", "PLC"],
                                                    stationId: "LOVV_S2" as StationId,
                                                },
                                                {
                                                    label: ["310-", "S1", "PLC"],
                                                    stationId: "LOVV_S1" as StationId,
                                                },
                                                {
                                                    label: ["390+", "S7", "PLC"],
                                                    stationId: "LOVV_S7" as StationId,
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
                                                    label: ["LOWG", "APP", "PLC"],
                                                    stationId: "LOWG_APP" as StationId,
                                                },
                                                {
                                                    label: ["LOWG", "TWR", "PLC"],
                                                    stationId: "LOWG_TWR" as StationId,
                                                },
                                            ],
                                            rows: 6,
                                        },
                                    },
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
                    {orientation: "vertical", thickness: 2, color: "#364153", oversize: 0.5},
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
