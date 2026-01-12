import Button from "./ui/Button.tsx";
import {
    DirectAccessPageDTO,
    GeoPageButtonDTO,
    GeoPageContainerDTO,
    GeoPageDividerDTO,
    isGeoPageButton,
    isGeoPageContainer,
    isGeoPageDivider,
} from "../types/profile.ts";
import {CSSProperties} from "preact";

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
                {label: ["KAR_MUN", "N"], size: 6.25, page: {} as DirectAccessPageDTO},
                {label: ["KAR_MUN", "S"], size: 6.25, page: {} as DirectAccessPageDTO},
                {label: ["ZUR"], size: 6.25, page: {} as DirectAccessPageDTO},
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
                        {label: ["FIC"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["Cont"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["FDU"], size: 6.25, page: {} as DirectAccessPageDTO},
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
                                {label: ["B", "LOWS"], size: 9, page: {} as DirectAccessPageDTO},
                                {label: ["N", "LOWL"], size: 9, page: {} as DirectAccessPageDTO},
                                {label: ["E", "APP"], size: 9, page: {} as DirectAccessPageDTO},
                            ],
                        },
                        {
                            direction: "row",
                            gap: 0.5,
                            children: [
                                {label: ["W", "WI_WK"], size: 9, page: {} as DirectAccessPageDTO},
                                {label: ["S", "LOWG"], size: 9, page: {} as DirectAccessPageDTO},
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
                        {label: ["PAD"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["LJU"], size: 6.25, page: {} as DirectAccessPageDTO},
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
                        {label: ["PRA"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["BRA"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["BUD"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["ZAG"], size: 6.25, page: {} as DirectAccessPageDTO},
                    ],
                },
                {orientation: "vertical", thickness: 2, color: "#364153"},
                {
                    direction: "col",
                    height: "100%",
                    gap: 0.75,
                    justifyContent: "space-between",
                    children: [
                        {label: ["MIL"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["FMP"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["SUP"], size: 6.25, page: {} as DirectAccessPageDTO},
                        {label: ["CWP"], size: 6.25, page: {} as DirectAccessPageDTO},
                    ],
                },
            ],
        },
    ],
};

function GeoPage() {
    return (
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

type GeoPageButtonProps = {
    button: GeoPageButtonDTO;
};

function GeoPageButton({button}: GeoPageButtonProps) {
    return (
        <Button
            color="gray"
            className={"aspect-square w-auto! rounded-none! overflow-hidden"}
            style={{height: `${button.size}rem`}}
            // TODO: OnClick => set active page
        >
            {button.label.map((s, index) => (
                <p key={index}>{s}</p>
            ))}
        </Button>
    );
}

export default GeoPage;
