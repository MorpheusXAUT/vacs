import {useState} from "preact/hooks";
import List from "../ui/List.tsx";
import {PlaybackListEntry} from "../../types/playback.ts";
import {clsx} from "clsx";

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

export default PlaybackList;
