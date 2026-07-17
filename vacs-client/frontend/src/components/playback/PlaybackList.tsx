import {clsx} from "clsx";
import {ClipMeta, clipUnixMs} from "../../types/playback.ts";
import {toUTCTimeString} from "../../utils/date.ts";
import List from "../ui/List.tsx";

type PlaybackListProps = {
    clips: ClipMeta[];
    selected: number;
    setSelected: (index: number) => void;
};

function PlaybackList(props: PlaybackListProps) {
    const playbackListEntry = (item: number, isSelected: boolean, onClick: () => void) => (
        <PlaybackEntryRow clip={props.clips[item]} isSelected={isSelected} onClick={onClick} />
    );

    return (
        <List
            className="w-full h-72! shrink-0"
            itemsCount={props.clips.length}
            selectedItem={props.selected}
            setSelectedItem={props.setSelected}
            defaultRows={7}
            row={playbackListEntry}
            rowHeight={2.2}
            columnWidths={["4.5ch", "5.5rem", "6ch", "auto"]}
            enableKeyboardNavigation={true}
        />
    );
}

type PlaybackEntryRowProps = {
    clip: ClipMeta | undefined;
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
                {props.clip && "Rx"}
            </div>
            <div
                className={clsx("flex items-center justify-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.clip && toUTCTimeString(new Date(clipUnixMs(props.clip.startedAt)))}
            </div>
            <div
                className={clsx("px-0.5 flex items-center font-semibold", color)}
                onClick={props.onClick}
            >
                {props.clip && `${(props.clip.durationMs / 1000).toFixed(1)}s`}
            </div>
            <div
                className={clsx("px-0.5 flex items-center font-semibold min-w-0", color)}
                onClick={props.onClick}
            >
                <p className="w-full truncate">{props.clip && clipToTarget(props.clip)}</p>
            </div>
        </>
    );
}

function clipToTarget(clip: ClipMeta): string {
    let callsignsString = clip?.callsigns.join("|");
    if (callsignsString.length > 0) {
        callsignsString += "\\";
    }

    if (clip.frequency === null || clip.frequency === 0) {
        return `${callsignsString}<UNKNOWN>`;
    }

    const freq = (clip.frequency / 1000).toString();
    const freqString = `${freq.substring(0, 3)}.${freq.substring(3)}`;

    return `${callsignsString}${freqString}`;
}

export default PlaybackList;
