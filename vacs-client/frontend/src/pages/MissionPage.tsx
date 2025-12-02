import {Fragment} from "preact";
import {clsx} from "clsx";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {useShallow} from "zustand/react/shallow";
import {useLayoutEffect, useMemo, useRef, useState} from "preact/hooks";
import {invokeStrict} from "../error.ts";

const HEADER_HEIGHT_REM = 1.75;
const CALL_ROW_HEIGHT_REM = 2.7;

function MissionPage() {
    const profiles = useSignalingStore(useShallow(state => Object.keys(state.stationsConfigProfiles).sort()));
    const profilesCount = profiles.length;
    const selectedProfileName = useSignalingStore(state => state.activeStationsProfileConfig);
    const selectedProfile = useSignalingStore(state => state.getActiveStationsProfileConfig());
    const setSelectedProfile = useSignalingStore(state => state.setActiveStationsProfileConfig);

    const listContainer = useRef<HTMLDivElement>(null);
    const [listContainerHeight, setListContainerHeight] = useState<number>(0);
    const [scrollOffset, setScrollOffset] = useState<number>(0);

    const {visibleProfileIndices, maxScrollOffset} = useMemo((): {
        visibleProfileIndices: number[],
        maxScrollOffset: number
    } => {
        let itemCount: number;

        if (listContainer.current) {
            const fontSize = parseFloat(getComputedStyle(listContainer.current).fontSize);
            const callListHeaderHeight = HEADER_HEIGHT_REM * fontSize;
            const callListItemHeight = CALL_ROW_HEIGHT_REM * fontSize;

            itemCount = Math.floor((listContainerHeight - callListHeaderHeight) / callListItemHeight);
        } else {
            itemCount = 6;
        }

        return {
            visibleProfileIndices: Array.from({length: itemCount}, (_, i) =>
                scrollOffset + i
            ),
            maxScrollOffset: profilesCount - itemCount
        };
    }, [listContainerHeight, profilesCount, scrollOffset]);

    useLayoutEffect(() => {
        if (!listContainer.current) return;
        const observer = new ResizeObserver(entries => {
            for (const entry of entries) {
                setListContainerHeight(entry.contentRect.height);
            }
        });
        observer.observe(listContainer.current);

        return () => {
            observer.disconnect();
        };
    }, []);

    return (
        <div className="w-full h-full relative overflow-visible">
            <div
                className="z-10 absolute h-[calc(100%+5rem+5rem+3px-0.5rem)] w-[calc(100%+3px)] translate-y-[calc(-4.75rem-1px)] translate-x-[calc(-1*(1px))] bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto rounded">
                <p className="w-full text-white bg-blue-700 font-semibold text-center">Mission</p>
                <div className="flex-1 min-h-0 flex flex-col">
                    <div className="w-full flex-1 min-h-0 bg-[#B5BBC6] py-3 px-2 flex flex-row">
                        <div
                            ref={listContainer}
                            className="h-full w-80 grid grid-cols-[1fr_4rem] box-border gap-[1px] [&>div]:outline-1 [&>div]:outline-gray-500"
                            style={{gridTemplateRows: `${HEADER_HEIGHT_REM}rem repeat(${visibleProfileIndices.length},1fr)`}}
                        >
                            {/*HEADER*/}
                            <div className="bg-gray-300 flex justify-center items-center font-bold">Profiles</div>
                            <div className="!outline-0"></div>

                            {visibleProfileIndices.map((profileIndex, idx) => {
                                const rowSpan = visibleProfileIndices.length - 2;
                                const lastElement =
                                    idx === 0 ? (
                                        <ScrollButtonRow direction="up" disabled={scrollOffset <= 0}
                                                         onClick={() => setScrollOffset(scrollOffset => Math.max(scrollOffset - 1, 0))}/>
                                    ) : idx === 1 ? (
                                        <div className="bg-gray-300" style={{gridRow: `span ${rowSpan} / span ${rowSpan}`}}>
                                            <div className="relative h-full w-full px-4 py-7">
                                                <div
                                                    className="h-full w-full border border-b-gray-100 border-r-gray-100 border-l-gray-700 border-t-gray-700 flex flex-col-reverse">
                                                </div>
                                                {/*<div*/}
                                                {/*    className={clsx(*/}
                                                {/*        "dotted-background absolute translate-y-[-50%] left-0 w-full aspect-square shadow-[0_0_0_1px_#364153] rounded-md cursor-pointer bg-blue-600 border",*/}
                                                {/*        true && "border-t-blue-200 border-l-blue-200 border-r-blue-900 border-b-blue-900",*/}
                                                {/*        false && "border-b-blue-200 border-r-blue-200 border-l-blue-900 border-t-blue-900 shadow-none",*/}
                                                {/*    )}*/}
                                                {/*    style={{top: `calc(2.25rem + (1 - ${1}) * (100% - 4.5rem))`}}>*/}
                                                {/*</div>*/}
                                            </div>
                                        </div>
                                    ) : idx === visibleProfileIndices.length - 1 ? (
                                        <ScrollButtonRow direction="down" disabled={scrollOffset >= maxScrollOffset}
                                                         onClick={() => setScrollOffset(scrollOffset => Math.min(scrollOffset + 1, maxScrollOffset))}/>
                                    ) : <></>;

                                const profile = profiles[profileIndex];

                                return (
                                    <Fragment key={idx}>
                                        <ProfileRow
                                            index={idx}
                                            name={profile}
                                            isSelected={profile === selectedProfileName}
                                            onClick={async () => {
                                                if (profile === undefined) return;
                                                try {
                                                    await invokeStrict("signaling_set_selected_stations_config_profile", {profile});
                                                    setSelectedProfile(profile);
                                                } catch {
                                                }
                                            }}
                                        />
                                        {lastElement}
                                    </Fragment>
                                )
                            })}
                        </div>
                        <div className="h-full ml-8 flex-1 flex flex-col">
                            <p className="font-semibold truncate">Selected Profile</p>
                            <div className="flex-1 min-h-0 grid grid-cols-[auto_1fr] grid-rows-[auto_auto_auto_1fr] gap-x-2 [&_p]:truncate">
                                <p>Include:</p><p>[{selectedProfile?.include.join(", ")}]</p>
                                <p>Exclude:</p><p>[{selectedProfile?.exclude.join(", ")}]</p>
                                <p>Priority:</p><p>[{selectedProfile?.priority.join(", ")}]</p>
                                <p>Alias:</p>
                                <div className="grid grid-flow-row grid-cols-2 overflow-y-auto min-h-0">
                                    {Object.entries(selectedProfile?.aliases ?? {}).sort().map(([key, value]) =>
                                        <p className="h-min" key={key}>{`${key} => ${value}`}</p>
                                    )}
                                </div>
                            </div>
                        </div>
                    </div>
                    <hr className="h-[1px] bg-white border-none"/>
                    <div className="w-full flex-1 min-h-0 rounded-b-sm bg-[#B5BBC6] py-3 px-2 flex justify-center items-center">
                        <p className="text-slate-600">Not implemented</p>
                    </div>
                </div>
            </div>
        </div>
    );
}

