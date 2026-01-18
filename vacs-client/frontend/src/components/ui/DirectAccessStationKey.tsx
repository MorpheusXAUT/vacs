import {DirectAccessKey} from "../../types/profile.ts";
import Button from "./Button.tsx";
import {clsx} from "clsx";
import {useStationsStore} from "../../stores/stations-store.ts";
import {useErrorOverlayStore} from "../../stores/error-overlay-store.ts";

type DirectAccessStationKeyProps = {
    data: DirectAccessKey;
    className?: string;
};

function DirectAccessStationKey({
    data: {stationId, label},
    className,
}: DirectAccessStationKeyProps) {
    const stations = useStationsStore(state => state.stations);
    const openErrorOverlay = useErrorOverlayStore(state => state.open);

    const station = stationId !== undefined && stations.get(stationId);
    const online = station !== undefined;
    const own = station !== undefined && station;

    return (
        <Button
            color="gray"
            disabled={stationId === undefined || !online}
            className={clsx(
                className,
                "w-25 h-full rounded p-1.5",
                (own || !online) && "text-gray-500",
            )}
            onClick={() => {
                if (own) {
                    openErrorOverlay("Call error", "You cannot call yourself", false, 5000);
                    return;
                }
                console.log("Calling", stationId);
            }}
        >
            {label.map((s, index) => (
                <p key={index}>{s}</p>
            ))}
        </Button>
    );
}

export default DirectAccessStationKey;
