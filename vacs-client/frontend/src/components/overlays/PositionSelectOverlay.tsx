import {connect, useConnectionStore} from "../../stores/connection-store.ts";
import {clsx} from "clsx";
import Button from "../ui/Button.tsx";
import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {PositionId} from "../../types/generic.ts";
import {Fragment} from "preact";

function PositionSelectOverlay() {
    const connecting = useConnectionStore(state => state.connectionState === "connecting");
    const positions = useConnectionStore(state => state.positionsToSelect);
    const visible = connecting && positions.length > 0;

    const setPositions = useConnectionStore(state => state.setPositionsToSelect);
    const setConnectionState = useConnectionStore(state => state.setConnectionState);

    const close = () => {
        setPositions([]);
        setConnectionState("disconnected");
    };

    const handlePositionClick = useAsyncDebounce(async (position: PositionId) => {
        close();
        await connect(position);
    });

    return visible ? (
        <div className="z-50 absolute top-0 left-0 w-full h-full flex justify-center items-center bg-[rgba(0,0,0,0.5)]">
            <div className="bg-gray-300 border-4 border-t-red-500 border-l-red-500 border-b-red-700 border-r-red-700 rounded w-100 py-2">
                <p className="w-full text-center text-lg font-semibold wrap-break-word">
                    Ambiguous position
                </p>
                <p className="w-full text-center wrap-break-word mb-2">
                    Multiple VATSIM positions matched your current position. Please select the
                    correct position manually.
                </p>
                <div className="px-3 flex flex-col gap-2">
                    <div className="max-h-46 overflow-y-auto flex justify-center">
                        <div className="w-min py-2 px-1 grid gap-2">
                            {positions.map(position =>
                                PositionButton(position, handlePositionClick),
                            )}
                        </div>
                    </div>
                    <Button color="red" className="w-min h-full px-3 py-1" onClick={close}>
                        Cancel
                    </Button>
                </div>
            </div>
        </div>
    ) : (
        <></>
    );
}

function PositionButton(position: PositionId, onClick: (position: PositionId) => Promise<void>) {
    const parts = position.split("_");

    return (
        <Button
            color="gray"
            className="shrink-0 w-full h-10 px-5 font-normal!"
            onClick={() => onClick(position)}
        >
            {parts.map((part, index) => (
                <Fragment key={index}>
                    <span className={clsx(parts.length - 1 !== index && "font-bold")}>{part}</span>
                    {parts.length - 1 !== index && (
                        <span className={clsx(parts.length - 2 !== index && "font-bold")}>_</span>
                    )}
                </Fragment>
            ))}
        </Button>
    );
}

export default PositionSelectOverlay;
