import Button from "../components/ui/Button.tsx";
import {
    GeoPageButton as GeoPageButtonModel,
    GeoPageContainer as GeoPageContainerModel,
    GeoPageDivider as GeoPageDividerModel,
    isGeoPageButton,
    isGeoPageContainer,
    isGeoPageDivider,
} from "../types/profile.ts";
import {CSSProperties} from "preact";
import {useProfileStore} from "../stores/profile-store.ts";
import DirectAccessPage from "../components/DirectAccessPage.tsx";
import {useCallStore} from "../stores/call-store.ts";
import {ClientId, StationId} from "../types/generic.ts";
import {Call} from "../types/call.ts";
import {useAuthStore} from "../stores/auth-store.ts";
import {clsx} from "clsx";

type GeoPageProps = {
    page: GeoPageContainerModel;
};

function GeoPage({page}: GeoPageProps) {
    const selectedPage = useProfileStore(state => state.page);

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
    container: GeoPageContainerModel;
    className?: string;
}) {
    const style: CSSProperties = {
        height: container.height,
        width: container.width,
        display: "flex",
        flexDirection: container.direction === "row" ? "row" : "column",
        justifyContent: container.justifyContent,
        alignItems: container.alignItems,
        gap: container.gap && `${container.gap}rem`,
    };

    if (container.padding !== undefined) {
        style.padding = `${container.padding}rem`;
    }
    if (container.paddingLeft !== undefined) {
        style.paddingLeft = `${container.paddingLeft}rem`;
    }
    if (container.paddingRight !== undefined) {
        style.paddingRight = `${container.paddingRight}rem`;
    }
    if (container.paddingTop !== undefined) {
        style.paddingTop = `${container.paddingTop}rem`;
    }
    if (container.paddingBottom !== undefined) {
        style.paddingBottom = `${container.paddingBottom}rem`;
    }

    if (container.alignItems !== undefined) {
        let alignItems: string = container.alignItems;

        if (container.alignItems === "start") {
            alignItems = "flex-start";
        } else if (container.alignItems === "end") {
            alignItems = "flex-end";
        }

        style.alignItems = alignItems;
    }

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
    button: GeoPageButtonModel;
};

function callInvolvesButtonStations(
    call: Call,
    stationIds: StationId[],
    cid: ClientId | undefined,
) {
    return call.source.clientId === cid
        ? call.target.station !== undefined && stationIds.includes(call.target.station)
        : call.source.stationId !== undefined && stationIds.includes(call.source.stationId);
}

function GeoPageButton({button}: GeoPageButtonProps) {
    const blink = useCallStore(state => state.blink);
    const callDisplay = useCallStore(state => state.callDisplay);
    const incomingCalls = useCallStore(state => state.incomingCalls);
    const cid = useAuthStore(state => state.cid);

    const setSelectedPage = useProfileStore(state => state.setPage);

    const stationIds =
        button.page?.keys.flatMap(key => {
            if (key.stationId === undefined) return [];
            return [key.stationId];
        }) ?? [];

    const isCalling = incomingCalls.some(
        call => call.source.stationId !== undefined && stationIds.includes(call.source.stationId),
    );
    const beingCalled =
        callDisplay?.type === "outgoing" &&
        callDisplay.call.target.station !== undefined &&
        stationIds.includes(callDisplay.call.target.station);
    const involved =
        callDisplay !== undefined && callInvolvesButtonStations(callDisplay.call, stationIds, cid);
    const inCall = callDisplay?.type === "accepted" && involved;
    const isRejected = callDisplay?.type === "rejected" && involved;
    const isError = callDisplay?.type === "error" && involved;

    const color = inCall
        ? "green"
        : (isCalling || isRejected) && blink
          ? "green"
          : isError && blink
            ? "red"
            : "gray";

    return (
        <Button
            color={color}
            highlight={beingCalled || isRejected ? "green" : undefined}
            className={clsx(
                "aspect-square w-auto! rounded-none! overflow-hidden",
                color === "gray" ? "p-1.5" : "p-[calc(0.375rem+1px)]",
            )}
            style={{height: `${button.size}rem`}}
            onClick={() => setSelectedPage(button.page)}
        >
            {button.label.map((s, index) => (
                <p key={index} className="max-w-full truncate" title={s}>
                    {s}
                </p>
            ))}
        </Button>
    );
}

type GeoPageDividerProps = {
    divider: GeoPageDividerModel;
};

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
                className="absolute w-full"
                style={{
                    backgroundColor: divider.color,
                    top: divider.oversize && `-${divider.oversize}rem`,
                    height: divider.oversize ? `calc(100% + ${divider.oversize * 2}rem)` : "100%",
                }}
            ></div>
        </div>
    );
}

export default GeoPage;
