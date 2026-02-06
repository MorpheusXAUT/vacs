import {DirectAccessPage as DirectAccessPageModel} from "../types/profile.ts";
import {CSSProperties} from "preact";
import DirectAccessStationKey from "./ui/DirectAccessStationKey.tsx";
import {clsx} from "clsx";
import ButtonLabel from "./ui/ButtonLabel.tsx";
import Button from "./ui/Button.tsx";
import {useProfileStore} from "../stores/profile-store.ts";
import {useCallState} from "../hooks/call-state-hook.ts";

type DirectAccessPageProps = {
    data: DirectAccessPageModel;
};

function DirectAccessPage({data}: DirectAccessPageProps) {
    const style: CSSProperties = {
        gridTemplateRows: `repeat(${data.rows}, 1fr)`,
        gridAutoFlow: "column",
    };

    return (
        <div className="w-full h-full overflow-auto">
            <div className="w-min min-h-full py-3 px-2 grid gap-2" style={style}>
                {data.keys.map((key, index) =>
                    key.page !== undefined ? (
                        <DirectAccessSubpageKey
                            key={index}
                            label={key.label}
                            page={key.page}
                            parent={data}
                        />
                    ) : (
                        <DirectAccessStationKey
                            key={index}
                            data={key}
                            className={
                                data.rows !== undefined && data.rows > 6
                                    ? "leading-4!"
                                    : "leading-4.5!"
                            }
                        />
                    ),
                )}
            </div>
        </div>
    );
}

type DirectAccessSubpageKeyProps = {
    label: string[];
    page: DirectAccessPageModel;
    parent: DirectAccessPageModel;
    className?: string;
};

function DirectAccessSubpageKey(props: DirectAccessSubpageKeyProps) {
    const {beingCalled, isRejected, color} = useCallState(props.page);
    const setSelectedPage = useProfileStore(state => state.setPage);

    return (
        <Button
            color={color}
            highlight={beingCalled || isRejected ? "green" : undefined}
            className={clsx(
                props.className,
                "w-25 h-full rounded",
                color === "gray" ? "p-1.5" : "p-[calc(0.375rem+1px)]",
            )}
            onClick={() => setSelectedPage({current: props.page, parent: props.parent})}
        >
            <ButtonLabel label={props.label} />
        </Button>
    );
}

export default DirectAccessPage;