type CallRowProps = {
    index: number;
    name?: string;
    isSelected: boolean;
    onClick: () => void;
};

function ProfileRow(props: CallRowProps) {
    const color = props.isSelected ? "bg-blue-700 text-white" : "bg-yellow-50";

    return (
        <>
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.name ?? ""}
            </div>
        </>
    );
}

function ScrollButtonRow({direction, disabled, onClick}: {
    direction: "up" | "down";
    disabled: boolean;
    onClick: () => void;
}) {
    return (
        <div className="relative bg-gray-300"
             style={{cursor: disabled ? "not-allowed" : "pointer"}}
             onClick={!disabled ? onClick : undefined}>
            <svg
                className={clsx(
                    "absolute h-[85%] max-w-[85%] top-1/2 -translate-y-1/2 left-1/2 -translate-x-1/2",
                    direction === "down" && "rotate-180"
                )}
                viewBox="0 0 125 89" fill="none" xmlns="http://www.w3.org/2000/svg">
                <path d="M62.5 0L120 60H5L62.5 0Z" fill={disabled ? "#6A7282" : "black"}/>
                <path
                    d="M63.2217 26.3076L120.722 86.3076L122.344 88H2.65625L4.27832 86.3076L61.7783 26.3076L62.5 25.5547L63.2217 26.3076Z"
                    fill={disabled ? "#6A7282" : "black"} stroke="#D1D5DC" strokeWidth="2"/>
            </svg>
        </div>
    );
}

export default MissionPage;