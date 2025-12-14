import {clsx} from "clsx";
import {useSignalingStore} from "../stores/signaling-store.ts";
import {useShallow} from "zustand/react/shallow";
import List from "../components/ui/List.tsx";
import {invokeSafe, invokeStrict} from "../error.ts";
import {StationsGroupMode} from "../types/stations.ts";
import Button from "../components/ui/Button.tsx";
import {useAsyncDebounce} from "../hooks/debounce-hook.ts";

function MissionPage() {
    const profiles = useSignalingStore(
        useShallow(state => Object.keys(state.stationsConfigProfiles).sort()),
    );
    const selectedProfileName = useSignalingStore(state => state.activeStationsProfileConfig);
    const selectedProfile = useSignalingStore(state => state.getActiveStationsProfileConfig());
    const setSelectedProfile = useSignalingStore(state => state.setActiveStationsProfileConfig);

    const selectedProfileIndex = profiles.indexOf(selectedProfileName);

    const handleSelectStationsConfigClick = useAsyncDebounce(async () => {
        await invokeSafe("app_pick_extra_stations_config");
    });

    return (
        <div className="z-10 absolute h-[calc(100%+5rem+5rem+3px-0.5rem)] w-[calc(100%+3px)] translate-y-[calc(-4.75rem-1px)] translate-x-[calc(-1*(1px))] bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto rounded">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Mission</p>
            <div className="flex-1 min-h-0 flex flex-col">
                <div className="w-full flex-1 min-h-0 bg-[#B5BBC6] py-3 px-2 flex flex-row">
                    <div className="h-full flex flex-col gap-2">
                        <List
                            className="w-80"
                            itemsCount={profiles.length}
                            selectedItem={selectedProfileIndex}
                            setSelectedItem={async index => {
                                const profile = profiles[index];
                                if (profile === undefined) return;
                                try {
                                    await invokeStrict(
                                        "signaling_set_selected_stations_config_profile",
                                        {profile},
                                    );
                                    setSelectedProfile(profile);
                                } catch {}
                            }}
                            defaultRows={6}
                            row={(index, isSelected, onClick) =>
                                ProfileRow(profiles[index], isSelected, onClick)
                            }
                            header={[{title: "Profiles"}]}
                            columnWidths={["1fr"]}
                        />
                        <Button
                            color="gray"
                            className="w-64 whitespace-nowrap px-3 py-2"
                            onClick={handleSelectStationsConfigClick}
                        >
                            Select stations config
                        </Button>
                    </div>
                    <div className="h-full ml-8 flex-1 flex flex-col">
                        <p className="font-semibold truncate">
                            Selected Profile - {selectedProfileName ?? "Default"}
                        </p>
                        <div className="flex-1 min-h-0 grid grid-cols-[auto_1fr] grid-rows-[auto_auto_auto_auto_auto_1fr] gap-x-2 [&_p]:truncate">
                            <p>Include:</p>
                            <p>[{selectedProfile?.include.join(", ")}]</p>
                            <p>Exclude:</p>
                            <p>[{selectedProfile?.exclude.join(", ")}]</p>
                            <p>Priority:</p>
                            <p>[{selectedProfile?.priority.join(", ")}]</p>
                            <p>Frequencies:</p>
                            <p>
                                {selectedProfile?.frequencies === "HideAll"
                                    ? "Hide all"
                                    : selectedProfile?.frequencies === "HideAliased"
                                      ? "Hide aliased"
                                      : "Show all"}
                            </p>
                            <p>Grouping:</p>
                            <p>{GroupingLabels[selectedProfile?.grouping ?? "None"]}</p>
                            <p>Alias:</p>
                            <div className="overflow-y-auto">
                                <div className="grid grid-flow-row grid-cols-2">
                                    {Object.entries(selectedProfile?.aliases ?? {})
                                        .sort()
                                        .map(([key, value]) => (
                                            <p
                                                className="h-min"
                                                key={key}
                                            >{`${key} => ${value}`}</p>
                                        ))}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                <hr className="h-[1px] bg-white border-none" />
                <div className="w-full flex-1 min-h-0 rounded-b-sm bg-[#B5BBC6] py-3 px-2 flex justify-center items-center">
                    <p className="text-slate-600">Not implemented</p>
                </div>
            </div>
        </div>
    );
}

const GroupingLabels: {[key in StationsGroupMode]: string} = {
    None: "None",
    Fir: "FIR",
    Icao: "ICAO",
    FirAndIcao: "FIR and ICAO",
};

function ProfileRow(name: string | undefined, isSelected: boolean, onClick: () => void) {
    const color = isSelected ? "bg-blue-700 text-white" : "bg-yellow-50";

    return (
        <div className={clsx("px-0.5 flex items-center font-semibold", color)} onClick={onClick}>
            {name ?? ""}
        </div>
    );
}

export default MissionPage;
