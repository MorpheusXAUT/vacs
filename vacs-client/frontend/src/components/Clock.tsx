import {useEffect, useState} from "preact/hooks";
import StatusIndicator from "./ui/StatusIndicator.tsx";

type TimeState = {
    hours: string;
    minutes: string;
    day: string;
};

function Clock() {
    const [time, setTime] = useState<TimeState>({
        hours: "99",
        minutes: "99",
        day: "99",
    });

    useEffect(() => {
        const updateClock = () => {
            const now = new Date();
            const hours = now.getUTCHours().toString().padStart(2, "0");
            const minutes = now.getUTCMinutes().toString().padStart(2, "0");
            const day = now.getUTCDate().toString().padStart(2, "0");

            setTime(prev => {
                if (prev.hours === hours && prev.minutes === minutes && prev.day === day) {
                    return prev;
                }
                return {hours, minutes, day};
            });
        };

        updateClock();
        const interval = setInterval(updateClock, 1000);

        return () => clearInterval(interval);
    }, []);

    return (
        <div className="h-full px-1 border-r bg-[#c3c8ce] w-min whitespace-nowrap">
            <div className="h-1/2 flex items-center">
                <p className="font-bold leading-3 tracking-wider text-xl">
                    {time.hours}:{time.minutes}
                </p>
            </div>
            <div className="h-1/2 flex items-center justify-between">
                <div className="h-full py-1.5 pl-0.5">
                    <StatusIndicator />
                </div>
                <p className="font-bold leading-3 tracking-wider text-xl text-gray-500">
                    {time.day}
                </p>
            </div>
        </div>
    );
}

export default Clock;
