import {useStationsStore} from "../stores/stations-store.ts";

function MissionPage() {
    const stations = useStationsStore(state => state.stations);

    return (
        <div className="z-10 absolute h-[calc(100%+5rem+5rem+3px-0.5rem)] w-[calc(100%+3px)] translate-y-[calc(-4.75rem-1px)] translate-x-[calc(-1*(1px))] bg-blue-700 border-t-0 px-2 pb-2 flex flex-col overflow-auto rounded">
            <p className="w-full text-white bg-blue-700 font-semibold text-center">Mission</p>
            <div className="flex-1 min-h-0 flex flex-col">
                <div className="w-full flex-1 min-h-0 rounded-b-sm bg-[#B5BBC6] py-3 px-2 flex justify-center items-center">
                    <p className="text-slate-600">Not implemented</p>
                    <p>{stations.size}</p>
                </div>
            </div>
        </div>
    );
}

export default MissionPage;
