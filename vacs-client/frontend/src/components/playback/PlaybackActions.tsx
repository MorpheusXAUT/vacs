import {useAsyncDebounce} from "../../hooks/debounce-hook.ts";
import {ClipMeta} from "../../types/playback.ts";
import {invokeSafe} from "../../error.ts";
import Button from "../ui/Button.tsx";
import {StateSetter} from "../../types/generic.ts";

type PlaybackActionsProps = {
    clips: ClipMeta[];
    selectedClip: ClipMeta | undefined;
    setClips: StateSetter<ClipMeta[]>;
    deleteDisabled: boolean;
};

function PlaybackActions({clips, selectedClip, setClips, deleteDisabled}: PlaybackActionsProps) {
    const handleDelete = useAsyncDebounce(async () => {
        if (selectedClip === undefined) return;
        await invokeSafe("playback_delete", {id: selectedClip.id});
        setClips(prev => prev.filter(c => c.id !== selectedClip.id));
    });

    const handleExport = useAsyncDebounce(async () => {
        if (selectedClip === undefined) return;
        await invokeSafe("playback_export", {id: selectedClip.id});
    });

    const handleClear = useAsyncDebounce(async () => {
        await invokeSafe("playback_clear");
        setClips([]);
    });

    return (
        <div className="flex flex-col gap-3">
            <Button
                color="gray"
                className="h-17 uppercase"
                disabled={selectedClip === undefined}
                onClick={handleExport}
            >
                Export
            </Button>
            <Button
                color="gray"
                className="h-17 uppercase"
                disabled={selectedClip === undefined || deleteDisabled}
                onClick={handleDelete}
            >
                Delete
            </Button>
            <Button
                color="gray"
                className="h-17 uppercase"
                disabled={clips.length === 0 || deleteDisabled}
                onClick={handleClear}
            >
                <p>
                    Delete <br /> All
                </p>
            </Button>
        </div>
    );
}

export default PlaybackActions;
