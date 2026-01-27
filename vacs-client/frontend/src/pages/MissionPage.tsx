import Select from "../components/ui/Select.tsx";
import {useOwnStationIds} from "../stores/stations-store.ts";
import {useState} from "preact/hooks";
import {StationId} from "../types/generic.ts";

function MissionPage() {
    const ownStations = useOwnStationIds();
    const [selectedStation, setSelectedStation] = useState<StationId>(ownStations[0]);

    return (
        <div className="z-10 absolute h-[calc(100%+5rem+5rem+3px-0.5rem)] w-[calc(100%+3px)] translate-y-[calc(-4.75rem-1px)] translate-x-[calc(-1*(1px))] bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto rounded">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Mission</p>
            <div className="flex-1 min-h-0 flex flex-col rounded-b-sm bg-[#B5BBC6] ">
                <div className="w-full flex-1 min-h-0 flex gap-3">
                    <div className="py-2 px-3 flex h-min gap-2 items-center whitespace-nowrap">
                        <p>Default Outbound Station: </p>
                        <Select
                            name="station"
                            selected={selectedStation}
                            onChange={stationId => {
                                // const prev = selectedStation;

                                setSelectedStation(stationId as StationId);
                                // TODO: Send to backend
                            }}
                            options={ownStations.map(s => ({value: s, text: s}))}
                        />
                    </div>
                </div>
                <hr className="text-white" />
                <div className="w-full flex-1 min-h-0 flex justify-center items-center">
                    <p className="text-slate-600">Not implemented</p>
                </div>
            </div>
        </div>
    );
}

export default MissionPage;
