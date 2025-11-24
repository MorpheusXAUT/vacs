import Button from "./ui/Button.tsx";
import "../styles/call-list.css";
import {CallListItem, useCallListStore} from "../stores/call-list-store.ts";
import {clsx} from "clsx";
import {HEADER_HEIGHT_REM, useCallList} from "../hooks/call-list-hook.ts";
import {startCall, useCallStore} from "../stores/call-store.ts";
import {Fragment} from "preact";

function CallList() {
    const calls = useCallListStore(state => state.callList);
    const {clearCallList} = useCallListStore(state => state.actions);
    const callDisplay = useCallStore(state => state.callDisplay);

    const {
        listContainer,
        scrollOffset,
        setScrollOffset,
        selectedCall,
        setSelectedCall,
        visibleCallIndices,
        maxScrollOffset
    } = useCallList({callsCount: calls.length});

    return (
        <div className="w-[37.5rem] h-full flex flex-col gap-3 p-3">
            <div
                ref={listContainer}
                className="h-full w-full grid grid-cols-[minmax(3.5rem,auto)_1fr_1fr_4rem] box-border gap-[1px] [&>div]:outline-1 [&>div]:outline-gray-500"
                style={{gridTemplateRows: `${HEADER_HEIGHT_REM}rem repeat(${visibleCallIndices.length},1fr)`}}
            >
                {/*HEADER*/}
                <div className="col-span-2 bg-gray-300 flex justify-center items-center font-bold">Name</div>
                <div className="bg-gray-300 flex justify-center items-center font-bold">Number</div>
                <div className="!outline-0"></div>

                {visibleCallIndices.map((callIndex, idx) => {
                    const rowSpan = visibleCallIndices.length - 2;
                    const lastElement =
                        idx === 0 ? (
                            <ScrollButtonRow direction="up" disabled={scrollOffset <= 0}
                                             onClick={() => setScrollOffset(scrollOffset => Math.max(scrollOffset - 1, 0))}/>
                        ) : idx === 1 ? (
                            <div className="bg-gray-300" style={{gridRow: `span ${rowSpan} / span ${rowSpan}`}}>
                                <div className="relative h-full w-full px-4 py-13">
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
                        ) : idx === visibleCallIndices.length - 1 ? (
                            <ScrollButtonRow direction="down" disabled={scrollOffset >= maxScrollOffset}
                                             onClick={() => setScrollOffset(scrollOffset => Math.min(scrollOffset + 1, maxScrollOffset))}/>
                        ) : <></>;

                    return (
                        <Fragment key={idx}>
                            <CallRow
                                index={idx}
                                call={calls[callIndex]}
                                isSelected={callIndex === selectedCall}
                                onClick={() => {
                                    setSelectedCall(callIndex);
                                }}
                            />
                            {lastElement}
                        </Fragment>
                    )
                })}
            </div>
            <div className="w-full shrink-0 flex flex-row justify-between pr-16 [&_button]:h-15 [&_button]:rounded">
                <Button color="gray" onClick={clearCallList}>
                    <p>Delete<br/>List</p>
                </Button>
                <Button color="gray" className="w-56 text-xl"
                        disabled={calls[selectedCall]?.number === undefined}
                        onClick={async () => {
                            const peerId: string | undefined = calls[selectedCall]?.number;
                            if (peerId === undefined || callDisplay !== undefined) return;
                            await startCall(peerId);
                        }}
                >
                    Call
                </Button>
            </div>
        </div>
    );
}

type CallRowProps = {
    index: number;
    call?: CallListItem;
    isSelected: boolean;
    onClick: () => void;
};

function CallRow(props: CallRowProps) {
    const color = props.isSelected ? "bg-blue-700 text-white" : "bg-yellow-50";

    return (
        <>
            <div
                className={clsx("p-0.5 text-center flex flex-col justify-between leading-4", color)}
                onClick={props.onClick}
            >
                <p>{props.call?.type ?? ""}</p>
                <p className="tracking-wider font-semibold">{props.call?.time ?? ""}</p>
            </div>
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.call?.name ?? ""}
            </div>
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.call?.number ?? ""}
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

export default CallList;