import {clsx} from "clsx";
import List from "../components/ui/List.tsx";
import {useState} from "preact/hooks";
import {CloseButton} from "./SettingsPage.tsx";
import Button from "../components/ui/Button.tsx";

type PlaybackListEntry = {
    type: "Rx" | "Tx" | "Ph";
    idk: boolean;
    time: string;
    target: string;
};

function PlaybackPage() {
    return (
        <div
            className={clsx(
                "z-10 absolute h-[calc(100%+3px)] w-[44rem] -top-px right-[-2px]",
                "bg-blue-700 px-2 pb-2 flex flex-col rounded-md",
            )}
        >
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Playback</p>
            <div className="w-full grow rounded-b-sm bg-[#B5BBC6] grid grid-cols-[6.5rem_auto] p-2 gap-2 overflow-auto">
                <div className="h-full w-full flex flex-col justify-between items-center">
                    <div className="w-full flex flex-col items-center bg-gray-300 border rounded-md">
                        <p className="w-full border-b text-center font-semibold">Filter</p>
                        <Button color="gray" className="h-15 my-2 uppercase">
                            Speech <br /> Only
                        </Button>
                        <Button color="blue" className="h-15 mt-2 uppercase rounded-b-none!">
                            Radio
                        </Button>
                        <Button color="blue" className="h-15 mb-2 uppercase rounded-t-none!">
                            Phone
                        </Button>
                    </div>
                    <div className="flex flex-col gap-3">
                        <Button color="gray" className="h-17 uppercase">
                            Export
                        </Button>
                        <Button color="gray" className="h-17 uppercase">
                            Delete
                        </Button>
                        <Button color="gray" className="h-17 uppercase">
                            Delete <br /> All
                        </Button>
                    </div>
                </div>
                <div className="h-full w-full flex flex-col p-px">
                    <PlaybackList />
                    <div className="relative w-full h-full flex flex-col items-center pr-16">
                        <p className="py-1 font-semibold">No playback</p>
                        <div className="shrink-0 w-full h-4 border bg-gray-300"></div>
                        <div className="flex-1 min-h-0 w-full flex items-end justify-center">
                            <div className="h-min w-min grid grid-flow-col grid-rows-2 gap-y-3 gap-x-2">
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center"
                                >
                                    <svg
                                        width="32"
                                        height="32"
                                        viewBox="0 0 74 74"
                                        fill="none"
                                        xmlns="http://www.w3.org/2000/svg"
                                    >
                                        <path d="M0 37V0L74 37L0 74V37Z" fill="currentColor" />
                                    </svg>
                                </Button>
                                <Button color="gray" className="h-17">
                                    H/S
                                </Button>
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center text-gray-600"
                                    disabled={true}
                                >
                                    <svg
                                        height="40"
                                        viewBox="0 0 96 110"
                                        fill="none"
                                        xmlns="http://www.w3.org/2000/svg"
                                    >
                                        <path d="M0 37V0L74 37L0 74V37Z" fill="currentColor" />
                                        <path
                                            d="M95.8945 68.2109L99.4717 70L95.8945 71.7891L19 110.236V29.7637L95.8945 68.2109Z"
                                            fill="currentColor"
                                            className="stroke-gray-300"
                                            stroke-width="4"
                                        />
                                    </svg>
                                </Button>
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center"
                                    disabled={true}
                                >
                                    <div className="h-8 aspect-square bg-gray-600"></div>
                                </Button>
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center text-gray-600"
                                    disabled={true}
                                >
                                    <svg
                                        height="32"
                                        viewBox="0 0 48 74"
                                        fill="none"
                                        xmlns="http://www.w3.org/2000/svg"
                                    >
                                        <path
                                            d="M48 0V74L11 37V74H0V0H11V37L48 0Z"
                                            fill="currentColor"
                                        />
                                    </svg>
                                </Button>
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center text-gray-600"
                                    disabled={true}
                                >
                                    <svg
                                        width="32"
                                        height="32"
                                        viewBox="0 0 74 74"
                                        fill="none"
                                        xmlns="http://www.w3.org/2000/svg"
                                    >
                                        <path
                                            d="M74 0V74L37 37V74L0 37L37 0V37L74 0Z"
                                            fill="currentColor"
                                        />
                                    </svg>
                                </Button>
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center text-gray-600"
                                    disabled={true}
                                >
                                    <svg
                                        transform="rotate(180)"
                                        height="32"
                                        viewBox="0 0 48 74"
                                        fill="none"
                                        xmlns="http://www.w3.org/2000/svg"
                                    >
                                        <path
                                            d="M48 0V74L11 37V74H0V0H11V37L48 0Z"
                                            fill="currentColor"
                                        />
                                    </svg>
                                </Button>
                                <Button
                                    color="gray"
                                    className="h-17 flex items-center justify-center text-gray-600"
                                    disabled={true}
                                >
                                    <svg
                                        transform="rotate(180)"
                                        width="32"
                                        height="32"
                                        viewBox="0 0 74 74"
                                        fill="none"
                                        xmlns="http://www.w3.org/2000/svg"
                                    >
                                        <path
                                            d="M74 0V74L37 37V74L0 37L37 0V37L74 0Z"
                                            fill="currentColor"
                                        />
                                    </svg>
                                </Button>
                            </div>
                        </div>
                        <CloseButton className="h-18 w-20! absolute bottom-0 right-0" />
                    </div>
                </div>
            </div>
        </div>
    );
}

function PlaybackList() {
    const playbackList: PlaybackListEntry[] = [
        {type: "Ph", idk: true, time: "12:34:56", target: "310 N1 PLC"},
        {type: "Rx", idk: true, time: "12:34:55", target: "FIN\\134.675"},
        {type: "Tx", idk: true, time: "12:34:54", target: "FIN\\134.675"},
        {type: "Rx", idk: true, time: "12:34:53", target: "FIN\\134.675"},
        {type: "Tx", idk: true, time: "12:34:52", target: "FIN\\134.675"},
        {type: "Rx", idk: true, time: "12:34:51", target: "FIN\\134.675"},
        {type: "Tx", idk: true, time: "12:34:50", target: "FIN\\134.675"},
    ];
    const [selected, setSelected] = useState<number>(0);

    const playbackListEntry = (item: number, isSelected: boolean, onClick: () => void) => (
        <PlaybackEntryRow entry={playbackList[item]} isSelected={isSelected} onClick={onClick} />
    );

    return (
        <List
            className="w-full h-72! shrink-0"
            itemsCount={playbackList.length}
            selectedItem={selected}
            setSelectedItem={setSelected}
            defaultRows={7}
            row={playbackListEntry}
            rowHeight={2.2}
            columnWidths={["5ch", "5ch", "5.5rem", "auto"]}
            enableKeyboardNavigation={true}
        />
    );
}

type PlaybackEntryRowProps = {
    entry: PlaybackListEntry | undefined;
    isSelected: boolean;
    onClick: () => void;
};

function PlaybackEntryRow(props: PlaybackEntryRowProps) {
    const color = props.isSelected ? "bg-blue-700 text-white" : "bg-yellow-50";

    return (
        <>
            <div
                className={clsx("px-0.5 text-center flex justify-between items-center", color)}
                onClick={props.onClick}
            >
                {props.entry?.type ?? ""}
            </div>
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.entry?.idk === true ? "Y" : ""}
            </div>
            <div
                className={clsx("flex items-center justify-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.entry?.time ?? ""}
            </div>
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.entry?.target ?? ""}
            </div>
        </>
    );
}

export default PlaybackPage;
