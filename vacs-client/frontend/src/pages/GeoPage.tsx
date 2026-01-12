import Button from "../components/ui/Button.tsx";
import {
    GeoPageButtonDTO,
    GeoPageContainerDTO,
    GeoPageDividerDTO,
    isGeoPageButton,
    isGeoPageContainer,
    isGeoPageDivider,
} from "../types/profile.ts";
import {CSSProperties} from "preact";
import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";
import {StationId} from "../types/generic.ts";

const page: GeoPageContainerDTO = {
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
};

function GeoPage() {
    const selectedPage = useProfileStore(state => state.selectedPage);

    return selectedPage !== undefined ? (
        <DirectAccessPage data={selectedPage} />
    ) : (
        <GeoPageContainer
            className="w-full h-full [&_div]:min-w-min [&_div]:min-h-min overflow-auto"
            container={page}
        />
    );
}

function GeoPageContainer({
    container,
    className,
}: {
    container: GeoPageContainerDTO;
    className?: string;
}) {
    const style: CSSProperties = {
        height: container.height,
        width: container.width,
        display: "flex",
        flexDirection: container.direction === "row" ? "row" : "column",
        justifyContent: container.justifyContent,
        alignItems: container.alignItems,
        paddingLeft: container.paddingLeft && `${container.paddingLeft}rem`,
        paddingRight: container.paddingRight && `${container.paddingRight}rem`,
        paddingTop: container.paddingTop && `${container.paddingTop}rem`,
        paddingBottom: container.paddingBottom && `${container.paddingBottom}rem`,
        padding: container.padding && `${container.padding}rem`,
        gap: container.gap && `${container.gap}rem`,
    };

    return (
        <div className={className} style={style}>
            {container.children.map((child, index) => {
                if (isGeoPageContainer(child)) {
                    return <GeoPageContainer container={child} key={index} />;
                } else if (isGeoPageDivider(child)) {
                    return <GeoPageDivider divider={child} key={index} />;
                } else if (isGeoPageButton(child)) {
                    return <GeoPageButton button={child} key={index} />;
                }
            })}
        </div>
    );
}

type GeoPageButtonProps = {
    button: GeoPageButtonDTO;
};

function GeoPageButton({button}: GeoPageButtonProps) {
    const setSelectedPage = useProfileStore(state => state.setSelectedPage);

    return (
        <Button
            color="gray"
            className={"aspect-square w-auto! rounded-none! overflow-hidden"}
            style={{height: `${button.size}rem`}}
            onClick={() => setSelectedPage(button.page)}
        >
            {button.label.map((s, index) => (
                <p key={index}>{s}</p>
            ))}
        </Button>
    );
}

type GeoPageDividerProps = {
    divider: GeoPageDividerDTO;
};

// TODO: Make good
function GeoPageDivider({divider}: GeoPageDividerProps) {
    return (
        <div
            className="relative"
            style={{
                height: divider.orientation === "vertical" ? "100%" : `${divider.thickness}px`,
                width: divider.orientation === "horizontal" ? "100%" : `${divider.thickness}px`,
            }}
        >
            <div
                className="absolute -top-2 h-[calc(100%+1rem)] w-full"
                style={{backgroundColor: divider.color}}
            ></div>
        </div>
    );
}

export default GeoPage;
